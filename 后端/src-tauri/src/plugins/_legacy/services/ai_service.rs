use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::crypto::aes_gcm;
use crate::crypto::mek_manager::MekManager;
use crate::db::repositories::ai_repo;
use crate::error::app_error::AppError;
use crate::models::ai_model::{AiAgent, AiModel, validate_provider};

// ============================================================================
// 安全审计修复（多用户隔离批次 1）：所有方法添加 user_id 参数 + 传递给 repo 层
//
// 原实现：service 层不传 user_id，repo 层无 WHERE user_id = ? 过滤，
//         任何登录用户可读取/修改/删除他人的 AI 模型与 Agent 配置。
// 现实现：所有方法强制接收 user_id，传递给 repo 层做所有权过滤。
// 规范：安全审计报告/2026-07-25-代码安全审计报告.md 附录七
// ============================================================================

pub async fn get_models(pool: &SqlitePool, user_id: i64) -> Result<Vec<AiModel>, AppError> {
    ai_repo::get_all_models(pool, user_id).await
}

pub async fn add_model(
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    user_id: i64,
    name: &str,
    provider: &str,
    api_url: Option<&str>,
    api_key: Option<&str>,
    api_format: Option<&str>,
    model_name: Option<&str>,
    display_name: Option<&str>,
    is_local: bool,
    multimodal: bool,
    system_prompt: Option<&str>,
    context_window: Option<i64>,
    temperature: Option<f64>,
) -> Result<AiModel, AppError> {
    let provider_lower = provider.to_lowercase();
    validate_provider(&provider_lower)
        .map_err(|e| AppError::Validation(e))?;

    if !is_local && model_name.map_or(true, |m| m.trim().is_empty()) {
        return Err(AppError::Validation("云端API模型必须填写模型 ID (model_name)".into()));
    }

    let (api_key_enc, api_key_nonce) = if let Some(key) = api_key {
        if key.trim().is_empty() {
            (None, None)
        } else {
            let mgr = mek_manager.read().await;
            let mek = mgr
                .get_mek(user_id)
                .ok_or_else(|| AppError::MekDecryption("MEK 未在内存中".into()))?;
            let (enc, nonce) = aes_gcm::encrypt_bytes(key.as_bytes(), mek)?;
            (Some(enc), Some(nonce.to_vec()))
        }
    } else {
        (None, None)
    };

    let now = chrono::Utc::now().timestamp_millis();
    ai_repo::add_model(
        pool,
        user_id,
        name,
        &provider_lower,
        api_url,
        api_key_enc.as_deref(),
        api_key_nonce.as_deref(),
        api_format,
        model_name,
        display_name,
        is_local,
        multimodal,
        system_prompt,
        context_window,
        temperature,
        now,
    )
    .await
}

pub async fn update_model(
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    user_id: i64,
    id: i64,
    name: Option<&str>,
    provider: Option<&str>,
    api_url: Option<&str>,
    api_key: Option<&str>,
    api_format: Option<&str>,
    model_name: Option<&str>,
    display_name: Option<&str>,
    is_local: Option<bool>,
    multimodal: Option<bool>,
    system_prompt: Option<&str>,
    context_window: Option<i64>,
    temperature: Option<f64>,
) -> Result<(), AppError> {
    let provider_lower = provider.map(|p| p.to_lowercase());
    if let Some(ref p) = provider_lower {
        validate_provider(p).map_err(|e| AppError::Validation(e))?;
    }

    let (api_key_enc, api_key_nonce) = if let Some(key) = api_key {
        if key.trim().is_empty() {
            (None, None)
        } else {
            let mgr = mek_manager.read().await;
            let mek = mgr
                .get_mek(user_id)
                .ok_or_else(|| AppError::MekDecryption("MEK 未在内存中".into()))?;
            let (enc, nonce) = aes_gcm::encrypt_bytes(key.as_bytes(), mek)?;
            (Some(enc), Some(nonce.to_vec()))
        }
    } else {
        (None, None)
    };

    let now = chrono::Utc::now().timestamp_millis();
    ai_repo::update_model(
        pool,
        id,
        user_id,
        name,
        api_url,
        api_key_enc.as_deref(),
        api_key_nonce.as_deref(),
        api_format,
        model_name,
        display_name,
        is_local,
        multimodal,
        system_prompt,
        context_window,
        temperature,
        now,
    )
    .await
}

pub async fn delete_model(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
    ai_repo::delete_model(pool, id, user_id).await
}

pub async fn get_agents(pool: &SqlitePool, user_id: i64) -> Result<Vec<AiAgent>, AppError> {
    ai_repo::get_all_agents(pool, user_id).await
}

pub async fn get_agent_by_id(pool: &SqlitePool, id: i64, user_id: i64) -> Result<Option<AiAgent>, AppError> {
    ai_repo::get_agent_by_id(pool, id, user_id).await
}

pub async fn add_agent(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    description: Option<&str>,
    system_prompt: Option<&str>,
    model_id: Option<i64>,
) -> Result<AiAgent, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation("Agent 名称不能为空".into()));
    }

    if let Some(mid) = model_id {
        if ai_repo::get_model_by_id(pool, mid, user_id).await?.is_none() {
            return Err(AppError::Validation(format!("模型 ID {} 不存在", mid)));
        }
    }

    let now = chrono::Utc::now().timestamp_millis();
    ai_repo::add_agent(pool, user_id, name, description, system_prompt, model_id, now).await
}

pub async fn update_agent(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    name: Option<&str>,
    description: Option<&str>,
    system_prompt: Option<&str>,
    model_id: Option<i64>,
) -> Result<(), AppError> {
    if let Some(mid) = model_id {
        if ai_repo::get_model_by_id(pool, mid, user_id).await?.is_none() {
            return Err(AppError::Validation(format!("模型 ID {} 不存在", mid)));
        }
    }

    let now = chrono::Utc::now().timestamp_millis();
    ai_repo::update_agent(pool, id, user_id, name, description, system_prompt, model_id, now).await
}

pub async fn delete_agent(pool: &SqlitePool, id: i64, user_id: i64) -> Result<(), AppError> {
    ai_repo::delete_agent(pool, id, user_id).await
}
