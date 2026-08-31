//! T2.13 MEK 轮换调度器
//!
//! 设计依据：功能展望/平台级增强/05_安全加固_A4.md §2.3.4
//!
//! 调度策略：
//! - 启动后 5 分钟检查首次（避免启动期峰值）
//! - 每 24 小时检查一次
//! - 查询所有 MEK 版本超过 90 天的用户，触发自动轮换
//!
//! 注意：
//! - 调度器仅触发"轮换"（生成新版本 MEK）
//! - 密文迁移由 MekManager::migrate_ciphertexts 异步执行
//! - 紧急轮换（如 KEK 泄露）由用户通过 Tauri 命令手动触发

use std::time::Duration;

use sqlx::SqlitePool;

use crate::crypto::mek_manager::MekManager;
use crate::error::app_error::AppError;

/// 启动 MEK 轮换调度器
///
/// 流程：
/// 1. 启动后 5 分钟（300s）执行首次检查
/// 2. 每 24 小时（86400s）执行一次定时检查
/// 3. 检查时调用 `check_and_rotate` 扫描所有需要轮换的用户
///
/// 注意：本调度器在后台 tokio task 中运行，不阻塞主线程
pub fn start_mek_rotation_scheduler(pool: SqlitePool) {
    tauri::async_runtime::spawn(async move {
        // 1. 启动后 5 分钟首次检查
        tracing::info!("MEK 轮换调度器已启动，5 分钟后首次检查");
        tokio::time::sleep(Duration::from_secs(300)).await;

        // 2. 首次检查
        if let Err(e) = check_and_rotate(&pool).await {
            tracing::warn!(error = %e, "MEK 首次轮换检查失败");
        }

        // 3. 每 24 小时定时检查
        let mut interval = tokio::time::interval(Duration::from_secs(86400));
        interval.tick().await; // 跳过首次立即触发（已在上面执行）
        loop {
            interval.tick().await;
            if let Err(e) = check_and_rotate(&pool).await {
                tracing::warn!(error = %e, "MEK 定时轮换检查失败");
            }
        }
    });
}

/// 执行一次轮换检查
///
/// 查询所有 MEK 版本超过 90 天的用户，对每个用户触发定时轮换。
///
/// 注意：
/// - 本函数不阻塞用户操作（后台异步执行）
/// - 单个用户轮换失败不会影响其他用户
/// - 真实场景下需要 KEK（当前从 key_derivation 模块获取，本调度器简化处理）
pub async fn check_and_rotate(pool: &SqlitePool) -> Result<u64, AppError> {
    let now = chrono::Local::now().timestamp();
    let threshold = now - 90 * 86400; // 90 天前

    // 查询所有需要轮换的用户（当前活跃版本创建时间超过 90 天）
    let users_to_rotate: Vec<String> = sqlx::query_scalar(
        r#"SELECT DISTINCT user_id FROM mek_versions
           WHERE is_active = 1 AND created_at < ?;"#,
    )
    .bind(threshold)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let mut rotated_count = 0u64;
    let _mgr = MekManager::new();

    for user_id_str in &users_to_rotate {
        // 解析 user_id（数据库中存为 TEXT，需转为 i64）
        let _user_id: i64 = match user_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                tracing::warn!(user_id = %user_id_str, "无法解析 user_id，跳过轮换");
                continue;
            }
        };

        // 注意：真实场景下需要从 key_derivation 获取 KEK
        // 当前调度器无法获取 KEK（KEK 由用户登录后从密码派生）
        // 因此调度器仅记录"需要轮换"的状态，实际轮换由用户下次登录时触发
        tracing::info!(
            user_id = user_id_str,
            "检测到 MEK 需要轮换（超过 90 天），将在用户下次登录时触发"
        );

        // TODO: 当 KEK 持久化方案上线后，可在此处自动触发轮换
        // mgr.rotate_mek(pool, user_id, &kek, RotationReason::Scheduled).await?;

        rotated_count += 1;
    }

    if rotated_count > 0 {
        tracing::info!(
            count = rotated_count,
            "MEK 轮换检查完成，发现 {} 个用户需要轮换",
            rotated_count
        );
    }

    Ok(rotated_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::aes_gcm;
    use crate::crypto::mek_manager::RotationReason;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn setup_test_pool() -> SqlitePool {
        let options = SqliteConnectOptions::new()
            .filename(":memory:")
            .create_if_missing(true)
            .shared_cache(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        sqlx::query(
            r#"CREATE TABLE mek_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                encrypted_mek BLOB NOT NULL,
                nonce BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                rotated_at INTEGER,
                is_active INTEGER NOT NULL DEFAULT 0,
                rotated_from INTEGER,
                UNIQUE(user_id, version)
            );"#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"CREATE TABLE mek_rotation_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                from_version INTEGER,
                to_version INTEGER NOT NULL,
                rotated_at INTEGER NOT NULL,
                trigger_reason TEXT NOT NULL,
                migrated_records INTEGER NOT NULL DEFAULT 0,
                duration_ms INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'completed',
                error_message TEXT
            );"#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    fn make_kek() -> [u8; 32] {
        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = (i as u8).wrapping_mul(7);
        }
        key
    }

    #[tokio::test]
    async fn test_check_and_rotate_no_users() {
        let pool = setup_test_pool().await;
        let count = check_and_rotate(&pool).await.unwrap();
        assert_eq!(count, 0, "无用户时应返回 0");
    }

    #[tokio::test]
    async fn test_check_and_rotate_finds_expired_users() {
        let pool = setup_test_pool().await;
        let kek = make_kek();
        let mut mgr = MekManager::new();

        // 用户 1：刚刚轮换（不应触发）
        mgr.rotate_mek(&pool, 1, &kek, RotationReason::Manual)
            .await
            .unwrap();

        // 用户 2：手动插入 100 天前的版本
        let old_timestamp = chrono::Local::now().timestamp() - 100 * 86400;
        let (encrypted_mek, nonce) = aes_gcm::encrypt_bytes(&[42u8; 32], &kek).unwrap();
        sqlx::query(
            r#"INSERT INTO mek_versions
               (user_id, version, encrypted_mek, nonce, created_at, is_active, rotated_from)
               VALUES ('2', 1, ?, ?, ?, 1, NULL);"#,
        )
        .bind(&encrypted_mek)
        .bind(&nonce[..])
        .bind(old_timestamp)
        .execute(&pool)
        .await
        .unwrap();

        // 执行检查：应发现 1 个用户需要轮换
        let count = check_and_rotate(&pool).await.unwrap();
        assert_eq!(count, 1, "应发现 1 个用户需要轮换");
    }

    #[tokio::test]
    async fn test_check_and_rotate_ignores_non_numeric_user_id() {
        let pool = setup_test_pool().await;
        let kek = make_kek();
        let old_timestamp = chrono::Local::now().timestamp() - 100 * 86400;
        let (encrypted_mek, nonce) = aes_gcm::encrypt_bytes(&[42u8; 32], &kek).unwrap();

        // 插入非数字 user_id（应被跳过，不计入轮换数量）
        sqlx::query(
            r#"INSERT INTO mek_versions
               (user_id, version, encrypted_mek, nonce, created_at, is_active, rotated_from)
               VALUES ('anonymous', 1, ?, ?, ?, 1, NULL);"#,
        )
        .bind(&encrypted_mek)
        .bind(&nonce[..])
        .bind(old_timestamp)
        .execute(&pool)
        .await
        .unwrap();

        let count = check_and_rotate(&pool).await.unwrap();
        assert_eq!(count, 0, "非数字 user_id 应被忽略不计入轮换数量");
    }
}
