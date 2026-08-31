//! Yuan Code v3.2 Task 3.4.1 — model_routing_rules 表仓储层
//!
//! 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.4.1
//!       + 项目核心设计意图 §三 / §八（强制规则 8.1.1）
//!
//! 与 api_key_repo 的协作：
//! - 本仓储：按 task_type 存取路由规则（provider + model_name）
//! - api_key_repo：按 provider 取云端 API Key 密文
//! - service 层串联：task_type → rule → provider → api_key → call cloud API

use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::model_routing::ModelRoutingRule;

/// 列出所有路由规则
pub async fn list_all(pool: &SqlitePool, user_id: i64) -> Result<Vec<ModelRoutingRule>, AppError> {
    let rows = sqlx::query_as::<_, ModelRoutingRule>(
        "SELECT * FROM model_routing_rules WHERE user_id = ? ORDER BY priority DESC, task_type ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(rows)
}

/// 按 task_type 查询规则（含未启用规则，由 service 层判断是否回退到默认）
pub async fn get_by_task_type(
    pool: &SqlitePool,
    user_id: i64,
    task_type: &str,
) -> Result<Option<ModelRoutingRule>, AppError> {
    let row = sqlx::query_as::<_, ModelRoutingRule>(
        "SELECT * FROM model_routing_rules WHERE task_type = ? AND user_id = ? LIMIT 1",
    )
    .bind(task_type)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(row)
}

/// 新增或更新路由规则（按 task_type UPSERT）
///
/// 注意：provider 校验由 service 层调用 model_routing::UpsertRoutingRuleRequest::validate 完成，
/// 此函数仅负责落盘（信任调用方已校验）。
pub async fn upsert(
    pool: &SqlitePool,
    user_id: i64,
    task_type: &str,
    provider: &str,
    model_name: &str,
    temperature: Option<f64>,
    max_tokens: Option<i32>,
    is_enabled: bool,
    now: i64,
) -> Result<ModelRoutingRule, AppError> {
    let row = sqlx::query_as::<_, ModelRoutingRule>(
        "INSERT INTO model_routing_rules (user_id, task_type, provider, model_name, temperature, max_tokens, is_enabled, is_cloud_only, priority, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, 1, 100, ?, ?)
         ON CONFLICT(user_id, task_type) DO UPDATE SET
            provider = excluded.provider,
            model_name = excluded.model_name,
            temperature = excluded.temperature,
            max_tokens = excluded.max_tokens,
            is_enabled = excluded.is_enabled,
            updated_at = excluded.updated_at
         RETURNING *",
    )
    .bind(user_id)
    .bind(task_type)
    .bind(provider)
    .bind(model_name)
    .bind(temperature)
    .bind(max_tokens)
    .bind(is_enabled)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(row)
}

/// 启用/禁用规则
pub async fn set_enabled(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    is_enabled: bool,
    now: i64,
) -> Result<(), AppError> {
    sqlx::query("UPDATE model_routing_rules SET is_enabled = ?, updated_at = ? WHERE id = ? AND user_id = ?")
        .bind(is_enabled)
        .bind(now)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

/// 删除规则
pub async fn delete(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM model_routing_rules WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! model_routing_repo 单元测试（v1.55 测试体系 Phase 2）
    use super::*;

    async fn setup() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        // 建表时直接包含 user_id 字段和 UNIQUE(user_id, task_type) 约束
        // （0110 原始 migration 无 user_id，0121 migration 通过重建表添加；
        //  测试环境直接建最终形态的表，跳过迁移历史）
        sqlx::query(
            "CREATE TABLE model_routing_rules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL DEFAULT 1,
                task_type TEXT NOT NULL,
                provider TEXT NOT NULL,
                model_name TEXT NOT NULL,
                temperature REAL,
                max_tokens INTEGER,
                is_enabled INTEGER NOT NULL DEFAULT 1,
                is_cloud_only INTEGER NOT NULL DEFAULT 1,
                priority INTEGER NOT NULL DEFAULT 100,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                UNIQUE(user_id, task_type)
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_upsert_and_get_by_task_type() {
        let pool = setup().await;

        let row = upsert(
            &pool,
            1,
            "programming",
            "openai",
            "gpt-4o",
            Some(0.2),
            None,
            true,
            1000,
        )
        .await
        .unwrap();
        assert_eq!(row.task_type, "programming");
        assert_eq!(row.provider, "openai");
        assert!(row.is_cloud_only);
        assert!(row.is_enabled);

        let fetched = get_by_task_type(&pool, 1, "programming").await.unwrap().unwrap();
        assert_eq!(fetched.id, row.id);
        assert_eq!(fetched.model_name, "gpt-4o");

        // UPSERT 更新（同 task_type）
        let updated = upsert(
            &pool,
            1,
            "programming",
            "anthropic",
            "claude-sonnet-4-20250514",
            Some(0.0),
            Some(8192),
            false,
            2000,
        )
        .await
        .unwrap();
        assert_eq!(updated.id, row.id); // 同一条记录
        assert_eq!(updated.provider, "anthropic");
        assert_eq!(updated.model_name, "claude-sonnet-4-20250514");
        assert_eq!(updated.max_tokens, Some(8192));
        assert!(!updated.is_enabled);
    }

    #[tokio::test]
    async fn test_list_all_and_set_enabled_and_delete() {
        let pool = setup().await;

        upsert(&pool, 1, "programming", "openai", "gpt-4o", None, None, true, 1000)
            .await
            .unwrap();
        upsert(&pool, 1, "review", "anthropic", "claude-sonnet-4-20250514", None, None, true, 1000)
            .await
            .unwrap();

        let list = list_all(&pool, 1).await.unwrap();
        assert_eq!(list.len(), 2);

        let first_id = list[0].id;
        set_enabled(&pool, 1, first_id, false, 3000).await.unwrap();
        let fetched = get_by_task_type(&pool, 1, &list[0].task_type)
            .await
            .unwrap()
            .unwrap();
        assert!(!fetched.is_enabled);

        delete(&pool, 1, first_id).await.unwrap();
        let after = list_all(&pool, 1).await.unwrap();
        assert_eq!(after.len(), 1);
    }

    #[tokio::test]
    async fn test_get_by_task_type_returns_none_when_empty() {
        let pool = setup().await;
        let fetched = get_by_task_type(&pool, 1, "programming").await.unwrap();
        assert!(fetched.is_none());
    }
}
