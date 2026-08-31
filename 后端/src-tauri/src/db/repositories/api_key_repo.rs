//! Yuan Code v3.1 Task 3.5.1 — api_keys 表仓储层
//!
//! 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.5.1
//! 加密：复用 crypto::aes_gcm（密文以 BLOB 存储，与 ai_repo 模式一致）
//!
//! 安全审计修复（多用户隔离批次 1）：所有方法添加 user_id 参数 + WHERE user_id = ? 过滤。
//! 原实现：任何登录用户可读取/修改/删除他人的 API Key 配置。
//! 现实现：所有查询/更新/删除操作强制按 user_id 过滤。
//! 规范：安全审计报告/2026-07-25-代码安全审计报告.md 附录七

use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::api_key::{ApiKey, ApiKeyResponse};

/// 列出当前用户的所有 API Key（响应形态，不暴露密文）
pub async fn list_all(pool: &SqlitePool, user_id: i64) -> Result<Vec<ApiKeyResponse>, AppError> {
    let rows: Vec<ApiKey> = sqlx::query_as::<_, ApiKey>(
        "SELECT * FROM api_keys WHERE user_id = ? ORDER BY provider ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(rows.into_iter().map(|r| r.to_response()).collect())
}

/// 按 provider 查询当前用户的完整记录（含密文，仅服务内部 cloud_api_router 使用）
pub async fn get_by_provider(
    pool: &SqlitePool,
    user_id: i64,
    provider: &str,
) -> Result<Option<ApiKey>, AppError> {
    sqlx::query_as::<_, ApiKey>("SELECT * FROM api_keys WHERE provider = ? AND user_id = ?")
        .bind(provider)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

/// 按 id 查询当前用户的完整记录（含密文）
pub async fn get_by_id(pool: &SqlitePool, id: i64, user_id: i64) -> Result<Option<ApiKey>, AppError> {
    sqlx::query_as::<_, ApiKey>("SELECT * FROM api_keys WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

/// 新增或更新（按 provider UPSERT）
///
/// 加密由调用方（cloud_api_router）完成，本函数仅接收已加密的 (enc, nonce) 落盘。
///
/// 安全说明：UPSERT 冲突目标为 provider（全局 UNIQUE）。
/// 若其他用户已配置同 provider 的 Key，本次 UPSERT 会更新他人的记录。
/// 应用层（service）应在调用前检查 provider 是否已被其他用户占用，返回明确错误。
pub async fn upsert(
    pool: &SqlitePool,
    user_id: i64,
    provider: &str,
    display_name: Option<&str>,
    api_key_enc: &[u8],
    api_key_nonce: &[u8],
    api_url: Option<&str>,
    is_enabled: bool,
    now: i64,
) -> Result<ApiKey, AppError> {
    // SQLite UPSERT（ON CONFLICT(provider) DO UPDATE）
    // 注意：provider 全局 UNIQUE（migration 0109），跨用户冲突时更新原记录的 user_id 为新所有者
    let row = sqlx::query_as::<_, ApiKey>(
        "INSERT INTO api_keys (user_id, provider, display_name, api_key_enc, api_key_nonce, api_url, is_cloud_only, is_enabled, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?, ?)
         ON CONFLICT(provider) DO UPDATE SET
            user_id = excluded.user_id,
            display_name = COALESCE(excluded.display_name, api_keys.display_name),
            api_key_enc = excluded.api_key_enc,
            api_key_nonce = excluded.api_key_nonce,
            api_url = COALESCE(excluded.api_url, api_keys.api_url),
            is_enabled = excluded.is_enabled,
            updated_at = excluded.updated_at
         RETURNING *",
    )
    .bind(user_id)
    .bind(provider)
    .bind(display_name)
    .bind(api_key_enc)
    .bind(api_key_nonce)
    .bind(api_url)
    .bind(is_enabled)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(row)
}

/// 仅更新启用状态（按 id + user_id 过滤）
pub async fn set_enabled(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
    is_enabled: bool,
    now: i64,
) -> Result<(), AppError> {
    sqlx::query("UPDATE api_keys SET is_enabled = ?, updated_at = ? WHERE id = ? AND user_id = ?")
        .bind(is_enabled)
        .bind(now)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

/// 更新 last_used_at（路由调用后刷新）
///
/// 安全说明：按 provider + user_id 过滤，确保只更新当前用户的 Key 使用时间。
pub async fn touch_last_used(
    pool: &SqlitePool,
    user_id: i64,
    provider: &str,
    now: i64,
) -> Result<(), AppError> {
    sqlx::query("UPDATE api_keys SET last_used_at = ? WHERE provider = ? AND user_id = ?")
        .bind(now)
        .bind(provider)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

/// 删除（按 id + user_id 过滤）
pub async fn delete(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM api_keys WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! api_key_repo 单元测试（v1.52 测试体系 Phase 2）
    //!
    //! 安全审计修复（多用户隔离批次 1）：测试用例已更新为带 user_id 参数。
    use super::*;

    #[tokio::test]
    async fn test_upsert_and_get_by_provider() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        // 建表
        sqlx::query(include_str!("../../../migrations/0109_create_api_keys_table/up.sql"))
            .execute(&pool)
            .await
            .unwrap();
        // v116 迁移：添加 user_id 字段
        sqlx::query("ALTER TABLE api_keys ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;")
            .execute(&pool)
            .await
            .unwrap();

        // 新增
        let row = upsert(
            &pool,
            1,
            "openai",
            Some("OpenAI 主账号"),
            b"encrypted-key-blob",
            b"nonce-12-byte",
            Some("https://api.openai.com/v1"),
            true,
            1000,
        )
        .await
        .unwrap();
        assert_eq!(row.provider, "openai");
        assert!(row.is_cloud_only);
        assert!(row.is_enabled);
        assert_eq!(row.user_id, 1);

        // 查询
        let fetched = get_by_provider(&pool, 1, "openai").await.unwrap().unwrap();
        assert_eq!(fetched.id, row.id);
        assert_eq!(fetched.api_key_enc, b"encrypted-key-blob");

        // UPSERT 更新（同 provider + 同 user_id）
        let updated = upsert(
            &pool,
            1,
            "openai",
            None,
            b"new-encrypted-key",
            b"new-nonce-12b",
            None,
            false,
            2000,
        )
        .await
        .unwrap();
        assert_eq!(updated.id, row.id); // 同一条记录
        assert_eq!(updated.api_key_enc, b"new-encrypted-key");
        assert!(!updated.is_enabled);
        assert_eq!(updated.updated_at, 2000);
    }

    #[tokio::test]
    async fn test_list_all_returns_response_form() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(include_str!("../../../migrations/0109_create_api_keys_table/up.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("ALTER TABLE api_keys ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;")
            .execute(&pool)
            .await
            .unwrap();

        upsert(&pool, 1, "openai", None, b"k1", b"n1", None, true, 1000)
            .await
            .unwrap();
        upsert(&pool, 1, "anthropic", None, b"k2", b"n2", None, true, 1000)
            .await
            .unwrap();

        let list = list_all(&pool, 1).await.unwrap();
        assert_eq!(list.len(), 2);
        // 响应不暴露密文字段
        let resp_json = serde_json::to_string(&list[0]).unwrap();
        assert!(!resp_json.contains("api_key_enc"));
        assert!(resp_json.contains("has_api_key"));
    }

    #[tokio::test]
    async fn test_touch_last_used_and_delete() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(include_str!("../../../migrations/0109_create_api_keys_table/up.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("ALTER TABLE api_keys ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;")
            .execute(&pool)
            .await
            .unwrap();

        let row = upsert(&pool, 1, "openai", None, b"k", b"n", None, true, 1000)
            .await
            .unwrap();
        touch_last_used(&pool, 1, "openai", 5000).await.unwrap();
        let fetched = get_by_id(&pool, row.id, 1).await.unwrap().unwrap();
        assert_eq!(fetched.last_used_at, Some(5000));

        delete(&pool, row.id, 1).await.unwrap();
        assert!(get_by_id(&pool, row.id, 1).await.unwrap().is_none());
    }

    /// 安全审计修复（多用户隔离批次 1）：验证跨用户隔离
    #[tokio::test]
    async fn test_user_isolation() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(include_str!("../../../migrations/0109_create_api_keys_table/up.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("ALTER TABLE api_keys ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;")
            .execute(&pool)
            .await
            .unwrap();

        // user_id=1 配置 openai key
        upsert(&pool, 1, "openai", None, b"admin-key", b"n1", None, true, 1000)
            .await
            .unwrap();

        // user_id=2 查询 openai key 应返回 None（隔离生效）
        let cross_user = get_by_provider(&pool, 2, "openai").await.unwrap();
        assert!(cross_user.is_none(), "user_id=2 不应看到 user_id=1 的 key");

        // user_id=2 列出所有 key 应为空
        let list = list_all(&pool, 2).await.unwrap();
        assert!(list.is_empty(), "user_id=2 不应看到任何 key");

        // user_id=2 尝试删除 user_id=1 的 key 应无效（rows_affected=0）
        let admin_key = get_by_id(&pool, 1, 1).await.unwrap().unwrap();
        delete(&pool, admin_key.id, 2).await.unwrap();
        // user_id=1 的 key 仍存在
        assert!(get_by_id(&pool, admin_key.id, 1).await.unwrap().is_some());
    }
}
