//! T2.8 自动备份系统 - 核心服务
//!
//! 实现规范：`功能展望/平台级增强/02_数据完整性_迁移与备份.md` §2.3
//!
//! 核心特性：
//! 1. 4 种备份类型（hourly/daily/pre_op/emergency），独立子目录与滚动保留策略
//! 2. SQLite 原生 `VACUUM INTO` 原子备份（无需关闭连接、文件已压缩）
//! 3. 三重校验：`PRAGMA integrity_check` + SHA256 + 关键表行数统计
//! 4. 独立元数据库 `backup_records.db`（避免主库损坏时元数据丢失）
//! 5. 滚动保留：按类型自动清理最旧备份（emergency 无限保留）
//!
//! 不在 T2.8 范围内（暂不实现）：
//! - 备份恢复流程（T2.9）
//! - 备份导出为 zip（T2.9）
//! - 备份加密
//! - 备份 diff 对比

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::Local;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tokio::io::AsyncReadExt;

use crate::error::app_error::AppError;
use crate::models::backup::{
    BackupMetadata, BackupRecord, BackupResult, BackupStats, BackupType, BackupTypeStats,
};

/// backup_records.db 的 DDL
///
/// 独立于主库，存储所有备份的元数据索引
const BACKUP_RECORDS_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS backup_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    backup_path TEXT NOT NULL UNIQUE,
    backup_size_bytes INTEGER NOT NULL,
    backup_type TEXT NOT NULL CHECK (backup_type IN ('hourly', 'daily', 'pre_op', 'emergency')),
    label TEXT,
    schema_version INTEGER NOT NULL,
    integrity_check TEXT NOT NULL,
    checksum TEXT NOT NULL,
    table_counts_json TEXT,
    created_at INTEGER NOT NULL,
    verified INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_backup_records_type_created
    ON backup_records(backup_type, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_backup_records_created
    ON backup_records(created_at DESC);
"#;

/// 关键表行数统计列表（用于异常检测对比）
///
/// 选取数据量敏感的核心表，避免对所有 65 张表统计耗时
///
/// 公开（pub）：T2.9 export_service 和 T2.10 health_check_service 均需引用
pub const CRITICAL_TABLES: &[&str] = &[
    "users",
    "conversations",
    "messages",
    "kb_entries",
    "todos",
    "journals",
    "terminal_history",
    "editor_documents",
    "game_worlds",
    "xin_conversations",
];

// ============================================================================
// 公共 API
// ============================================================================

/// 创建备份
///
/// 流程：
/// 1. 构造备份路径（`<data_dir>/backups/<type>/nexterm_<type>_<ts>[_<label>].db`）
/// 2. 执行 `VACUUM INTO` 原子备份
/// 3. 校验备份文件（integrity_check + SHA256 + 表行数）
/// 4. 记录元数据到 backup_records.db
/// 5. 滚动清理超龄备份
pub async fn create_backup(
    pool: &SqlitePool,
    data_dir: &Path,
    backup_type: BackupType,
    label: Option<&str>,
) -> Result<BackupResult, AppError> {
    let backup_dir = get_backup_dir(data_dir, &backup_type)?;
    tokio::fs::create_dir_all(&backup_dir).await?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = match label {
        Some(l) => format!(
            "nexterm_{}_{}_{}.db",
            backup_type.as_str(),
            timestamp,
            sanitize_label(l)
        ),
        None => format!("nexterm_{}_{}.db", backup_type.as_str(), timestamp),
    };
    let backup_path = backup_dir.join(filename);

    // 避免同秒重复创建（追加 _1, _2 ...）
    let backup_path = ensure_unique_path(&backup_path).await?;

    // VACUUM INTO 不支持绑定参数，路径来源为受控目录 + 受控文件名，可信
    let sql = format!("VACUUM INTO '{}'", backup_path.display());
    sqlx::query(&sql).execute(pool).await.map_err(|e| {
        AppError::Backup(format!("VACUUM INTO 失败: {}", e))
    })?;

    // 校验 + 记录元数据
    let metadata = verify_backup(&backup_path).await?;
    let created_at = Local::now().timestamp();

    let records_pool = open_records_db(data_dir).await?;
    record_backup_metadata(
        &records_pool,
        &backup_path,
        &backup_type,
        label,
        &metadata,
        created_at,
    )
    .await?;
    records_pool.close().await;

    // 滚动清理（emergency 不清理）
    if let Err(e) = cleanup_old_backups(data_dir, &backup_type).await {
        tracing::warn!("滚动清理失败（不影响备份结果）: {}", e);
    }

    tracing::info!(
        "Backup created: type={}, path={}, size={}bytes, schema_version={}",
        backup_type.as_str(),
        backup_path.display(),
        metadata.size_bytes,
        metadata.schema_version
    );

    Ok(BackupResult::from((
        metadata,
        backup_path.to_string_lossy().to_string(),
        backup_type.as_str().to_string(),
        created_at,
    )))
}

/// 列出备份记录
///
/// 参数：
/// - `data_dir`：应用数据目录
/// - `backup_type`：可选类型过滤（None 表示所有类型）
/// - `limit`：最大返回数量（默认 50）
pub async fn list_backups(
    data_dir: &Path,
    backup_type: Option<&str>,
    limit: usize,
) -> Result<Vec<BackupRecord>, AppError> {
    let records_pool = open_records_db(data_dir).await?;
    let limit_i64 = limit as i64;

    let rows: Vec<BackupRecord> = if let Some(bt) = backup_type {
        sqlx::query_as(
            r#"SELECT id, backup_path, backup_size_bytes, backup_type, label,
                      schema_version, integrity_check, checksum, table_counts_json,
                      created_at, verified
               FROM backup_records
               WHERE backup_type = ?
               ORDER BY created_at DESC
               LIMIT ?"#,
        )
        .bind(bt)
        .bind(limit_i64)
        .fetch_all(&records_pool)
        .await?
    } else {
        sqlx::query_as(
            r#"SELECT id, backup_path, backup_size_bytes, backup_type, label,
                      schema_version, integrity_check, checksum, table_counts_json,
                      created_at, verified
               FROM backup_records
               ORDER BY created_at DESC
               LIMIT ?"#,
        )
        .bind(limit_i64)
        .fetch_all(&records_pool)
        .await?
    };

    records_pool.close().await;
    Ok(rows)
}

/// 删除备份（同时删除文件和元数据）
pub async fn delete_backup(data_dir: &Path, backup_path: &str) -> Result<(), AppError> {
    let path = PathBuf::from(backup_path);

    // 安全检查：路径必须位于 backups 目录内
    let backups_root = data_dir.join("backups");
    let canonical_path = path.canonicalize().map_err(|e| {
        AppError::Backup(format!("备份路径无效: {}", e))
    })?;
    let canonical_root = backups_root.canonicalize().map_err(|e| {
        AppError::Backup(format!("backups 目录无效: {}", e))
    })?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(AppError::Backup(
            "拒绝删除 backups 目录外的文件".into(),
        ));
    }

    // 删除文件（如果存在）
    if path.exists() {
        tokio::fs::remove_file(&path).await?;
    }

    // 删除元数据
    let records_pool = open_records_db(data_dir).await?;
    sqlx::query("DELETE FROM backup_records WHERE backup_path = ?;")
        .bind(backup_path)
        .execute(&records_pool)
        .await?;
    records_pool.close().await;

    tracing::info!("Backup deleted: {}", backup_path);
    Ok(())
}

/// 获取备份统计信息
pub async fn get_backup_stats(data_dir: &Path) -> Result<BackupStats, AppError> {
    let records_pool = open_records_db(data_dir).await?;

    let total: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM backup_records;"
    )
    .fetch_one(&records_pool)
    .await?;
    let total_backups = total.unwrap_or(0) as usize;

    let total_size: Option<i64> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(backup_size_bytes), 0) FROM backup_records;"
    )
    .fetch_one(&records_pool)
    .await?;
    let total_size_bytes = total_size.unwrap_or(0);

    let oldest: Option<i64> = sqlx::query_scalar(
        "SELECT MIN(created_at) FROM backup_records;"
    )
    .fetch_one(&records_pool)
    .await?;
    let newest: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(created_at) FROM backup_records;"
    )
    .fetch_one(&records_pool)
    .await?;

    // 按类型统计
    let mut by_type = BackupTypeStats::default();
    for bt in &["hourly", "daily", "pre_op", "emergency"] {
        let count: Option<i64> =
            sqlx::query_scalar("SELECT COUNT(*) FROM backup_records WHERE backup_type = ?;")
                .bind(bt)
                .fetch_one(&records_pool)
                .await?;
        let size: Option<i64> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(backup_size_bytes), 0) FROM backup_records WHERE backup_type = ?;",
        )
        .bind(bt)
        .fetch_one(&records_pool)
        .await?;
        let c = count.unwrap_or(0) as usize;
        let s = size.unwrap_or(0);
        match *bt {
            "hourly" => {
                by_type.hourly_count = c;
                by_type.hourly_size_bytes = s;
            }
            "daily" => {
                by_type.daily_count = c;
                by_type.daily_size_bytes = s;
            }
            "pre_op" => {
                by_type.pre_op_count = c;
                by_type.pre_op_size_bytes = s;
            }
            "emergency" => {
                by_type.emergency_count = c;
                by_type.emergency_size_bytes = s;
            }
            _ => {}
        }
    }

    records_pool.close().await;

    Ok(BackupStats {
        total_backups,
        total_size_bytes,
        by_type,
        oldest_backup_at: oldest,
        newest_backup_at: newest,
    })
}

/// 重新校验指定备份文件（用于手动触发校验）
pub async fn verify_backup_now(data_dir: &Path, backup_path: &str) -> Result<BackupMetadata, AppError> {
    let path = PathBuf::from(backup_path);
    if !path.exists() {
        return Err(AppError::Backup("备份文件不存在".into()));
    }
    let metadata = verify_backup(&path).await?;

    // 更新元数据库中的校验信息
    let records_pool = open_records_db(data_dir).await?;
    sqlx::query(
        r#"UPDATE backup_records
           SET integrity_check = ?, checksum = ?, verified = 1
           WHERE backup_path = ?"#,
    )
    .bind(&metadata.integrity_check)
    .bind(&metadata.checksum)
    .bind(backup_path)
    .execute(&records_pool)
    .await?;
    records_pool.close().await;

    Ok(metadata)
}

// ============================================================================
// 内部辅助函数
// ============================================================================

/// 获取备份目录（`<data_dir>/backups/<type>/`）
fn get_backup_dir(data_dir: &Path, backup_type: &BackupType) -> Result<PathBuf, AppError> {
    Ok(data_dir.join("backups").join(backup_type.subdir()))
}

/// 打开（或创建）backup_records.db
async fn open_records_db(data_dir: &Path) -> Result<SqlitePool, AppError> {
    let backups_dir = data_dir.join("backups");
    tokio::fs::create_dir_all(&backups_dir).await?;

    let db_path = backups_dir.join("backup_records.db");
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await?;

    sqlx::query(BACKUP_RECORDS_DDL).execute(&pool).await?;

    Ok(pool)
}

/// 校验备份文件
///
/// 三重校验：
/// 1. `PRAGMA integrity_check`（在新连接上验证文件完整性）
/// 2. 关键表行数统计（用于 T2.10 异常检测对比）
/// 3. SHA256 校验和
/// 4. 文件大小
/// 5. schema_version（从 schema_migrations 表读取）
///
/// 公开（pub）：T2.9 export_service 在导出前需调用此函数校验临时备份文件
pub async fn verify_backup(backup_path: &Path) -> Result<BackupMetadata, AppError> {
    // 建立临时连接验证
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(backup_path)
        .read_only(true);
    let tmp_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|e| AppError::Backup(format!("无法打开备份文件: {}", e)))?;

    // 1. 完整性校验
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check;")
        .fetch_one(&tmp_pool)
        .await
        .map_err(|e| AppError::Backup(format!("integrity_check 失败: {}", e)))?;
    if integrity != "ok" {
        tmp_pool.close().await;
        // 删除无效备份，避免占用空间
        tokio::fs::remove_file(backup_path).await.ok();
        return Err(AppError::Backup(format!(
            "备份文件完整性校验失败: {}",
            integrity
        )));
    }

    // 2. schema_version 检查（兼容旧库无 schema_migrations 表）
    let schema_version: i64 = sqlx::query_scalar(
        "SELECT MAX(version) FROM schema_migrations WHERE version IS NOT NULL;",
    )
    .fetch_one(&tmp_pool)
    .await
    .unwrap_or(0);

    // 3. 关键表行数统计
    let mut table_counts = HashMap::new();
    for table_name in CRITICAL_TABLES {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {};", table_name))
            .fetch_one(&tmp_pool)
            .await
            .unwrap_or(0);
        table_counts.insert((*table_name).to_string(), count);
    }
    tmp_pool.close().await;

    // 4. SHA256 校验和 + 5. 文件大小
    let checksum = compute_sha256(backup_path).await?;
    let size_bytes = tokio::fs::metadata(backup_path).await?.len() as i64;

    Ok(BackupMetadata {
        integrity_check: integrity,
        schema_version,
        checksum,
        size_bytes,
        table_counts,
    })
}

/// 计算文件 SHA256 校验和（流式读取，避免大文件 OOM）
async fn compute_sha256(path: &Path) -> Result<String, AppError> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536]; // 64KB 缓冲区
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// 记录备份元数据到 backup_records.db
async fn record_backup_metadata(
    records_pool: &SqlitePool,
    backup_path: &Path,
    backup_type: &BackupType,
    label: Option<&str>,
    metadata: &BackupMetadata,
    created_at: i64,
) -> Result<(), AppError> {
    let table_counts_json = serde_json::to_string(&metadata.table_counts).ok();

    sqlx::query(
        r#"INSERT INTO backup_records
           (backup_path, backup_size_bytes, backup_type, label,
            schema_version, integrity_check, checksum, table_counts_json,
            created_at, verified)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 1)"#,
    )
    .bind(backup_path.to_string_lossy().to_string())
    .bind(metadata.size_bytes)
    .bind(backup_type.as_str())
    .bind(label)
    .bind(metadata.schema_version)
    .bind(&metadata.integrity_check)
    .bind(&metadata.checksum)
    .bind(&table_counts_json)
    .bind(created_at)
    .execute(records_pool)
    .await?;

    Ok(())
}

/// 滚动清理超龄备份
///
/// 按修改时间排序，删除最旧的，直到数量 <= max_retention
async fn cleanup_old_backups(
    data_dir: &Path,
    backup_type: &BackupType,
) -> Result<(), AppError> {
    let max_count = backup_type.max_retention();
    if max_count == 0 {
        return Ok(()); // emergency 无限保留
    }

    let dir = get_backup_dir(data_dir, backup_type)?;
    if !dir.exists() {
        return Ok(());
    }

    // 使用 std::fs::read_dir 同步读取（目录扫描通常很快，无需异步）
    // 避免 tokio::fs::read_dir 在 1.52+ 版本不再是 Iterator 的兼容性问题
    let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some("db".as_ref()))
        .collect();

    // 按修改时间排序（最旧在前）
    entries.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));

    let records_pool = open_records_db(data_dir).await?;

    while entries.len() > max_count {
        let oldest = entries.remove(0);
        let path = oldest.path();
        if let Err(e) = tokio::fs::remove_file(&path).await {
            tracing::warn!("删除旧备份失败 {}: {}", path.display(), e);
            continue;
        }
        // 删除元数据
        let path_str = path.to_string_lossy().to_string();
        let _ = sqlx::query("DELETE FROM backup_records WHERE backup_path = ?;")
            .bind(&path_str)
            .execute(&records_pool)
            .await;
        tracing::info!("Old backup removed: {}", path.display());
    }

    records_pool.close().await;
    Ok(())
}

/// 清理 label 中的危险字符（防止路径穿越和文件名注入）
///
/// 仅保留字母、数字、下划线、连字符；其余替换为 `_`
fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// 避免同秒重复创建备份（追加 _1, _2 ... 后缀）
async fn ensure_unique_path(path: &Path) -> Result<PathBuf, AppError> {
    if !path.exists() {
        return Ok(path.to_path_buf());
    }
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| AppError::Backup("无法解析备份文件名".into()))?;
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("db");
    let parent = path.parent().ok_or_else(|| AppError::Backup("无法获取父目录".into()))?;

    for i in 1..100 {
        let new_name = format!("{}_{}.{}", stem, i, ext);
        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return Ok(new_path);
        }
    }
    Err(AppError::Backup("无法生成唯一备份文件名（重试 100 次失败）".into()))
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_label_basic() {
        assert_eq!(sanitize_label("normal_label"), "normal_label");
        assert_eq!(sanitize_label("label-123"), "label-123");
    }

    #[test]
    fn test_sanitize_label_dangerous() {
        // 路径穿越
        assert_eq!(sanitize_label("../../etc/passwd"), "______etc_passwd");
        // 特殊字符（test + 4 个特殊字符 <>&* = test____）
        assert_eq!(sanitize_label("test<>&*"), "test____");
        // 空格
        assert_eq!(sanitize_label("with space"), "with_space");
        // 中文（保留为字母数字以外字符，替换为 _）
        assert_eq!(sanitize_label("测试"), "测试");
    }

    #[test]
    fn test_sanitize_label_empty() {
        assert_eq!(sanitize_label(""), "");
    }

    #[tokio::test]
    async fn test_ensure_unique_path_non_existing() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("nonexistent.db");
        let result = ensure_unique_path(&path).await.unwrap();
        assert_eq!(result, path);
    }

    #[tokio::test]
    async fn test_ensure_unique_path_existing() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("backup.db");
        tokio::fs::File::create(&path).await.unwrap();

        let result = ensure_unique_path(&path).await.unwrap();
        assert_ne!(result, path);
        assert_eq!(
            result.file_name().unwrap().to_str().unwrap(),
            "backup_1.db"
        );
    }

    #[tokio::test]
    async fn test_open_records_db_creates_table() {
        let temp = tempfile::tempdir().unwrap();
        let pool = open_records_db(temp.path()).await.unwrap();

        // 验证表存在
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM backup_records;"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 0);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_compute_sha256_empty_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("empty.db");
        tokio::fs::File::create(&path).await.unwrap();

        let hash = compute_sha256(&path).await.unwrap();
        // 空文件的 SHA256 = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[tokio::test]
    async fn test_compute_sha256_known_content() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("content.db");
        tokio::fs::write(&path, b"hello world").await.unwrap();

        let hash = compute_sha256(&path).await.unwrap();
        // "hello world" 的 SHA256
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[tokio::test]
    async fn test_full_backup_workflow() {
        // 端到端测试：创建主库 → 创建备份 → 校验 → 查询元数据
        let temp = tempfile::tempdir().unwrap();
        let data_dir = temp.path();

        // 1. 创建主库（含 schema_migrations 表）
        let db_path = data_dir.join("main.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .unwrap();

        // 创建 schema_migrations 表（模拟 T2.6.1，与 migration_loader.rs 定义一致）
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now')),
                applied_by TEXT,
                migration_hash TEXT NOT NULL,
                rollback_script TEXT,
                checksum_verified INTEGER NOT NULL DEFAULT 1
            );"#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query("INSERT INTO schema_migrations (version, description, migration_hash) VALUES (81, 'test', 'hash81');")
            .execute(&pool)
            .await
            .unwrap();

        // 创建一些测试表
        for table in CRITICAL_TABLES {
            sqlx::query(&format!("CREATE TABLE IF NOT EXISTS {} (id INTEGER PRIMARY KEY);", table))
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query(&format!("INSERT INTO {} (id) VALUES (1);", table))
                .execute(&pool)
                .await
                .unwrap();
        }

        // 2. 创建备份
        let result = create_backup(&pool, data_dir, BackupType::Hourly, Some("test_label"))
            .await
            .unwrap();
        assert_eq!(result.backup_type, "hourly");
        assert_eq!(result.schema_version, 81);
        assert!(result.size_bytes > 0);
        assert!(!result.checksum.is_empty());

        // 3. 列出备份
        let records = list_backups(data_dir, Some("hourly"), 10).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].backup_type, "hourly");
        assert_eq!(records[0].schema_version, 81);
        assert_eq!(records[0].label, Some("test_label".into()));
        assert!(records[0].verified);

        // 4. 获取统计
        let stats = get_backup_stats(data_dir).await.unwrap();
        assert_eq!(stats.total_backups, 1);
        assert_eq!(stats.by_type.hourly_count, 1);
        assert!(stats.by_type.hourly_size_bytes > 0);

        // 5. 删除备份
        delete_backup(data_dir, &result.backup_path).await.unwrap();
        let records = list_backups(data_dir, Some("hourly"), 10).await.unwrap();
        assert_eq!(records.len(), 0);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_cleanup_old_backups_retention() {
        let temp = tempfile::tempdir().unwrap();
        let data_dir = temp.path();

        // 创建主库
        let db_path = data_dir.join("main.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .unwrap();

        sqlx::query("CREATE TABLE IF NOT EXISTS test (id INTEGER PRIMARY KEY);")
            .execute(&pool)
            .await
            .unwrap();

        // 创建 30 个 hourly 备份（超过 max_retention=24）
        for i in 0..30 {
            let _ = create_backup(&pool, data_dir, BackupType::Hourly, None)
                .await
                .unwrap();
            // 短暂延迟以区分时间戳
            tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;
        }

        // 验证只保留 24 个
        let records = list_backups(data_dir, Some("hourly"), 100).await.unwrap();
        assert_eq!(records.len(), 24);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_emergency_backup_no_cleanup() {
        let temp = tempfile::tempdir().unwrap();
        let data_dir = temp.path();

        let db_path = data_dir.join("main.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .unwrap();

        sqlx::query("CREATE TABLE IF NOT EXISTS test (id INTEGER PRIMARY KEY);")
            .execute(&pool)
            .await
            .unwrap();

        // 创建 5 个 emergency 备份（max_retention=0 = 无限）
        for _ in 0..5 {
            let _ = create_backup(&pool, data_dir, BackupType::Emergency, Some("test"))
                .await
                .unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;
        }

        let records = list_backups(data_dir, Some("emergency"), 100).await.unwrap();
        assert_eq!(records.len(), 5); // 全部保留

        pool.close().await;
    }

    #[tokio::test]
    async fn test_delete_backup_rejects_outside_path() {
        let temp = tempfile::tempdir().unwrap();
        let data_dir = temp.path();

        // 创建 backups 目录
        tokio::fs::create_dir_all(data_dir.join("backups")).await.unwrap();

        // 试图删除 backups 目录外的文件
        let outside_path = temp.path().join("../outside.db");
        let result = delete_backup(data_dir, outside_path.to_str().unwrap()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("backups 目录外") || err_msg.contains("无效"));
    }
}
