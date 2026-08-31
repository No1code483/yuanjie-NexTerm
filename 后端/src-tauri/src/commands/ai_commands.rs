use tauri::{AppHandle, Emitter, State};
use serde::{Serialize, Deserialize};

use crate::db::connection::AppState;
use crate::db::repositories::ai_repo;
use crate::models::ai_model::{
    AddAgentRequest, AddModelRequest, AiAgent, AiModelResponse, ModelHealthInfo,
    UpdateAgentRequest, UpdateModelRequest,
    get_provider_available_models, get_provider_default_url, get_provider_default_model,
};
use crate::models::api_response::ApiResponse;
use crate::models::chat::OrchestratorResult;
use crate::services::ai_service;
use crate::services::chat_service;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn get_ai_models(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<AiModelResponse>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match ai_service::get_models(&state.pool, user_id).await {
        Ok(models) => {
            let response: Vec<AiModelResponse> =
                models.iter().map(|m| m.to_response()).collect();
            Ok(ApiResponse::success(response))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn add_ai_model(
    state: State<'_, AppState>,
    request: AddModelRequest,
) -> Result<ApiResponse<AiModelResponse>, String> {
    crate::commands::common::require_auth(&state).await?;
    if let Err(err) = request.validate() {
        return Err(err);
    }
    let user_id = {
        let user = state.current_user.read().await;
        user.ok_or_else(|| crate::error::app_error::AppError::Auth("未登录".into()))?
    };
    match ai_service::add_model(
        &state.pool,
        &state.mek_manager,
        user_id,
        &request.name,
        &request.provider,
        request.api_url.as_deref(),
        request.api_key.as_deref(),
        request.api_format.as_deref(),
        request.model_name.as_deref(),
        request.display_name.as_deref(),
        request.is_local,
        request.multimodal,
        request.system_prompt.as_deref(),
        request.context_window,
        request.temperature,
    )
    .await
    {
        Ok(model) => Ok(ApiResponse::success(model.to_response())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn update_ai_model(
    state: State<'_, AppState>,
    request: UpdateModelRequest,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    if let Err(err) = request.validate() {
        return Err(err);
    }
    let user_id = {
        let user = state.current_user.read().await;
        user.ok_or_else(|| crate::error::app_error::AppError::Auth("未登录".into()))?
    };
    match ai_service::update_model(
        &state.pool,
        &state.mek_manager,
        user_id,
        request.id,
        request.name.as_deref(),
        request.provider.as_deref(),
        request.api_url.as_deref(),
        request.api_key.as_deref(),
        request.api_format.as_deref(),
        request.model_name.as_deref(),
        request.display_name.as_deref(),
        request.is_local,
        request.multimodal,
        request.system_prompt.as_deref(),
        request.context_window,
        request.temperature,
    )
    .await
    {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn delete_ai_model(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match ai_service::delete_model(&state.pool, id, user_id).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn get_ai_agents(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<AiAgent>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match ai_service::get_agents(&state.pool, user_id).await {
        Ok(agents) => Ok(ApiResponse::success(agents)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn add_ai_agent(
    state: State<'_, AppState>,
    request: AddAgentRequest,
) -> Result<ApiResponse<AiAgent>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match ai_service::add_agent(
        &state.pool,
        user_id,
        &request.name,
        request.description.as_deref(),
        request.system_prompt.as_deref(),
        request.model_id,
    )
    .await
    {
        Ok(agent) => Ok(ApiResponse::success(agent)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn update_ai_agent(
    state: State<'_, AppState>,
    request: UpdateAgentRequest,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match ai_service::update_agent(
        &state.pool,
        user_id,
        request.id,
        request.name.as_deref(),
        request.description.as_deref(),
        request.system_prompt.as_deref(),
        request.model_id,
    )
    .await
    {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn delete_ai_agent(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match ai_service::delete_agent(&state.pool, id, user_id).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn ai_get_orchestration_status(
    state: State<'_, AppState>,
    conversation_id: i64,
) -> Result<ApiResponse<OrchestratorResult>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let messages = chat_service::get_messages(&state.pool, conversation_id, user_id)
        .await
        .map_err(|e| String::from(e))?;
    let total_tokens: i64 = messages.iter().map(|m| m.content.len() as i64 / 4).sum();
    let round_count = messages.iter().map(|m| m.round).max().unwrap_or(0) as u32;

    Ok(ApiResponse::success(OrchestratorResult {
        total_rounds: round_count,
        total_tokens,
        summary: format!("当前第{}轮，已消耗{} tokens", round_count, total_tokens),
    }))
}

#[tauri::command]
pub async fn ai_end_group_chat(
    state: State<'_, AppState>,
    conversation_id: i64,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match chat_service::stop_generation(&state.pool, conversation_id, &state.force_stop_flags).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub provider: String,
    pub default_url: Option<String>,
    pub default_model: Option<String>,
    pub available_models: Vec<String>,
}

#[tauri::command]
pub async fn get_ai_provider_info(
    state: State<'_, AppState>,
    provider: String,
) -> Result<ApiResponse<ProviderInfo>, String> {
    crate::commands::common::require_auth(&state).await?;
    let models = get_provider_available_models(&provider)
        .iter()
        .map(|s| s.to_string())
        .collect();
    Ok(ApiResponse::success(ProviderInfo {
        provider: provider.clone(),
        default_url: get_provider_default_url(&provider).map(|s| s.to_string()),
        default_model: get_provider_default_model(&provider).map(|s| s.to_string()),
        available_models: models,
    }))
}

// spec ai-chat-enhancement Phase 1 §1.3: 用户主动触发批量检测
//
// 立即执行一次全量检测（不等下一个周期），并发上限 5，结果同步返回。
// 与 `ModelHealthMonitor::start_background_task` 的检测逻辑共享 `run_check_once`，
// 检测过程中通过 `ai-model-health-changed` 事件实时推送状态变化给前端，
// 命令返回值为本次检测的 `Vec<ModelHealthStatus>`（与单模型检测返回类型一致）。
#[tauri::command]
pub async fn check_all_models_health(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<crate::models::chat::ModelHealthStatus>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;

    let models = ai_repo::get_all_models(&state.pool, user_id)
        .await
        .map_err(|e| String::from(e))?;

    use futures::stream::{iter, StreamExt};
    let results: Vec<crate::models::chat::ModelHealthStatus> = iter(models.into_iter())
        .map(|m| {
            let pool = state.pool.clone();
            let mek = state.mek_manager.clone();
            async move {
                chat_service::check_model_health(&pool, m.id, user_id, &mek)
                    .await
                    .ok()
            }
        })
        .buffer_unordered(crate::services::model_health_monitor::MAX_CONCURRENCY)
        .filter_map(|r| async { r })
        .collect()
        .await;

    // 同步广播一次"全量检测完成"事件（前端可借此刷新整列表）
    // 注：每个模型检测完成时 check_model_health 已通过 monitor 的事件或本身写 DB
    //     触发前端更新；此处广播可省略，但保留以兼容前端轮询模式。
    let _ = app_handle.emit(
        crate::services::model_health_monitor::HEALTH_CHANGED_EVENT,
        serde_json::json!({ "event": "batch_complete", "count": results.len() }),
    );

    Ok(ApiResponse::success(results))
}

// spec ai-chat-enhancement Phase 1 §1.3: 获取单个模型最新健康状态（从 DB 读取）
//
// 与 `check_model_health` 的差异：本命令不触发检测，仅返回 DB 中存储的最新状态
// （含 `status` 字符串、`latency_ms`、`last_health_check`），供前端首次进入页面时初始化显示。
#[tauri::command]
pub async fn get_model_health_status(
    state: State<'_, AppState>,
    model_id: i64,
) -> Result<ApiResponse<ModelHealthInfo>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let model = ai_repo::get_model_by_id(&state.pool, model_id, user_id)
        .await
        .map_err(|e| String::from(e))?
        .ok_or_else(|| format!("模型不存在: id={}", model_id))?;

    Ok(ApiResponse::success(ModelHealthInfo {
        model_id: model.id,
        name: model.name,
        status: model.status,
        latency_ms: model.latency_ms,
        last_health_check: model.last_health_check,
    }))
}

// spec ai-chat-enhancement Phase 1 §1.4: 设置检测间隔（前端 UI 调用）
//
// 单位：分钟（与前端 UI 选项 5/10/15/30 对齐）。下一次 sleep 周期生效。
// 注：此处通过 `state.model_health_monitor` 直接访问 AppState 中的 monitor 实例，
// 不需要 `Arc<ModelHealthMonitor>` —— ModelHealthMonitor 内部字段均为 Arc 包裹，Clone 廉价。
#[tauri::command]
pub async fn set_health_check_interval(
    state: State<'_, AppState>,
    minutes: u64,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    state.model_health_monitor.set_interval(minutes).await;
    Ok(ApiResponse::success(()))
}