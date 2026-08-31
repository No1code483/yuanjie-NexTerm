//! A5 离线与同步机制 - Phase 3 Task 5 配置同步降级
//!
//! 设计文档：功能展望/平台级增强/04_离线与同步机制.md §Phase 3
//!
//! ## 职责
//!
//! - 记录配置变更到 sync_queue（复用 `sync_service::enqueue`）
//! - 网络恢复时 flush：将 `failed` 的配置条目重置为 `pending`，等待 scheduler 同步
//! - 查询待发配置条数（`pending` + `failed`）
//!
//! ## 设计说明
//!
//! - 配置条目通过 `table_name = 'app_config'` 标识，与普通数据变更区分
//! - `record_id` 固定为 0（配置项以 key 标识，无自增 ID；payload 中携带 config_key）
//! - 离线时配置变更仍入队为 `pending`，但前端不主动触发 `sync_run_once`（避免无谓失败）
//! - 若 scheduler 曾尝试同步失败（标记为 `failed`），网络恢复后由 `flush_config_queue`
//!   重置为 `pending`，再由前端触发 `sync_run_once` 完成同步
//! - 配置本地立即生效由前端处理，本服务仅负责入队与同步

use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::services::sync_service;

/// 配置同步队列的 table_name 标识
///
/// 所有配置变更入队时使用此 table_name，与普通数据表（todos / kb_entries 等）区分。
/// `flush_config_queue` 与 `get_pending_config_count` 均按此字段过滤。
pub const CONFIG_TABLE_NAME: &str = "app_config";

/// 配置条目的 record_id
///
/// 配置项以 `config_key`（存于 payload）标识，无自增 ID，统一用 0。
/// 同一 key 的多次变更会产生多条队列记录，scheduler 同步时按 LWW 合并。
pub const CONFIG_RECORD_ID: i64 = 0;

/// 记录配置变更到 sync_queue
///
/// - 配置本地立即生效由前端处理（此函数仅负责入队）
/// - payload 格式：`{"key": "<config_key>", "value": "<config_value>"}`
/// - 入队后状态为默认 `pending`，等待 scheduler 同步
/// - 离线场景：前端不触发 `sync_run_once`，条目保持 `pending`；
///   网络恢复后由 `flush_config_queue` 重置 `failed` 条目，再触发同步
pub async fn record_config_change(
    pool: &SqlitePool,
    config_key: &str,
    config_value: &str,
) -> Result<(), AppError> {
    let payload = serde_json::json!({
        "key": config_key,
        "value": config_value,
    })
    .to_string();

    sync_service::enqueue(
        pool,
        CONFIG_TABLE_NAME,
        CONFIG_RECORD_ID,
        sync_service::SyncOperation::Update,
        &payload,
        None,
    )
    .await?;

    tracing::debug!(
        "[config_sync] 配置变更已入队: key={}, value_len={}",
        config_key,
        config_value.len()
    );
    Ok(())
}

/// 网络恢复时触发：将 `failed` 的配置条目重置为 `pending`
///
/// - 仅影响 `table_name = 'app_config'` 且 `sync_status = 'failed'` 的条目
/// - 重置 `last_error = NULL`、`retry_count = 0`，让 scheduler 重新尝试
/// - 返回受影响的行数（若为 0 表示无 failed 条目需要 flush）
///
/// 调用时机：`connectivity::check_now` 检测到 `is_online: false → true` 时自动触发
/// （见 `services/sync/connectivity.rs`）。
pub async fn flush_config_queue(pool: &SqlitePool) -> Result<i64, AppError> {
    let result = sqlx::query(
        "UPDATE sync_queue
         SET sync_status = 'pending', last_error = NULL, retry_count = 0
         WHERE table_name = ? AND sync_status = 'failed'",
    )
    .bind(CONFIG_TABLE_NAME)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    let affected = result.rows_affected() as i64;
    if affected > 0 {
        tracing::info!(
            "[config_sync] 已 flush {} 条 failed 配置变更到 pending",
            affected
        );
    }
    Ok(affected)
}

/// 获取待发配置条数（`pending` + `failed`）
///
/// 前端用于展示"有 N 条配置变更等待同步"。
/// 包含 `failed` 是因为它们也属于"待发"（等待网络恢复后重试）。
pub async fn get_pending_config_count(pool: &SqlitePool) -> Result<i64, AppError> {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sync_queue
         WHERE table_name = ? AND sync_status IN ('pending', 'failed')",
    )
    .bind(CONFIG_TABLE_NAME)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(count.0)
}

#[cfg(test)]
mod tests {
    // 单元测试需要内存 SQLite 池，集成验证依赖实际迁移环境。
    // 此处保留测试骨架，后续可在 tests/ 目录补充集成测试。
    use super::*;

    #[test]
    fn test_config_table_name_constant() {
        assert_eq!(CONFIG_TABLE_NAME, "app_config");
    }

    #[test]
    fn test_config_record_id_constant() {
        assert_eq!(CONFIG_RECORD_ID, 0);
    }
}
