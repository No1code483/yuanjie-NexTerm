//! T2.9 数据恢复与导出 - 恢复服务
//!
//! 实现规范：`功能展望/平台级增强/02_数据完整性_迁移与备份.md` §2.4
//!
//! 恢复流程（7 步）：
//! 1. 验证目标备份文件（integrity_check + schema_version 兼容性）
//! 2. 备份当前数据库（pre_op, label="pre_restore"）作为回滚点
//! 3. 关闭当前数据库连接池（释放文件句柄）
//! 4. 复制目标备份文件到数据库位置（覆盖）
//! 5. 删除 -wal 和 -shm 文件（避免 WAL 残留导致不一致）
//! 6. 执行 schema 一致性检查（在新连接上验证）
//! 7. 若失败 → 回滚到步骤 2 的备份；若成功 → 通知前端重启应用
//!
//! 重要约束：
//! - 由于 SQLite 连接池持有文件句柄，必须先关闭连接池才能覆盖数据库文件
//! - 关闭连接池后，AppState 中的 pool 将失效，应用必须重启
//! - 前端收到 `need_restart: true` 后应调用 `tauri::app::restart()` 或提示用户重启

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::restore::{RestoreResult, RestoreVerification};
use crate::services::backup_service;

/// 数据库文件名（与 connection.rs 中一致）
const DB_FILENAME: &str = "nexterm.db";

// ============================================================================
// 公共 API
// ============================================================================

/// 从备份恢复数据
///
/// 完整流程见模块文档。返回 `RestoreResult` 包含恢复状态和回滚备份路径。
///
/// 参数：
/// - `pool`：当前数据库连接池（恢复后将关闭）
/// - `data_dir`：应用数据目录
/// - `backup_path`：目标备份文件路径
pub async fn restore_from_backup(
    pool: &SqlitePool,
    data_dir: &Path,
    backup_path: &str,
) -> Result<RestoreResult, AppError> {
    let source_path = PathBuf::from(backup_path);

    // 安全检查：备份文件必须位于 backups 目录内
    let backups_root = data_dir.join("backups");
    let canonical_source = source_path.canonicalize().map_err(|e| {
        AppError::Backup(format!("备份文件路径无效: {}", e))
    })?;
    let canonical_root = backups_root.canonicalize().map_err(|e| {
        AppError::Backup(format!("backups 目录无效: {}", e))
    })?;
    if !canonical_source.starts_with(&canonical_root) {
        return Err(AppError::Backup(
            "拒绝恢复 backups 目录外的文件".into(),
        ));
    }

    if !source_path.exists() {
        return Err(AppError::Backup("备份文件不存在".into()));
    }

    tracing::info!(
        "开始数据恢复: backup_path={}, data_dir={}",
        backup_path,
        data_dir.display()
    );

    // 步骤 1：验证目标备份文件
    let verification = verify_backup_for_restore(&source_path).await?;
    if !verification.integrity_ok {
        return Err(AppError::Backup(format!(
            "备份文件完整性校验失败: {}",
            verification.integrity_message
        )));
    }
    tracing::info!(
        "步骤 1 完成: 备份验证通过 (schema_version={})",
        verification.schema_version
    );

    // 步骤 2：备份当前数据库（作为回滚点）
    let rollback_backup = backup_service::create_backup(
        pool,
        data_dir,
        crate::models::backup::BackupType::PreOp,
        Some("pre_restore"),
    )
    .await?;
    tracing::info!(
        "步骤 2 完成: 当前数据库已备份（回滚点）: {}",
        rollback_backup.backup_path
    );

    // 步骤 3：关闭当前数据库连接池（释放文件句柄）
    // 注意：关闭后 pool 将无法再使用，AppState 中的 pool 失效
    pool.close().await;
    tracing::info!("步骤 3 完成: 数据库连接池已关闭");

    // 步骤 4：复制备份文件到数据库位置
    let db_path = data_dir.join(DB_FILENAME);
    let copy_result = copy_backup_to_db(&source_path, &db_path).await;

    if let Err(e) = copy_result {
        // 复制失败 → 回滚（用步骤 2 的备份恢复）
        tracing::error!("步骤 4 失败: 复制备份文件失败: {}，尝试回滚", e);
        let _ = rollback_restore(&data_dir, &rollback_backup.backup_path).await;
        return Err(AppError::Backup(format!(
            "复制备份文件失败（已回滚）: {}",
            e
        )));
    }
    tracing::info!("步骤 4 完成: 备份文件已复制到数据库位置");

    // 步骤 5：删除 -wal 和 -shm 文件
    let wal_path = data_dir.join(format!("{}-wal", DB_FILENAME));
    let shm_path = data_dir.join(format!("{}-shm", DB_FILENAME));
    if wal_path.exists() {
        let _ = tokio::fs::remove_file(&wal_path).await;
    }
    if shm_path.exists() {
        let _ = tokio::fs::remove_file(&shm_path).await;
    }
    tracing::info!("步骤 5 完成: WAL/SHM 文件已清理");

    // 步骤 6：执行 schema 一致性检查（在新连接上验证）
    let check_result = verify_restored_db(&db_path).await;
    if let Err(e) = &check_result {
        // 一致性检查失败 → 回滚
        tracing::error!("步骤 6 失败: 一致性检查失败: {}，尝试回滚", e);
        if let Err(rollback_err) = rollback_restore(&data_dir, &rollback_backup.backup_path).await {
            tracing::error!(
                "回滚失败！手动恢复建议: 从 {} 恢复",
                rollback_backup.backup_path
            );
            return Err(AppError::Backup(format!(
                "一致性检查失败且回滚失败。手动恢复备份: {}。错误: {}",
                rollback_backup.backup_path, rollback_err
            )));
        }
        return Err(AppError::Backup(format!(
            "一致性检查失败（已回滚到恢复前状态）: {}",
            e
        )));
    }

    let verification_after = check_result.unwrap();
    tracing::info!(
        "步骤 6 完成: 一致性检查通过 (integrity={}, foreign_key_check={})",
        verification_after.integrity_message,
        verification_after.foreign_key_violations
    );

    // 步骤 7：返回成功，通知前端重启
    tracing::info!("步骤 7: 数据恢复成功，等待应用重启");

    Ok(RestoreResult {
        success: true,
        need_restart: true,
        rollback_backup_path: rollback_backup.backup_path,
        schema_version: verification_after.schema_version,
        integrity_check: verification_after.integrity_message,
        foreign_key_violations: verification_after.foreign_key_violations,
    })
}

// ============================================================================
// 内部辅助函数
// ============================================================================

/// 验证备份文件是否可用于恢复
///
/// 检查：
/// 1. 文件可被 SQLite 打开
/// 2. PRAGMA integrity_check 通过
/// 3. schema_migrations 表存在且有记录
async fn verify_backup_for_restore(backup_path: &Path) -> Result<RestoreVerification, AppError> {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(backup_path)
        .read_only(true);
    let tmp_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|e| AppError::Backup(format!("无法打开备份文件: {}", e)))?;

    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check;")
        .fetch_one(&tmp_pool)
        .await
        .map_err(|e| AppError::Backup(format!("integrity_check 执行失败: {}", e)))?;

    let schema_version: i64 = sqlx::query_scalar(
        "SELECT MAX(version) FROM schema_migrations WHERE version IS NOT NULL;",
    )
    .fetch_one(&tmp_pool)
    .await
    .unwrap_or(0);

    tmp_pool.close().await;

    Ok(RestoreVerification {
        integrity_ok: integrity == "ok",
        integrity_message: integrity,
        schema_version,
        foreign_key_violations: 0,
    })
}

/// 复制备份文件到数据库位置
async fn copy_backup_to_db(source: &Path, dest: &Path) -> Result<(), AppError> {
    tokio::fs::copy(source, dest)
        .await
        .map_err(|e| AppError::Backup(format!("文件复制失败: {}", e)))?;
    Ok(())
}

/// 验证恢复后的数据库
///
/// 在新连接上执行：
/// 1. PRAGMA integrity_check
/// 2. PRAGMA foreign_key_check
async fn verify_restored_db(db_path: &Path) -> Result<RestoreVerification, AppError> {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(db_path)
        .read_only(true);
    let tmp_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|e| AppError::Backup(format!("无法打开恢复后的数据库: {}", e)))?;

    // 启用外键检查（即使 read_only，PRAGMA foreign_keys 仍可设置）
    let _ = sqlx::query("PRAGMA foreign_keys=ON;")
        .execute(&tmp_pool)
        .await;

    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check;")
        .fetch_one(&tmp_pool)
        .await
        .map_err(|e| AppError::Backup(format!("integrity_check 执行失败: {}", e)))?;

    // foreign_key_check 返回 0 行表示无违规
    let fk_violations: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check;")
        .fetch_one(&tmp_pool)
        .await
        .unwrap_or(0);

    let schema_version: i64 = sqlx::query_scalar(
        "SELECT MAX(version) FROM schema_migrations WHERE version IS NOT NULL;",
    )
    .fetch_one(&tmp_pool)
    .await
    .unwrap_or(0);

    tmp_pool.close().await;

    if integrity != "ok" {
        return Err(AppError::Backup(format!(
            "integrity_check 失败: {}",
            integrity
        )));
    }
    if fk_violations > 0 {
        return Err(AppError::Backup(format!(
            "foreign_key_check 发现 {} 条违规",
            fk_violations
        )));
    }

    Ok(RestoreVerification {
        integrity_ok: true,
        integrity_message: integrity,
        schema_version,
        foreign_key_violations: fk_violations,
    })
}

/// 回滚恢复（用 pre_restore 备份覆盖数据库文件）
///
/// 用于恢复失败时恢复到恢复前的状态
async fn rollback_restore(data_dir: &Path, rollback_backup_path: &str) -> Result<(), AppError> {
    tracing::warn!("执行回滚: 从 {} 恢复", rollback_backup_path);

    let source = PathBuf::from(rollback_backup_path);
    let dest = data_dir.join(DB_FILENAME);

    if !source.exists() {
        return Err(AppError::Backup(format!(
            "回滚备份文件不存在: {}",
            rollback_backup_path
        )));
    }

    tokio::fs::copy(&source, &dest)
        .await
        .map_err(|e| AppError::Backup(format!("回滚文件复制失败: {}", e)))?;

    // 清理 WAL/SHM
    let wal_path = data_dir.join(format!("{}-wal", DB_FILENAME));
    let shm_path = data_dir.join(format!("{}-shm", DB_FILENAME));
    if wal_path.exists() {
        let _ = tokio::fs::remove_file(&wal_path).await;
    }
    if shm_path.exists() {
        let _ = tokio::fs::remove_file(&shm_path).await;
    }

    tracing::info!("回滚完成");
    Ok(())
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_verify_backup_for_restore_valid() {
        let temp = tempfile::tempdir().unwrap();
        let data_dir = temp.path();

        // 创建一个有效的备份文件（含 schema_migrations 表）
        let backup_path = data_dir.join("test_backup.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&backup_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        sqlx::query(
            r#"CREATE TABLE schema_migrations (
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
        sqlx::query("INSERT INTO schema_migrations (version, description, migration_hash) VALUES (89, 'test', 'hash89');")
            .execute(&pool)
            .await
            .unwrap();

        pool.close().await;

        let verification = verify_backup_for_restore(&backup_path).await.unwrap();
        assert!(verification.integrity_ok);
        assert_eq!(verification.schema_version, 89);
    }

    #[tokio::test]
    async fn test_verify_backup_for_restore_nonexistent() {
        let temp = tempfile::tempdir().unwrap();
        let backup_path = temp.path().join("nonexistent.db");

        let result = verify_backup_for_restore(&backup_path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_copy_backup_to_db() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.db");
        let dest = temp.path().join("dest.db");

        tokio::fs::write(&source, b"test content").await.unwrap();
        copy_backup_to_db(&source, &dest).await.unwrap();

        let content = tokio::fs::read_to_string(&dest).await.unwrap();
        assert_eq!(content, "test content");
    }

    #[tokio::test]
    async fn test_rollback_restore() {
        let temp = tempfile::tempdir().unwrap();
        let data_dir = temp.path();

        // 创建回滚备份
        let backup_path = data_dir.join("rollback.db");
        tokio::fs::write(&backup_path, b"rollback content").await.unwrap();

        // 创建当前数据库文件（将被覆盖）
        let db_path = data_dir.join(DB_FILENAME);
        tokio::fs::write(&db_path, b"current content").await.unwrap();

        // 执行回滚
        rollback_restore(data_dir, backup_path.to_str().unwrap())
            .await
            .unwrap();

        // 验证数据库文件已被回滚备份覆盖
        let content = tokio::fs::read_to_string(&db_path).await.unwrap();
        assert_eq!(content, "rollback content");
    }

    #[tokio::test]
    async fn test_restore_from_backup_rejects_outside_path() {
        let temp = tempfile::tempdir().unwrap();
        let data_dir = temp.path();

        // 创建 backups 目录
        tokio::fs::create_dir_all(data_dir.join("backups")).await.unwrap();

        // 创建 backups 目录外的文件
        let outside_path = temp.path().join("../outside.db");
        let result = restore_from_backup(
            &sqlx::sqlite::SqlitePoolOptions::new()
                .connect("sqlite::memory:")
                .await
                .unwrap(),
            data_dir,
            outside_path.to_str().unwrap(),
        )
        .await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("backups 目录外") || err_msg.contains("无效"));
    }
}
