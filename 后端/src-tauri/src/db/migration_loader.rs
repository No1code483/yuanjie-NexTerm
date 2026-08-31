//! T2.6 数据完整性 - 迁移系统升级（loader 基础设施）
//!
//! 实现规范：`功能展望/平台级增强/02_数据完整性_迁移与备份.md` §2.1
//!
//! 核心特性：
//! 1. 升级版 `schema_migrations` 表（含 description/applied_by/migration_hash/rollback_script/checksum_verified）
//! 2. 启动短路机制（已应用版本集合 `HashSet` 命中即跳过）
//! 3. 迁移 hash 防篡改（SHA256）
//! 4. 宏 `apply_migration!` 统一封装"检查未应用 → 执行 → 记录"流程
//!
//! 当前阶段（T2.6.1）：仅提供基础设施与 RustFn 调用通道，现有 80+ inline 函数通过宏逐步注册。
//! 后续阶段（T2.6.2-T2.6.5）：将 inline 函数提取为 `.sql` 文件，通过 `apply_sql_migration!` 宏调用。

use std::collections::HashSet;

use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::error::app_error::AppError;

/// 升级版 `schema_migrations` 表 DDL
///
/// 字段说明：
/// - `version`：迁移版本号（INTEGER PRIMARY KEY，单调递增）
/// - `description`：人类可读描述（迁移函数名或 .sql 文件名）
/// - `applied_at`：应用时间戳（datetime 默认值）
/// - `applied_by`：执行者（用户名，用于审计；当前阶段固定为 NULL）
/// - `migration_hash`：SHA256 hash（防篡改；RustFn 阶段为函数名 hash，.sql 阶段为 SQL 内容 hash）
/// - `rollback_script`：回滚 SQL（T2.6 阶段保留为 NULL，待 T2.8 备份系统就绪后实现）
/// - `checksum_verified`：hash 校验状态（1=已验证，0=待校验；当前阶段固定为 1）
pub const SCHEMA_MIGRATIONS_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    description TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT (datetime('now')),
    applied_by TEXT,
    migration_hash TEXT NOT NULL,
    rollback_script TEXT,
    checksum_verified INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_schema_migrations_version ON schema_migrations(version);
"#;

/// 确保 `schema_migrations` 表存在（幂等）
pub async fn ensure_schema_migrations_table(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query(SCHEMA_MIGRATIONS_DDL)
        .execute(pool)
        .await?;
    Ok(())
}

/// 查询已应用的迁移版本集合
///
/// 返回 `HashSet<i64>`，用于 O(1) 判断某个版本是否已应用。
pub async fn get_applied_versions(pool: &SqlitePool) -> Result<HashSet<i64>, AppError> {
    let versions: Vec<i64> = sqlx::query_scalar("SELECT version FROM schema_migrations;")
        .fetch_all(pool)
        .await?;
    Ok(versions.into_iter().collect())
}

/// 查询当前已应用的最大版本号（用于短路日志）
pub async fn get_current_version(pool: &SqlitePool) -> Result<Option<i64>, AppError> {
    let version: Option<i64> = sqlx::query_scalar("SELECT MAX(version) FROM schema_migrations;")
        .fetch_one(pool)
        .await?;
    Ok(version)
}

/// 记录已应用的迁移到 `schema_migrations` 表
pub async fn record_migration(
    pool: &SqlitePool,
    version: u32,
    description: &str,
    hash: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO schema_migrations (version, description, migration_hash)
         VALUES (?, ?, ?);",
    )
    .bind(version as i64)
    .bind(description)
    .bind(hash)
    .execute(pool)
    .await?;
    Ok(())
}

/// 计算字符串的 SHA256 hash（hex 编码）
///
/// T2.6.1 阶段：输入为迁移函数名（如 `create_users_table`）
/// T2.6.2+ 阶段：输入为 .sql 文件内容
pub fn compute_hash(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hex::encode(hasher.finalize())
}

/// 校验已记录的迁移 hash 是否与当前 hash 一致（防篡改）
///
/// 返回不一致的版本列表（空表示全部一致）
pub async fn verify_migration_hashes(
    pool: &SqlitePool,
    expected_hashes: &[(u32, &str)],
) -> Result<Vec<u32>, AppError> {
    let mut tampered = Vec::new();
    for (version, expected_hash_source) in expected_hashes {
        let expected_hash = compute_hash(expected_hash_source);
        let recorded: Option<String> =
            sqlx::query_scalar("SELECT migration_hash FROM schema_migrations WHERE version = ?;")
                .bind(*version as i64)
                .fetch_optional(pool)
                .await?;
        match recorded {
            None => {
                tracing::warn!("Migration v{} not recorded (skipped)", version);
            }
            Some(recorded_hash) if recorded_hash != expected_hash => {
                tracing::error!(
                    "Migration v{} hash mismatch: expected {}, got {}",
                    version,
                    expected_hash,
                    recorded_hash
                );
                tampered.push(*version);
            }
            _ => {}
        }
    }
    Ok(tampered)
}

/// 应用迁移的统一宏
///
/// 调用方式：`apply_migration!(pool, applied, 1, create_users_table);`
///
/// 展开为：
/// 1. 检查版本是否已在 `applied` 集合中 → 是则跳过
/// 2. 调用迁移函数 `create_users_table(pool).await?`
/// 3. 记录到 `schema_migrations` 表（version=1, description="create_users_table", hash=SHA256("create_users_table")）
///
/// 注意：宏内含 `?` 与 `.await`，必须在 `async fn` 中调用。
#[macro_export]
macro_rules! apply_migration {
    ($pool:expr, $applied:expr, $version:expr, $func:ident) => {
        if !$applied.contains(&($version as i64)) {
            tracing::info!("Applying migration v{}: {}", $version, stringify!($func));
            $func($pool).await?;
            $crate::db::migration_loader::record_migration(
                $pool,
                $version,
                stringify!($func),
                &$crate::db::migration_loader::compute_hash(stringify!($func)),
            )
            .await?;
        }
    };
}

/// 应用 SQL 文件迁移的宏（T2.6.2+ 阶段使用）
///
/// 调用方式：`apply_sql_migration!(pool, applied, 81, "create_new_table", sql_text);`
///
/// 与 `apply_migration!` 区别：
/// - 直接执行 SQL 字符串（而非调用 Rust 函数）
/// - hash 基于 SQL 内容（而非函数名）
#[macro_export]
macro_rules! apply_sql_migration {
    ($pool:expr, $applied:expr, $version:expr, $description:expr, $sql:expr) => {
        if !$applied.contains(&($version as i64)) {
            tracing::info!("Applying SQL migration v{}: {}", $version, $description);
            sqlx::query($sql).execute($pool).await?;
            $crate::db::migration_loader::record_migration(
                $pool,
                $version,
                $description,
                &$crate::db::migration_loader::compute_hash($sql),
            )
            .await?;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash_deterministic() {
        let h1 = compute_hash("create_users_table");
        let h2 = compute_hash("create_users_table");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA256 hex = 64 chars
    }

    #[test]
    fn test_compute_hash_distinct() {
        let h1 = compute_hash("create_users_table");
        let h2 = compute_hash("create_permissions_table");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_compute_hash_known_value() {
        // SHA256("test") = 9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08
        let h = compute_hash("test");
        assert_eq!(
            h,
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
    }

    #[tokio::test]
    async fn test_ensure_schema_migrations_table_idempotent() {
        // 使用内存数据库测试
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // 第一次创建
        ensure_schema_migrations_table(&pool).await.unwrap();
        // 第二次创建（幂等）
        ensure_schema_migrations_table(&pool).await.unwrap();

        // 验证表存在
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_migrations';",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_record_and_get_versions() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        ensure_schema_migrations_table(&pool).await.unwrap();

        // 初始为空
        let applied = get_applied_versions(&pool).await.unwrap();
        assert!(applied.is_empty());

        // 记录 3 个迁移
        record_migration(&pool, 1, "migrate_1", &compute_hash("migrate_1"))
            .await
            .unwrap();
        record_migration(&pool, 2, "migrate_2", &compute_hash("migrate_2"))
            .await
            .unwrap();
        record_migration(&pool, 5, "migrate_5", &compute_hash("migrate_5"))
            .await
            .unwrap();

        // 验证集合
        let applied = get_applied_versions(&pool).await.unwrap();
        assert_eq!(applied.len(), 3);
        assert!(applied.contains(&1));
        assert!(applied.contains(&2));
        assert!(applied.contains(&5));
        assert!(!applied.contains(&3));

        // 验证当前版本（MAX）
        let current = get_current_version(&pool).await.unwrap();
        assert_eq!(current, Some(5));
    }

    #[tokio::test]
    async fn test_verify_migration_hashes_all_match() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        ensure_schema_migrations_table(&pool).await.unwrap();
        record_migration(&pool, 1, "migrate_1", &compute_hash("migrate_1"))
            .await
            .unwrap();
        record_migration(&pool, 2, "migrate_2", &compute_hash("migrate_2"))
            .await
            .unwrap();

        let expected: Vec<(u32, &str)> = vec![(1, "migrate_1"), (2, "migrate_2")];
        let tampered = verify_migration_hashes(&pool, &expected).await.unwrap();
        assert!(tampered.is_empty());
    }

    #[tokio::test]
    async fn test_verify_migration_hashes_detects_tamper() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        ensure_schema_migrations_table(&pool).await.unwrap();
        // 故意记录错误的 hash（模拟篡改）
        record_migration(&pool, 1, "migrate_1", "tampered_hash_value")
            .await
            .unwrap();

        let expected: Vec<(u32, &str)> = vec![(1, "migrate_1")];
        let tampered = verify_migration_hashes(&pool, &expected).await.unwrap();
        assert_eq!(tampered, vec![1]);
    }
}
