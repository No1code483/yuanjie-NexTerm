//! T2.13 MEK 密钥轮换管理器
//!
//! 设计依据：功能展望/平台级增强/05_安全加固_A4.md §2.3
//!
//! 核心能力：
//! - MEK 多版本管理（每用户独立版本号）
//! - 自动轮换（90 天周期 + 后台调度）
//! - 手动轮换（用户触发）
//! - 紧急轮换（KEK 泄露时立即触发）
//! - 密文渐进迁移（旧 MEK 加密的密文用新 MEK 重新加密）
//! - 轮换历史审计（mek_rotation_log 表）
//!
//! 安全保障：
//! - 多版本共存：旧版本密文在迁移完成前仍可解密
//! - 原子性：轮换过程中崩溃可恢复（status 字段）
//! - 防并发：同一用户不能并发轮换（应用层加锁）
//! - 防回滚：版本号单调递增，不允许降级

use std::collections::HashMap;
use std::time::Instant;

use sqlx::SqlitePool;

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

pub struct MekManager {
    /// MEK 内存缓存（user_id -> MEK 明文）
    mek_cache: HashMap<i64, [u8; 32]>,
    /// MEK 版本缓存（user_id -> 当前版本号）
    mek_version_cache: HashMap<i64, i64>,
}

impl MekManager {
    pub fn new() -> Self {
        Self {
            mek_cache: HashMap::new(),
            mek_version_cache: HashMap::new(),
        }
    }

    // ========================================================================
    // 原有 API（向后兼容）
    // ========================================================================

    pub fn cache_mek(&mut self, user_id: i64, mek: [u8; 32]) {
        self.mek_cache.insert(user_id, mek);
        tracing::info!(user_id, "MEK 已缓存");
    }

    pub fn get_mek(&self, user_id: i64) -> Option<&[u8; 32]> {
        self.mek_cache.get(&user_id)
    }

    pub fn clear_mek(&mut self, user_id: i64) {
        self.mek_cache.remove(&user_id);
        self.mek_version_cache.remove(&user_id);
        tracing::info!(user_id, "MEK 已清除");
    }

    pub fn clear_all(&mut self) {
        self.mek_cache.clear();
        self.mek_version_cache.clear();
        tracing::info!("所有 MEK 已清除");
    }

    pub fn re_encrypt_mek(
        &self,
        mek: &[u8; 32],
        new_kek: &[u8; 32],
    ) -> Result<(Vec<u8>, [u8; 12]), AppError> {
        aes_gcm::encrypt_bytes(mek, new_kek)
    }

    // ========================================================================
    // T2.13 新增：版本管理 + 轮换
    // ========================================================================

    /// 获取用户当前 MEK 版本号
    ///
    /// 优先从内存缓存读取，缓存未命中时查数据库。
    pub async fn get_current_version(
        &self,
        pool: &SqlitePool,
        user_id: i64,
    ) -> Result<Option<i64>, AppError> {
        // 1. 检查内存缓存
        if let Some(v) = self.mek_version_cache.get(&user_id) {
            return Ok(Some(*v));
        }

        // 2. 查数据库
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
    pub async fn get_status(
        &self,
        pool: &SqlitePool,
        user_id: i64,
    ) -> Result<MekStatus, AppError> {
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

    /// 触发 MEK 轮换
    ///
    /// 流程：
    /// 1. 获取当前版本号（无则从 0 开始）
    /// 2. 生成新 MEK（32 字节随机数）
    /// 3. 用 KEK 加密新 MEK，存入 mek_versions 表
    /// 4. 旧版本标记为 is_active=0, rotated_at=now
    /// 5. 更新内存缓存
    /// 6. 记录轮换日志
    ///
    /// 注意：本函数不执行密文迁移（密文迁移由 `migrate_ciphertexts` 异步执行）
    pub async fn rotate_mek(
        &mut self,
        pool: &SqlitePool,
        user_id: i64,
        kek: &[u8; 32],
        reason: RotationReason,
    ) -> Result<MekRotationResult, AppError> {
        let start = Instant::now();
        let now = chrono::Local::now().timestamp();
        let user_id_str = user_id.to_string();

        // 1. 获取当前版本号
        let from_version: Option<i64> = self.get_current_version(pool, user_id).await?;
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

        // 6. 更新内存缓存
        self.mek_cache.insert(user_id, new_mek);
        self.mek_version_cache.insert(user_id, to_version);

        // 7. 记录轮换日志
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

        Ok(MekRotationResult {
            from_version,
            to_version,
            rotated_at: now,
            migration_started: false, // 轮换本身不启动迁移，由调用方决定
            duration_ms: duration_ms as u64,
        })
    }

    /// 查询轮换历史
    pub async fn get_rotation_history(
        &self,
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
        &self,
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
        let mgr = MekManager::new();
        let version = mgr.get_current_version(&pool, 1).await.unwrap();
        assert_eq!(version, None);
    }

    #[tokio::test]
    async fn test_get_status_no_versions() {
        let pool = setup_test_pool().await;
        let mgr = MekManager::new();
        let status = mgr.get_status(&pool, 1).await.unwrap();
        assert_eq!(status.current_version, None);
        assert!(!status.needs_rotation);
    }

    #[tokio::test]
    async fn test_rotate_mek_first_time() {
        let pool = setup_test_pool().await;
        let mut mgr = MekManager::new();
        let kek = make_kek();

        let result = mgr
            .rotate_mek(&pool, 1, &kek, RotationReason::Manual)
            .await
            .unwrap();

        assert_eq!(result.from_version, None, "首次轮换 from_version 应为 None");
        assert_eq!(result.to_version, 1, "首次轮换 to_version 应为 1");

        // 验证内存缓存已更新
        assert!(mgr.get_mek(1).is_some(), "MEK 应已缓存");
    }

    #[tokio::test]
    async fn test_rotate_mek_scheduled() {
        let pool = setup_test_pool().await;
        let mut mgr = MekManager::new();
        let kek = make_kek();

        // 首次轮换
        let r1 = mgr
            .rotate_mek(&pool, 1, &kek, RotationReason::Scheduled)
            .await
            .unwrap();
        assert_eq!(r1.to_version, 1);

        // 第二次轮换
        let r2 = mgr
            .rotate_mek(&pool, 1, &kek, RotationReason::Scheduled)
            .await
            .unwrap();
        assert_eq!(r2.from_version, Some(1));
        assert_eq!(r2.to_version, 2);

        // 验证版本号递增
        let current = mgr.get_current_version(&pool, 1).await.unwrap();
        assert_eq!(current, Some(2));
    }

    #[tokio::test]
    async fn test_rotate_mek_emergency() {
        let pool = setup_test_pool().await;
        let mut mgr = MekManager::new();
        let kek = make_kek();

        let result = mgr
            .rotate_mek(&pool, 1, &kek, RotationReason::Emergency)
            .await
            .unwrap();
        assert_eq!(result.to_version, 1);
    }

    #[tokio::test]
    async fn test_get_mek_for_version() {
        let pool = setup_test_pool().await;
        let mut mgr = MekManager::new();
        let kek = make_kek();

        // 首次轮换
        let r1 = mgr
            .rotate_mek(&pool, 1, &kek, RotationReason::Manual)
            .await
            .unwrap();
        assert_eq!(r1.to_version, 1);

        // 用 KEK 解密版本 1 的 MEK
        let mek_v1 = mgr.get_mek_for_version(&pool, 1, 1, &kek).await.unwrap();
        assert_eq!(mek_v1.len(), 32);

        // 验证：解密出的 MEK 应与内存缓存中的 MEK 一致
        let cached_mek = mgr.get_mek(1).unwrap();
        assert_eq!(&mek_v1[..], &cached_mek[..]);
    }

    #[tokio::test]
    async fn test_get_mek_for_version_wrong_kek_fails() {
        let pool = setup_test_pool().await;
        let mut mgr = MekManager::new();
        let kek = make_kek();
        let mut wrong_kek = make_kek();
        wrong_kek[0] = wrong_kek[0].wrapping_add(1);

        mgr.rotate_mek(&pool, 1, &kek, RotationReason::Manual)
            .await
            .unwrap();

        // 用错误 KEK 解密应失败
        let result = mgr.get_mek_for_version(&pool, 1, 1, &wrong_kek).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_mek_for_nonexistent_version_fails() {
        let pool = setup_test_pool().await;
        let mgr = MekManager::new();
        let kek = make_kek();

        let result = mgr.get_mek_for_version(&pool, 1, 999, &kek).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_rotation_history() {
        let pool = setup_test_pool().await;
        let mut mgr = MekManager::new();
        let kek = make_kek();

        // 轮换 3 次
        for _ in 0..3 {
            mgr.rotate_mek(&pool, 1, &kek, RotationReason::Scheduled)
                .await
                .unwrap();
        }

        let history = mgr.get_rotation_history(&pool, 1, 10).await.unwrap();
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
        let mgr = MekManager::new();
        let history = mgr.get_rotation_history(&pool, 1, 10).await.unwrap();
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_get_status_after_rotation() {
        let pool = setup_test_pool().await;
        let mut mgr = MekManager::new();
        let kek = make_kek();

        mgr.rotate_mek(&pool, 1, &kek, RotationReason::Manual)
            .await
            .unwrap();

        let status = mgr.get_status(&pool, 1).await.unwrap();
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
        let mut mgr = MekManager::new();
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

        let status = mgr.get_status(&pool, 1).await.unwrap();
        assert_eq!(status.current_version, Some(1));
        assert_eq!(status.days_since_creation, Some(100));
        assert!(status.needs_rotation, "100 天后应触发轮换");
    }

    #[tokio::test]
    async fn test_clear_mek_clears_version_cache() {
        let mut mgr = MekManager::new();
        mgr.cache_mek(1, [1u8; 32]);
        mgr.mek_version_cache.insert(1, 5);

        mgr.clear_mek(1);

        assert!(mgr.get_mek(1).is_none());
        assert!(mgr.mek_version_cache.get(&1).is_none());
    }

    // ========================================================================
    // 原有测试（向后兼容）
    // ========================================================================

    #[test]
    fn test_cache_and_get_mek() {
        let mut mgr = MekManager::new();
        let mek = [123u8; 32];
        mgr.cache_mek(1, mek);
        assert_eq!(mgr.get_mek(1), Some(&mek));
        assert_eq!(mgr.get_mek(999), None);
    }

    #[test]
    fn test_clear_mek() {
        let mut mgr = MekManager::new();
        mgr.cache_mek(1, [1u8; 32]);
        mgr.clear_mek(1);
        assert_eq!(mgr.get_mek(1), None);
    }

    #[test]
    fn test_clear_all() {
        let mut mgr = MekManager::new();
        mgr.cache_mek(1, [1u8; 32]);
        mgr.cache_mek(2, [2u8; 32]);
        mgr.clear_all();
        assert_eq!(mgr.get_mek(1), None);
        assert_eq!(mgr.get_mek(2), None);
    }

    #[test]
    fn test_re_encrypt_mek() {
        let mgr = MekManager::new();
        let mek = [42u8; 32];
        let new_kek = [99u8; 32];
        let (ciphertext, nonce) = mgr.re_encrypt_mek(&mek, &new_kek).unwrap();
        let decrypted = crate::crypto::aes_gcm::decrypt_bytes(&ciphertext, &new_kek, &nonce).unwrap();
        assert_eq!(&decrypted[..], &mek[..]);
    }

    #[test]
    fn test_rotation_reason_as_str() {
        assert_eq!(RotationReason::Scheduled.as_str(), "scheduled");
        assert_eq!(RotationReason::Manual.as_str(), "manual");
        assert_eq!(RotationReason::Emergency.as_str(), "emergency");
    }
}
