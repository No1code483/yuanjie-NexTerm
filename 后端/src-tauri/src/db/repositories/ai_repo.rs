use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::ai_model::{AiAgent, AiModel};

// ============================================================================
// 安全审计修复（多用户隔离批次 1）：所有方法添加 user_id 参数 + WHERE user_id = ? 过滤
//
// 原实现：任何登录用户可读取/修改/删除他人的 AI 模型与 Agent 配置（含加密 API Key）。
// 现实现：所有查询/更新/删除操作强制按 user_id 过滤，确保用户只能操作自己的数据。
//
// 规范：安全审计报告/2026-07-25-代码安全审计报告.md 附录七
// ============================================================================

pub async fn get_all_models(pool: &SqlitePool, user_id: i64) -> Result<Vec<AiModel>, AppError> {
    sqlx::query_as::<_, AiModel>("SELECT * FROM ai_models WHERE user_id = ? ORDER BY created_at DESC")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn get_model_by_id(pool: &SqlitePool, id: i64, user_id: i64) -> Result<Option<AiModel>, AppError> {
    sqlx::query_as::<_, AiModel>("SELECT * FROM ai_models WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn add_model(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    provider: &str,
    api_url: Option<&str>,
    api_key_enc: Option<&[u8]>,
    api_key_nonce: Option<&[u8]>,
    api_format: Option<&str>,
    model_name: Option<&str>,
    display_name: Option<&str>,
    is_local: bool,
    multimodal: bool,
    system_prompt: Option<&str>,
    context_window: Option<i64>,
    temperature: Option<f64>,
    now: i64,
) -> Result<AiModel, AppError> {
    sqlx::query_as::<_, AiModel>(
        "INSERT INTO ai_models (user_id, name, provider, api_url, api_key_enc, api_key_nonce, api_format, model_name, display_name, is_local, multimodal, system_prompt, context_window, temperature, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(user_id)
    .bind(name)
    .bind(provider)
    .bind(api_url)
    .bind(api_key_enc)
    .bind(api_key_nonce)
    .bind(api_format)
    .bind(model_name)
    .bind(display_name)
    .bind(is_local)
    .bind(multimodal)
    .bind(system_prompt)
    .bind(context_window)
    .bind(temperature)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn update_model(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
    name: Option<&str>,
    api_url: Option<&str>,
    api_key_enc: Option<&[u8]>,
    api_key_nonce: Option<&[u8]>,
    api_format: Option<&str>,
    model_name: Option<&str>,
    display_name: Option<&str>,
    is_local: Option<bool>,
    multimodal: Option<bool>,
    system_prompt: Option<&str>,
    context_window: Option<i64>,
    temperature: Option<f64>,
    now: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE ai_models SET
         name = COALESCE(?, name),
         api_url = COALESCE(?, api_url),
         api_key_enc = COALESCE(?, api_key_enc),
         api_key_nonce = COALESCE(?, api_key_nonce),
         api_format = COALESCE(?, api_format),
         model_name = COALESCE(?, model_name),
         display_name = COALESCE(?, display_name),
         is_local = COALESCE(?, is_local),
         multimodal = COALESCE(?, multimodal),
         system_prompt = COALESCE(?, system_prompt),
         context_window = COALESCE(?, context_window),
         temperature = COALESCE(?, temperature),
         updated_at = ?
         WHERE id = ? AND user_id = ?",
    )
    .bind(name)
    .bind(api_url)
    .bind(api_key_enc)
    .bind(api_key_nonce)
    .bind(api_format)
    .bind(model_name)
    .bind(display_name)
    .bind(is_local)
    .bind(multimodal)
    .bind(system_prompt)
    .bind(context_window)
    .bind(temperature)
    .bind(now)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn delete_model(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM ai_models WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

/// 更新模型健康状态（由 check_model_health 和 ModelHealthMonitor 调用）
///
/// - status: online / offline / checking / error / unknown
/// - latency_ms: 成功检测的延迟（失败时传 None）
/// - now: 当前 unix timestamp ms
///
/// 安全说明：本方法按 model_id 定位记录，不传 user_id 过滤。
/// 原因：健康监测由系统后台任务（ModelHealthMonitor）触发，非用户直接调用，
/// 需要跨用户更新所有匹配 model_id 的记录。调用方（健康监测服务）已通过 require_auth
/// 或后台任务授权，不存在 IDOR 风险。
pub async fn update_model_health(
    pool: &SqlitePool,
    model_id: i64,
    status: &str,
    latency_ms: Option<i64>,
    now: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE ai_models SET status = ?, latency_ms = ?, last_health_check = ?, updated_at = ? WHERE id = ?",
    )
    .bind(status)
    .bind(latency_ms)
    .bind(now)
    .bind(now)
    .bind(model_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn get_all_agents(pool: &SqlitePool, user_id: i64) -> Result<Vec<AiAgent>, AppError> {
    sqlx::query_as::<_, AiAgent>("SELECT * FROM ai_agents WHERE user_id = ? ORDER BY created_at DESC")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn get_agent_by_id(pool: &SqlitePool, id: i64, user_id: i64) -> Result<Option<AiAgent>, AppError> {
    sqlx::query_as::<_, AiAgent>("SELECT * FROM ai_agents WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn add_agent(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    description: Option<&str>,
    system_prompt: Option<&str>,
    model_id: Option<i64>,
    now: i64,
) -> Result<AiAgent, AppError> {
    sqlx::query_as::<_, AiAgent>(
        "INSERT INTO ai_agents (user_id, name, description, system_prompt, model_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(user_id)
    .bind(name)
    .bind(description)
    .bind(system_prompt)
    .bind(model_id)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn update_agent(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
    name: Option<&str>,
    description: Option<&str>,
    system_prompt: Option<&str>,
    model_id: Option<i64>,
    now: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE ai_agents SET
            name = COALESCE(?, name),
            description = COALESCE(?, description),
            system_prompt = COALESCE(?, system_prompt),
            model_id = COALESCE(?, model_id),
            updated_at = ?
         WHERE id = ? AND user_id = ?",
    )
    .bind(name)
    .bind(description)
    .bind(system_prompt)
    .bind(model_id)
    .bind(now)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn delete_agent(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM ai_agents WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}
