//! A5 离线与同步机制 - 同步服务骨架
//!
//! 设计文档：功能展望/平台级增强/04_离线与同步机制.md
//!
//! 当前实现（Phase 1 基础架构）：
//! - sync_queue 队列管理：enqueue / fetch_pending / mark_synced / mark_failed
//! - LWW 冲突解决：基于 updated_at 时间戳的 Last-Write-Wins
//! - 传输后端 trait：TransportBackend（WebDAV/S3 适配器在 Phase 2 实现）
//! - 设备管理：register_device / list_devices / get_current_device
//!
//! 未实现（留待 Phase 2-4）：
//! - CRDT 合并（Yjs/Automerge）
//! - 实际传输后端（WebDAV/S3 HTTP 调用）
//! - 退避重试调度器
//! - E2E 加密
//! - 手动冲突解决 UI

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::app_error::AppError;

// ============================================================================
// 数据模型
// ============================================================================

/// 同步操作类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum SyncOperation {
    Insert,
    Update,
    Delete,
}

impl SyncOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Insert => "INSERT",
            Self::Update => "UPDATE",
            Self::Delete => "DELETE",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "INSERT" => Some(Self::Insert),
            "UPDATE" => Some(Self::Update),
            "DELETE" => Some(Self::Delete),
            _ => None,
        }
    }

    /// 优先级排序值（DELETE 优先级最高，节省空间）
    pub fn priority(self) -> u8 {
        match self {
            Self::Delete => 1,
            Self::Update => 2,
            Self::Insert => 3,
        }
    }
}

/// 同步状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Pending,
    Syncing,
    Synced,
    Failed,
    Conflict,
}

impl SyncStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Syncing => "syncing",
            Self::Synced => "synced",
            Self::Failed => "failed",
            Self::Conflict => "conflict",
        }
    }
}

/// sync_queue 记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SyncQueueItem {
    pub id: i64,
    pub table_name: String,
    pub record_id: i64,
    pub operation: String,
    pub payload: String,
    pub vector_clock: Option<String>,
    pub device_id: Option<String>,
    pub created_at: String,
    pub synced_at: Option<String>,
    pub sync_status: String,
    pub retry_count: i64,
    pub last_error: Option<String>,
}

/// sync_devices 记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SyncDevice {
    pub id: String,
    pub device_name: String,
    pub device_type: String,
    pub public_key: String,
    pub registered_at: String,
    pub last_seen_at: Option<String>,
    pub last_sync_at: Option<String>,
    pub is_current_device: i64,
}

/// LWW 冲突解决结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LwwResolution {
    /// "local" 或 "remote"
    pub winner: String,
    /// 胜出的数据（JSON 字符串）
    pub data: String,
    /// 冲突原因说明
    pub reason: String,
}

// ============================================================================
// sync_queue 队列管理
// ============================================================================

/// 将变更记录入队（应用层显式调用）
pub async fn enqueue(
    pool: &SqlitePool,
    table_name: &str,
    record_id: i64,
    operation: SyncOperation,
    payload: &str,
    device_id: Option<&str>,
) -> Result<i64, AppError> {
    let result = sqlx::query(
        "INSERT INTO sync_queue (table_name, record_id, operation, payload, device_id)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(table_name)
    .bind(record_id)
    .bind(operation.as_str())
    .bind(payload)
    .bind(device_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(result.last_insert_rowid())
}

/// 获取待同步队列（按优先级排序：DELETE > UPDATE > INSERT）
pub async fn fetch_pending(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<SyncQueueItem>, AppError> {
    let items = sqlx::query_as::<_, SyncQueueItem>(
        "SELECT * FROM sync_queue
         WHERE sync_status = 'pending'
         ORDER BY
            CASE operation
                WHEN 'DELETE' THEN 1
                WHEN 'UPDATE' THEN 2
                WHEN 'INSERT' THEN 3
            END,
            created_at ASC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(items)
}

/// 标记为同步中
pub async fn mark_syncing(pool: &SqlitePool, id: i64) -> Result<(), AppError> {
    sqlx::query("UPDATE sync_queue SET sync_status = 'syncing' WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

/// 标记为已同步
pub async fn mark_synced(pool: &SqlitePool, id: i64) -> Result<(), AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE sync_queue SET sync_status = 'synced', synced_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

/// 标记为同步失败（增加 retry_count + 记录错误）
pub async fn mark_failed(
    pool: &SqlitePool,
    id: i64,
    error_msg: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE sync_queue
         SET sync_status = 'failed', retry_count = retry_count + 1, last_error = ?
         WHERE id = ?",
    )
    .bind(error_msg)
    .bind(id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

/// 标记为冲突
pub async fn mark_conflict(
    pool: &SqlitePool,
    id: i64,
    conflict_reason: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE sync_queue
         SET sync_status = 'conflict', last_error = ?
         WHERE id = ?",
    )
    .bind(conflict_reason)
    .bind(id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

/// 获取队列统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncQueueStats {
    pub pending: i64,
    pub syncing: i64,
    pub synced: i64,
    pub failed: i64,
    pub conflict: i64,
}

pub async fn get_queue_stats(pool: &SqlitePool) -> Result<SyncQueueStats, AppError> {
    let pending: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sync_queue WHERE sync_status = 'pending'",
    )
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    let syncing: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sync_queue WHERE sync_status = 'syncing'",
    )
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    let synced: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sync_queue WHERE sync_status = 'synced'",
    )
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    let failed: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sync_queue WHERE sync_status = 'failed'",
    )
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    let conflict: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sync_queue WHERE sync_status = 'conflict'",
    )
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(SyncQueueStats {
        pending: pending.0,
        syncing: syncing.0,
        synced: synced.0,
        failed: failed.0,
        conflict: conflict.0,
    })
}

// ============================================================================
// LWW 冲突解决
// ============================================================================

/// LWW（Last-Write-Wins）冲突解决
///
/// 比较本地和远端记录的 updated_at 时间戳，选择较新的。
///
/// 参数：
/// - local_updated_at: 本地记录的更新时间戳（Unix 毫秒）
/// - local_data: 本地记录的 JSON 数据
/// - remote_updated_at: 远端记录的更新时间戳（Unix 毫秒）
/// - remote_data: 远端记录的 JSON 数据
pub fn resolve_lww(
    local_updated_at: i64,
    local_data: &str,
    remote_updated_at: i64,
    remote_data: &str,
) -> LwwResolution {
    if remote_updated_at > local_updated_at {
        LwwResolution {
            winner: "remote".into(),
            data: remote_data.to_string(),
            reason: format!(
                "远端 updated_at({}) > 本地 updated_at({})，采用远端",
                remote_updated_at, local_updated_at
            ),
        }
    } else if local_updated_at > remote_updated_at {
        LwwResolution {
            winner: "local".into(),
            data: local_data.to_string(),
            reason: format!(
                "本地 updated_at({}) > 远端 updated_at({})，采用本地",
                local_updated_at, remote_updated_at
            ),
        }
    } else {
        // 时间戳相等：默认采用本地（避免不必要的数据替换）
        LwwResolution {
            winner: "local".into(),
            data: local_data.to_string(),
            reason: format!(
                "时间戳相等({})，默认采用本地",
                local_updated_at
            ),
        }
    }
}

// ============================================================================
// 设备管理
// ============================================================================

/// 注册新设备
pub async fn register_device(
    pool: &SqlitePool,
    id: &str,
    device_name: &str,
    device_type: &str,
    public_key: &str,
    is_current: bool,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO sync_devices (id, device_name, device_type, public_key, is_current_device)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(device_name)
    .bind(device_type)
    .bind(public_key)
    .bind(if is_current { 1 } else { 0 })
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    tracing::info!("📱 [A5] 设备已注册: {} ({})", device_name, device_type);
    Ok(())
}

/// 列出所有已注册设备
pub async fn list_devices(pool: &SqlitePool) -> Result<Vec<SyncDevice>, AppError> {
    let devices = sqlx::query_as::<_, SyncDevice>(
        "SELECT * FROM sync_devices ORDER BY registered_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(devices)
}

/// 获取当前设备
pub async fn get_current_device(pool: &SqlitePool) -> Result<Option<SyncDevice>, AppError> {
    let device = sqlx::query_as::<_, SyncDevice>(
        "SELECT * FROM sync_devices WHERE is_current_device = 1 LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(device)
}

/// 更新设备最后同步时间
pub async fn update_last_sync(pool: &SqlitePool, device_id: &str) -> Result<(), AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE sync_devices SET last_sync_at = ?, last_seen_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(&now)
    .bind(device_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

/// 注销设备（删除）
pub async fn unregister_device(pool: &SqlitePool, device_id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM sync_devices WHERE id = ?")
        .bind(device_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lww_remote_newer() {
        let result = resolve_lww(1000, r#"{"v":"local"}"#, 2000, r#"{"v":"remote"}"#);
        assert_eq!(result.winner, "remote");
        assert!(result.data.contains("remote"));
    }

    #[test]
    fn test_lww_local_newer() {
        let result = resolve_lww(3000, r#"{"v":"local"}"#, 2000, r#"{"v":"remote"}"#);
        assert_eq!(result.winner, "local");
        assert!(result.data.contains("local"));
    }

    #[test]
    fn test_lww_equal_timestamp_defaults_local() {
        let result = resolve_lww(1000, r#"{"v":"local"}"#, 1000, r#"{"v":"remote"}"#);
        assert_eq!(result.winner, "local");
    }

    #[test]
    fn test_sync_operation_priority() {
        assert!(SyncOperation::Delete.priority() < SyncOperation::Update.priority());
        assert!(SyncOperation::Update.priority() < SyncOperation::Insert.priority());
    }

    #[test]
    fn test_sync_operation_from_str() {
        assert_eq!(SyncOperation::from_str("INSERT"), Some(SyncOperation::Insert));
        assert_eq!(SyncOperation::from_str("UPDATE"), Some(SyncOperation::Update));
        assert_eq!(SyncOperation::from_str("DELETE"), Some(SyncOperation::Delete));
        assert_eq!(SyncOperation::from_str("invalid"), None);
    }
}
