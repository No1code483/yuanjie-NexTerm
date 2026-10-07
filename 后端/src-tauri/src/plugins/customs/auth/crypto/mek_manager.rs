//! customs.auth 私有 MEK 密钥管理（阶段3 批次1a-2a：自内核 `src/crypto/` 收编）。
//!
//! 契约依据：`02_核心机制设计/02_后端插件架构设计.md` §七
//! 「`crypto/`（用户密钥）→ auth 插件私有 + kernel 仅留加密基础设施接口」。
//!
//! 边界：
//! - 本文件为 auth 插件私有：MEK 版本管理 / 轮换 / 轮换历史 / 按版本解密的 DB 逻辑集中于此；
//! - 内核仅保留门面（`crate::crypto::mek_manager::MekManager`）与加密原语
//!   （`crate::crypto::aes_gcm` 等），其他模块不直接引用本文件；
//! - 会话密钥缓存由 `AuthMekProvider` 持有，装配期以唯一实例注入
//!   `AppState.mek_manager`；轮换结果直接落回同一实例——原实现每条命令
//!   `MekManager::new()` 自建实例、缓存不入会话（批次1a-2a 已修正）。
//!
//! 设计依据（沿用 v1）：功能展望/平台级增强/05_安全加固_A4.md §2.3

use std::collections::HashMap;
use std::time::Instant;

use sqlx::SqlitePool;

use kernel_api::MekProvider;

use crate::crypto::aes_gcm;
use crate::error::app_error::AppError;

/// MEK 轮换触发原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationReason {
    /// 定时触发（90 天周期）
    Scheduled,
    /// 用户手动触发
    Manual,
    /// 紧急触发（如 KEK 泄露）
    Emergency,
}

impl RotationReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            RotationReason::Scheduled => "scheduled",
            RotationReason::Manual => "manual",
            RotationReason::Emergency => "emergency",
        }
    }
}

/// MEK 轮换结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct MekRotationResult {
    /// 原版本号（None 表示首次创建）
    pub from_version: Option<i64>,
    /// 新版本号
    pub to_version: i64,
    /// 轮换时间戳（Unix 秒）
    pub rotated_at: i64,
    /// 是否已启动密文迁移（true=后台迁移中，false=无需迁移）
    pub migration_started: bool,
    /// 轮换耗时（毫秒）
    pub duration_ms: u64,
}

/// MEK 当前状态查询结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct MekStatus {
    /// 当前版本号（None 表示尚未初始化）
    pub current_version: Option<i64>,
    /// 当前 MEK 创建时间（Unix 秒）
    pub created_at: Option<i64>,
    /// 距上次轮换天数（None 表示无历史轮换）
    pub days_since_creation: Option<i64>,
    /// 是否需要轮换（超过 90 天）
    pub needs_rotation: bool,
}

/// MEK 轮换历史记录
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct MekRotationLog {
    pub id: i64,
    pub user_id: String,
    pub from_version: Option<i64>,
    pub to_version: i64,
    pub rotated_at: i64,
    pub trigger_reason: String,
    pub migrated_records: i64,
    pub duration_ms: i64,
    pub status: String,
    pub error_message: Option<String>,
}

/// MEK 版本记录
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MekVersion {
    pub id: i64,
    pub user_id: String,
    pub version: i64,
    pub encrypted_mek: Vec<u8>,
    pub nonce: Vec<u8>,
    pub created_at: i64,
    pub rotated_at: Option<i64>,
    pub is_active: i64,
    pub rotated_from: Option<i64>,
}

/// MEK 轮换周期配置（90 天）
const MEK_ROTATION_INTERVAL_DAYS: i64 = 90;

// ============================================================================
// 会话密钥缓存（kernel_api::MekProvider 的 auth 侧实现）
// ============================================================================

/// MEK 会话密钥缓存：装配期唯一实例，经内核门面暴露给全模块。
///
/// 串行化由外层 `Arc<RwLock<MekManager>>` 承担，故本结构不含锁。
#[derive(Default)]
pub struct AuthMekProvider {
    /// MEK 内存缓存（user_id -> MEK 明文）
    mek_cache: HashMap<i64, [u8; 32]>,
}

impl AuthMekProvider {
    pub fn new() -> Self {
        Self {
            mek_cache: HashMap::new(),
        }
    }
}

impl MekProvider for AuthMekProvider {
    fn cache_mek(&mut self, user_id: i64, mek: [u8; 32]) {
        self.mek_cache.insert(user_id, mek);
        tracing::info!(user_id, "MEK 已缓存");
    }

    fn get_mek(&self, user_id: i64) -> Option<&[u8; 32]> {
        self.mek_cache.get(&user_id)
    }

    fn clear_mek(&mut self, user_id: i64) {
        self.mek_cache.remove(&user_id);
        tracing::info!(user_id, "MEK 已清除");
    }

    fn clear_all(&mut self) {
        self.mek_cache.clear();
        tracing::info!("所有 MEK 已清除");
    }

    fn re_encrypt_mek(
        &self,
        mek: &[u8; 32],
        new_kek: &[u8; 32],
    ) -> Result<(Vec<u8>, [u8; 12]), String> {
        aes_gcm::encrypt_bytes(mek, new_kek).map_err(|e| e.to_string())
    }
}

// ============================================================================
// T2.13 版本管理 + 轮换（auth 私有）
// ============================================================================

/// 获取用户当前 MEK 版本号（无活跃版本时返回 None）。
pub async fn current_version(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Option<i64>, AppError> {
    let version: Option<(i64,)> = sqlx::query_as(
        "SELECT version FROM mek_versions WHERE user_id = ? AND is_active = 1 LIMIT 1;",
    )
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(version.map(|(v,)| v))
}

/// 获取用户 MEK 状态（含是否需要轮换）
pub async fn get_status(pool: &SqlitePool, user_id: i64) -> Result<MekStatus, AppError> {
    let row: Option<(i64, i64)> = sqlx::query_as(
        "SELECT version, created_at FROM mek_versions WHERE user_id = ? AND is_active = 1 LIMIT 1;",
    )
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    match row {
        Some((version, created_at)) => {
            let now = chrono::Local::now().timestamp();
            let days_since = (now - created_at) / 86400;
            Ok(MekStatus {
                current_version: Some(version),
                created_at: Some(created_at),
                days_since_creation: Some(days_since),
                needs_rotation: days_since >= MEK_ROTATION_INTERVAL_DAYS,
            })
        }
        None => Ok(MekStatus {
            current_version: None,
            created_at: None,
            days_since_creation: None,
            needs_rotation: false, // 无版本时不强制轮换
        }),
    }
}

/// 触发 MEK 轮换，返回 `(轮换结果, 新 MEK 明文)`。
///
/// 流程：
/// 1. 获取当前版本号（无则从 0 开始）
/// 2. 生成新 MEK（32 字节随机数）
/// 3. 用 KEK 加密新 MEK，存入 mek_versions 表
/// 4. 旧版本标记为 is_active=0, rotated_at=now
/// 5. 记录轮换日志
///
/// **新 MEK 不写回内存缓存**：由调用方（auth dispatcher）经内核门面写入
/// `AppState.mek_manager`，确保会话缓存与库中活跃版本一致。
///
/// 注意：本函数不执行密文迁移（密文迁移由 `migrate_ciphertexts` 异步执行）。
pub async fn rotate_mek(
    pool: &SqlitePool,
    user_id: i64,
    kek: &[u8; 32],
    reason: RotationReason,
) -> Result<(MekRotationResult, [u8; 32]), AppError> {
    let start = Instant::now();
    let now = chrono::Local::now().timestamp();
    let user_id_str = user_id.to_string();

    // 1. 获取当前版本号
    let from_version: Option<i64> = current_version(pool, user_id).await?;
    let to_version = from_version.unwrap_or(0) + 1;

    // 2. 生成新 MEK（32 字节随机数）
    let mut new_mek = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut new_mek);

    // 3. 用 KEK 加密新 MEK
    let (encrypted_mek, nonce) = aes_gcm::encrypt_bytes(&new_mek, kek)?;

    // 4. 旧版本标记为非活跃（如有）
    if from_version.is_some() {
        sqlx::query(
            "UPDATE mek_versions SET is_active = 0, rotated_at = ? WHERE user_id = ? AND is_active = 1;",
        )
        .bind(now)
        .bind(&user_id_str)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    }

    // 5. 插入新版本（is_active = 1）
    sqlx::query(
        r#"INSERT INTO mek_versions
           (user_id, version, encrypted_mek, nonce, created_at, rotated_at, is_active, rotated_from)
           VALUES (?, ?, ?, ?, ?, NULL, 1, ?);"#,
    )
    .bind(&user_id_str)
    .bind(to_version)
    .bind(&encrypted_mek)
    .bind(&nonce[..])
    .bind(now)
    .bind(from_version)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    // 6. 记录轮换日志
    let duration_ms = start.elapsed().as_millis() as i64;
    sqlx::query(
        r#"INSERT INTO mek_rotation_log
           (user_id, from_version, to_version, rotated_at, trigger_reason, migrated_records, duration_ms, status)
           VALUES (?, ?, ?, ?, ?, 0, ?, 'completed');"#,
    )
    .bind(&user_id_str)
    .bind(from_version)
    .bind(to_version)
    .bind(now)
    .bind(reason.as_str())
    .bind(duration_ms)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    tracing::info!(
        user_id, from_version, to_version, reason = reason.as_str(),
        duration_ms, "MEK 轮换完成"
    );

    Ok((
        MekRotationResult {
            from_version,
            to_version,
            rotated_at: now,
            migration_started: false, // 轮换本身不启动迁移，由调用方决定
            duration_ms: duration_ms as u64,
        },
        new_mek,
    ))
}

/// 查询轮换历史
pub async fn get_rotation_history(
    pool: &SqlitePool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<MekRotationLog>, AppError> {
    let logs: Vec<MekRotationLog> = sqlx::query_as(
        r#"SELECT id, user_id, from_version, to_version, rotated_at,
                  trigger_reason, migrated_records, duration_ms, status, error_message
           FROM mek_rotation_log
           WHERE user_id = ?
           ORDER BY rotated_at DESC, id DESC
           LIMIT ?;"#,
    )
    .bind(user_id.to_string())
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(logs)
}

/// 获取指定版本的 MEK 明文（用 KEK 解密）
///
/// 用于：
/// - 解密历史密文（旧版本密文迁移前的过渡期）
/// - 验证旧版本完整性
pub async fn get_mek_for_version(
    pool: &SqlitePool,
    user_id: i64,
    version: i64,
    kek: &[u8; 32],
) -> Result<[u8; 32], AppError> {
    let row: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as(
        "SELECT encrypted_mek, nonce FROM mek_versions WHERE user_id = ? AND version = ? LIMIT 1;",
    )
    .bind(user_id.to_string())
    .bind(version)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    match row {
        Some((encrypted_mek, nonce)) => {
            if nonce.len() != 12 {
                return Err(AppError::Crypto("nonce 长度错误".into()));
            }
            let nonce_arr: [u8; 12] = nonce
                .try_into()
                .map_err(|_| AppError::Crypto("nonce 转换失败".into()))?;
            let plaintext = aes_gcm::decrypt_bytes(&encrypted_mek, kek, &nonce_arr)?;
            let mek: [u8; 32] = plaintext
                .as_slice()
                .try_into()
                .map_err(|_| AppError::Crypto("MEK 长度错误".into()))?;
            Ok(mek)
        }
        None => Err(AppError::NotFound),
    }
}

// 引入 rand::RngCore trait（用于 fill_bytes）
use rand::RngCore;

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    /// 创建内存数据库 + mek_versions + mek_rotation_log 表的测试连接池
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
    async fn test_get_current_version_no_versions() {
        let pool = setup_test_pool().await;
        let version = current_version(&pool, 1).await.unwrap();
        assert_eq!(version, None);
    }

    #[tokio::test]
    async fn test_get_status_no_versions() {
        let pool = setup_test_pool().await;
        let status = get_status(&pool, 1).await.unwrap();
        assert_eq!(status.current_version, None);
        assert!(!status.needs_rotation);
    }

    #[tokio::test]
    async fn test_rotate_mek_first_time() {
        let pool = setup_test_pool().await;
        let kek = make_kek();

        let (result, new_mek) = rotate_mek(&pool, 1, &kek, RotationReason::Manual)
            .await
            .unwrap();

        assert_eq!(result.from_version, None, "首次轮换 from_version 应为 None");
        assert_eq!(result.to_version, 1, "首次轮换 to_version 应为 1");
        assert_eq!(new_mek.len(), 32, "轮换必须产出 32 字节新 MEK");
    }

    #[tokio::test]
    async fn test_rotate_mek_scheduled() {
        let pool = setup_test_pool().await;
        let kek = make_kek();

        // 首次轮换
        let (r1, _) = rotate_mek(&pool, 1, &kek, RotationReason::Scheduled)
            .await
            .unwrap();
        assert_eq!(r1.to_version, 1);

        // 第二次轮换
        let (r2, _) = rotate_mek(&pool, 1, &kek, RotationReason::Scheduled)
            .await
            .unwrap();
        assert_eq!(r2.from_version, Some(1));
        assert_eq!(r2.to_version, 2);

        // 验证版本号递增
        let current = current_version(&pool, 1).await.unwrap();
        assert_eq!(current, Some(2));
    }

    #[tokio::test]
    async fn test_rotate_mek_emergency() {
        let pool = setup_test_pool().await;
        let kek = make_kek();

        let (result, _) = rotate_mek(&pool, 1, &kek, RotationReason::Emergency)
            .await
            .unwrap();
        assert_eq!(result.to_version, 1);
    }

    #[tokio::test]
    async fn test_get_mek_for_version() {
        let pool = setup_test_pool().await;
        let kek = make_kek();

        let (r1, new_mek) = rotate_mek(&pool, 1, &kek, RotationReason::Manual)
            .await
            .unwrap();
        assert_eq!(r1.to_version, 1);

        // 用 KEK 解密版本 1 的 MEK
        let mek_v1 = get_mek_for_version(&pool, 1, 1, &kek).await.unwrap();
        assert_eq!(mek_v1.len(), 32);

        // 验证：解密出的 MEK 应与轮换返回的新 MEK 一致
        assert_eq!(&mek_v1[..], &new_mek[..]);
    }

    #[tokio::test]
    async fn test_get_mek_for_version_wrong_kek_fails() {
        let pool = setup_test_pool().await;
        let kek = make_kek();
        let mut wrong_kek = make_kek();
        wrong_kek[0] = wrong_kek[0].wrapping_add(1);

        rotate_mek(&pool, 1, &kek, RotationReason::Manual)
            .await
            .unwrap();

        // 用错误 KEK 解密应失败
        let result = get_mek_for_version(&pool, 1, 1, &wrong_kek).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_mek_for_nonexistent_version_fails() {
        let pool = setup_test_pool().await;
        let kek = make_kek();

        let result = get_mek_for_version(&pool, 1, 999, &kek).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_rotation_history() {
        let pool = setup_test_pool().await;
        let kek = make_kek();

        // 轮换 3 次
        for _ in 0..3 {
            rotate_mek(&pool, 1, &kek, RotationReason::Scheduled)
                .await
                .unwrap();
        }

        let history = get_rotation_history(&pool, 1, 10).await.unwrap();
        assert_eq!(history.len(), 3);

        // 验证：最新轮换在第一位（按 rotated_at DESC 排序）
        assert_eq!(history[0].to_version, 3);
        assert_eq!(history[1].to_version, 2);
        assert_eq!(history[2].to_version, 1);

        // 验证：from_version 链正确
        assert_eq!(history[0].from_version, Some(2));
        assert_eq!(history[1].from_version, Some(1));
        assert_eq!(history[2].from_version, None);

        // 验证：trigger_reason 正确
        for log in &history {
            assert_eq!(log.trigger_reason, "scheduled");
            assert_eq!(log.status, "completed");
        }
    }

    #[tokio::test]
    async fn test_get_rotation_history_empty() {
        let pool = setup_test_pool().await;
        let history = get_rotation_history(&pool, 1, 10).await.unwrap();
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_get_status_after_rotation() {
        let pool = setup_test_pool().await;
        let kek = make_kek();

        rotate_mek(&pool, 1, &kek, RotationReason::Manual)
            .await
            .unwrap();

        let status = get_status(&pool, 1).await.unwrap();
        assert_eq!(status.current_version, Some(1));
        assert!(status.created_at.is_some());
        // 刚创建，距上次轮换天数应为 0
        assert_eq!(status.days_since_creation, Some(0));
        // 0 < 90，不需要轮换
        assert!(!status.needs_rotation);
    }

    #[tokio::test]
    async fn test_get_status_needs_rotation() {
        let pool = setup_test_pool().await;
        let kek = make_kek();

        // 手动插入一个 100 天前的版本
        let old_timestamp = chrono::Local::now().timestamp() - 100 * 86400;
        let (encrypted_mek, nonce) = aes_gcm::encrypt_bytes(&[42u8; 32], &kek).unwrap();
        sqlx::query(
            r#"INSERT INTO mek_versions
               (user_id, version, encrypted_mek, nonce, created_at, is_active, rotated_from)
               VALUES ('1', 1, ?, ?, ?, 1, NULL);"#,
        )
        .bind(&encrypted_mek)
        .bind(&nonce[..])
        .bind(old_timestamp)
        .execute(&pool)
        .await
        .unwrap();

        let status = get_status(&pool, 1).await.unwrap();
        assert_eq!(status.current_version, Some(1));
        assert_eq!(status.days_since_creation, Some(100));
        assert!(status.needs_rotation, "100 天后应触发轮换");
    }

    #[test]
    fn test_cache_and_get_mek() {
        let mut mgr = AuthMekProvider::new();
        let mek = [123u8; 32];
        mgr.cache_mek(1, mek);
        assert_eq!(mgr.get_mek(1), Some(&mek));
        assert_eq!(mgr.get_mek(999), None);
    }

    #[test]
    fn test_clear_mek() {
        let mut mgr = AuthMekProvider::new();
        mgr.cache_mek(1, [1u8; 32]);
        mgr.clear_mek(1);
        assert_eq!(mgr.get_mek(1), None);
    }

    #[test]
    fn test_clear_all() {
        let mut mgr = AuthMekProvider::new();
        mgr.cache_mek(1, [1u8; 32]);
        mgr.cache_mek(2, [2u8; 32]);
        mgr.clear_all();
        assert_eq!(mgr.get_mek(1), None);
        assert_eq!(mgr.get_mek(2), None);
    }

    #[test]
    fn test_re_encrypt_mek() {
        let mgr = AuthMekProvider::new();
        let mek = [42u8; 32];
        let new_kek = [99u8; 32];
        let (ciphertext, nonce) = mgr.re_encrypt_mek(&mek, &new_kek).unwrap();
        let decrypted =
            crate::crypto::aes_gcm::decrypt_bytes(&ciphertext, &new_kek, &nonce).unwrap();
        assert_eq!(&decrypted[..], &mek[..]);
    }

    #[test]
    fn test_rotation_reason_as_str() {
        assert_eq!(RotationReason::Scheduled.as_str(), "scheduled");
        assert_eq!(RotationReason::Manual.as_str(), "manual");
        assert_eq!(RotationReason::Emergency.as_str(), "emergency");
    }
}
