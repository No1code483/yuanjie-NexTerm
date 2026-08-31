//! Yuan Code v3.1 Task 3.5 — 云端 API Key 管理 IPC 命令
//!
//! 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.5.1 / §3.5.3
//!       + 项目核心设计意图 §三（Yuan Code 编程 AI 必须走云端 API）
//!
//! 提供前端 ModelSelector UI 调用的 IPC 命令：
//! - cloud_api_list           : 列出所有已配置的云端 API Key
//! - cloud_api_upsert         : 新增/更新云端 API Key（加密落盘）
//! - cloud_api_set_enabled     : 启用/禁用某个 provider
//! - cloud_api_delete          : 删除某个 API Key
//! - cloud_api_test_connection: 测试某个 provider 的连通性
//! - cloud_api_providers      : 列出支持的云端 provider 白名单

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::State;

use crate::crypto::aes_gcm;
use crate::db::connection::AppState;
use crate::db::repositories::api_key_repo;
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;
use crate::models::api_key::{
    CLOUD_API_PROVIDERS, ApiKeyResponse, UpsertApiKeyRequest,
    get_cloud_provider_default_model, get_cloud_provider_default_url,
};

// 安全审计修复（发现 14，MEDIUM）：原 `get_user_id` 仅检查 `current_user.is_some()`，
// 不调用 `auth_service::verify_token`，不校验 token 签名/过期时间。
// 现移除本地 `get_user_id`，全部改用 `crate::commands::common::require_auth`，
// 强制走 `verify_token` 路径校验 token 签名与过期时间。

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 列出所有已配置的云端 API Key（不返回密文）
#[tauri::command]
pub async fn cloud_api_list(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<ApiKeyResponse>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let list = api_key_repo::list_all(&state.pool, user_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(list))
}

/// 新增或更新云端 API Key
///
/// 流程：
/// 1. 校验 provider 在云端白名单内（拒绝本地底层智能模型）
/// 2. 调用 aes_gcm::encrypt 加密明文（复用 MEK）
/// 3. UPSERT 到 api_keys 表
#[tauri::command]
pub async fn cloud_api_upsert(
    state: State<'_, AppState>,
    request: UpsertApiKeyRequest,
) -> Result<ApiResponse<ApiKeyResponse>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;

    // 校验（强制约束：仅允许云端 provider）
    request.validate().map_err(|e| e)?;

    // 加密（复用 MEK）— * deref 将 &[u8;32] 复制为 [u8;32]，避免 mgr 生命周期问题
    let mek: [u8; 32] = {
        let mgr = state.mek_manager.read().await;
        *mgr.get_mek(user_id)
            .ok_or_else(|| String::from("MEK 未在内存中，请重新登录"))?
    };
    let (enc, nonce) = aes_gcm::encrypt_bytes(request.api_key_plain.as_bytes(), &mek)
        .map_err(|e| e.to_string())?;

    let api_url = request
        .api_url
        .or_else(|| get_cloud_provider_default_url(&request.provider).map(String::from));

    let is_enabled = request.is_enabled.unwrap_or(true);
    let now = now_secs();

    let row = api_key_repo::upsert(
        &state.pool,
        user_id,
        &request.provider,
        request.display_name.as_deref(),
        &enc,
        &nonce,
        api_url.as_deref(),
        is_enabled,
        now,
    )
    .await
    .map_err(|e| e.to_string())?;

    tracing::info!(
        "[cloud_api] API Key 已配置: provider={}, id={}, user={}",
        row.provider, row.id, user_id
    );

    Ok(ApiResponse::success(row.to_response()))
}

/// 启用/禁用某个 provider 的 API Key
#[tauri::command]
pub async fn cloud_api_set_enabled(
    state: State<'_, AppState>,
    id: i64,
    is_enabled: bool,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    api_key_repo::set_enabled(&state.pool, id, user_id, is_enabled, now_secs())
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(()))
}

/// 删除某个 API Key
#[tauri::command]
pub async fn cloud_api_delete(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    api_key_repo::delete(&state.pool, id, user_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(()))
}

/// 列出支持的云端 provider 白名单
///
/// 前端 ModelSelector UI 用此构造可选项
#[tauri::command]
pub async fn cloud_api_providers() -> Result<ApiResponse<Vec<ProviderInfo>>, String> {
    let providers: Vec<ProviderInfo> = CLOUD_API_PROVIDERS
        .iter()
        .map(|&p| ProviderInfo {
            provider: p.to_string(),
            display_name: provider_display_name(p),
            default_api_url: get_cloud_provider_default_url(p).map(String::from),
            default_model: get_cloud_provider_default_model(p).map(String::from),
            available_models: available_models_for_provider(p),
        })
        .collect();
    Ok(ApiResponse::success(providers))
}

/// Provider 元信息（用于前端 ModelSelector）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderInfo {
    pub provider: String,
    pub display_name: String,
    pub default_api_url: Option<String>,
    pub default_model: Option<String>,
    pub available_models: Vec<String>,
}

fn provider_display_name(p: &str) -> String {
    match p {
        "openai" => "OpenAI (GPT-4o / o1 / o3-mini)".into(),
        "anthropic" => "Anthropic (Claude Sonnet/Opus/Haiku)".into(),
        "azure" => "Azure OpenAI".into(),
        "deepseek" => "DeepSeek (Chat / Reasoner / V3)".into(),
        "moonshot" => "Moonshot Kimi".into(),
        "zhipu" => "智谱 GLM".into(),
        "qwen" => "阿里通义千问".into(),
        "custom" => "自定义（OpenAI 兼容）".into(),
        _ => p.into(),
    }
}

fn available_models_for_provider(p: &str) -> Vec<String> {
    use crate::models::ai_model::get_provider_available_models;
    get_provider_available_models(p)
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// 测试某 provider 的 API Key 连通性
///
/// 调用 cloud_api_router 发送一个最小 prompt 验证 key 可用
#[tauri::command]
pub async fn cloud_api_test_connection(
    state: State<'_, AppState>,
    provider: String,
    model_name: String,
) -> Result<ApiResponse<ConnectionTestResult>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;

    // 强制校验：仅允许云端 provider
    let provider_lower = provider.to_lowercase();
    if !CLOUD_API_PROVIDERS.contains(&provider_lower.as_str()) {
        return Err(format!(
            "测试连通性失败：provider '{}' 不在云端白名单内（禁止本地底层智能模型）",
            provider
        ));
    }

    let ai_service = Arc::new(crate::services::ai_model_service::AiModelService::new());
    let router = crate::services::cloud_api_router::CloudApiRouter::new(ai_service);

    let req = crate::services::cloud_api_router::ProgrammingRequest {
        provider: provider_lower.clone(),
        model_name: model_name.clone(),
        prompt: "Reply with the single word: OK".into(),
        system_prompt: Some("You are a connection test assistant.".into()),
        temperature: Some(0.0),
        max_tokens: Some(10),
        stream: false,
        conversation_id: None,
        agent_id: Some("connection_test".into()),
    };

    let start = std::time::Instant::now();
    match router
        .route_programming_request(&state.pool, &state.mek_manager, user_id, req)
        .await
    {
        Ok(resp) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            Ok(ApiResponse::success(ConnectionTestResult {
                success: true,
                provider: resp.provider,
                model_used: resp.model_used,
                response_snippet: resp.content.chars().take(200).collect(),
                latency_ms,
                error: None,
            }))
        }
        Err(AppError::Validation(msg)) => {
            // provider 未配置 Key 等校验错误
            Ok(ApiResponse::success(ConnectionTestResult {
                success: false,
                provider: provider_lower,
                model_used: model_name,
                response_snippet: String::new(),
                latency_ms: start.elapsed().as_millis() as u64,
                error: Some(msg),
            }))
        }
        Err(e) => Ok(ApiResponse::success(ConnectionTestResult {
            success: false,
            provider: provider_lower,
            model_used: model_name,
            response_snippet: String::new(),
            latency_ms: start.elapsed().as_millis() as u64,
            error: Some(e.to_string()),
        })),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub provider: String,
    pub model_used: String,
    pub response_snippet: String,
    pub latency_ms: u64,
    pub error: Option<String>,
}
