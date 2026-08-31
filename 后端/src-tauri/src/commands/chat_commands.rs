use tauri::{AppHandle, State};
use std::sync::Arc;

use crate::db::connection::AppState;
use crate::models::ai_model::ConversationWithPreview;
use crate::models::api_response::ApiResponse;
use crate::models::chat::{
    Conversation, ConversationParticipant, CreateConversationRequest, Message,
    ModelHealthStatus, OrchestratorConfig, OrchestratorResult, SendMessageRequest,
    UpdateConversationRequest,
    SearchConversationsRequest, BranchConversationRequest, ExportConversationRequest,
    CreatePromptTemplateRequest, UpdatePromptTemplateRequest, PromptTemplate,
};
use crate::services::chat_service;
use crate::services::intelligence_v4_service;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。
///
/// 多用户隔离批次 2：所有命令通过 require_auth 取 user_id，透传到 chat_service → chat_repo，
/// 确保会话与消息按用户隔离。

#[tauri::command]
pub async fn create_conversation(
    state: State<'_, AppState>,
    request: CreateConversationRequest,
) -> Result<ApiResponse<Conversation>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let participants: Vec<(Option<i64>, Option<i64>, String)> = request
        .participants
        .into_iter()
        .map(|p| (p.model_id, p.agent_id, p.role))
        .collect();
    let participant_refs: Vec<(Option<i64>, Option<i64>, &str)> = participants
        .iter()
        .map(|(m, a, r)| (*m, *a, r.as_str()))
        .collect();

    match chat_service::create_conversation(
        &state.pool,
        user_id,
        request.title.as_deref(),
        &request.r#type,
        request.is_temp,
        request.token_budget,
        &participant_refs,
    )
    .await
    {
        Ok(conv) => Ok(ApiResponse::success(conv)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn get_conversations(
    state: State<'_, AppState>,
    r#type: Option<String>,
) -> Result<ApiResponse<Vec<ConversationWithPreview>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    // spec ai-chat-enhancement Phase 2 §2.1: 改为返回带最后消息预览的会话列表
    match chat_service::get_conversations_with_preview(&state.pool, user_id, r#type.as_deref()).await {
        Ok(convs) => Ok(ApiResponse::success(convs)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn update_conversation(
    state: State<'_, AppState>,
    request: UpdateConversationRequest,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match chat_service::update_conversation(
        &state.pool,
        request.id,
        user_id,
        request.title.as_deref(),
        request.token_budget,
    )
    .await
    {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn delete_conversation(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match chat_service::delete_conversation(&state.pool, id, user_id).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn send_message(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    request: SendMessageRequest,
) -> Result<ApiResponse<Message>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;

    match chat_service::send_message(
        &state.pool,
        request.conversation_id,
        "user",
        None,
        &request.content,
        user_id,
        &state.mek_manager,
        &app_handle,
    )
    .await
    {
        Ok(msg) => {
            intelligence_v4_service::instrument_cmd(&state, "ai_chat", "发送消息", None).await;
            Ok(ApiResponse::success(msg))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn get_messages(
    state: State<'_, AppState>,
    conversation_id: i64,
) -> Result<ApiResponse<Vec<Message>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match chat_service::get_messages(&state.pool, conversation_id, user_id).await {
        Ok(msgs) => Ok(ApiResponse::success(msgs)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn get_participants(
    state: State<'_, AppState>,
    conversation_id: i64,
) -> Result<ApiResponse<Vec<ConversationParticipant>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match chat_service::get_participants(&state.pool, conversation_id, user_id).await {
        Ok(parts) => Ok(ApiResponse::success(parts)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn delete_message(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match chat_service::delete_message(&state.pool, id, user_id).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn run_orchestrator(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    conversation_id: i64,
    user_message: String,
    config: Option<OrchestratorConfig>,
) -> Result<ApiResponse<OrchestratorResult>, String> {
    // 安全原则：始终使用 require_auth 取得的 auth_user_id，不信任前端传入的 user_id
    let auth_user_id = crate::commands::common::require_auth(&state).await?;
    let config = config.unwrap_or_default();

    let now = chrono::Utc::now().timestamp_millis();
    let _ = crate::db::repositories::chat_repo::add_message(
        &state.pool, auth_user_id, conversation_id, "user", None, &user_message, 0, now,
    ).await;

    let stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let mut flags = state.force_stop_flags.write().await;
        flags.insert(conversation_id, stop_flag.clone());
    }

    let result = chat_service::run_orchestrator(
        &state.pool,
        conversation_id,
        auth_user_id,
        &user_message,
        &config,
        &app_handle,
        &state.mek_manager,
        stop_flag,
    )
    .await;

    // 清理 stop flag
    {
        let mut flags = state.force_stop_flags.write().await;
        flags.remove(&conversation_id);
    }

    match result {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn stop_generation(
    state: State<'_, AppState>,
    conversation_id: i64,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match chat_service::stop_generation(&state.pool, conversation_id, &state.force_stop_flags).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn check_model_health(
    state: State<'_, AppState>,
    model_id: i64,
) -> Result<ApiResponse<ModelHealthStatus>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;

    match chat_service::check_model_health(&state.pool, model_id, user_id, &state.mek_manager).await
    {
        Ok(status) => Ok(ApiResponse::success(status)),
        Err(e) => Err(e.into()),
    }
}

// spec ai-chat-enhancement Phase 2 §3.2: 标记会话已读（unread_count 清零）
#[tauri::command]
pub async fn mark_conversation_read(
    state: State<'_, AppState>,
    conversation_id: i64,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match chat_service::mark_conversation_read(&state.pool, conversation_id, user_id).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

// spec ai-chat-enhancement Phase 2 §3.3: 批量更新会话排序（拖拽自定义排序）
#[tauri::command]
pub async fn reorder_conversations(
    state: State<'_, AppState>,
    items: Vec<crate::models::ai_model::ReorderItem>,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match chat_service::reorder_conversations(&state.pool, user_id, items).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

// ===== 搜索 =====

#[tauri::command]
pub async fn search_conversations(
    state: State<'_, AppState>,
    request: SearchConversationsRequest,
) -> Result<ApiResponse<Vec<crate::db::repositories::chat_repo::ConversationSearchResult>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match chat_service::search_conversations(&state.pool, user_id, &request.query).await {
        Ok(results) => Ok(ApiResponse::success(results)),
        Err(e) => Err(e.into()),
    }
}

// ===== 星标 =====

#[tauri::command]
pub async fn toggle_star_conversation(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<bool>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match chat_service::toggle_star_conversation(&state.pool, id, user_id).await {
        Ok(starred) => Ok(ApiResponse::success(starred)),
        Err(e) => Err(e.into()),
    }
}

// ===== 分支 =====

#[tauri::command]
pub async fn branch_conversation(
    state: State<'_, AppState>,
    request: BranchConversationRequest,
) -> Result<ApiResponse<Conversation>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match chat_service::branch_conversation(&state.pool, user_id, request.conversation_id, request.message_id).await {
        Ok(conv) => Ok(ApiResponse::success(conv)),
        Err(e) => Err(e.into()),
    }
}

// ===== 导出 =====

#[tauri::command]
pub async fn export_conversation(
    state: State<'_, AppState>,
    request: ExportConversationRequest,
) -> Result<ApiResponse<String>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match chat_service::export_conversation(&state.pool, request.conversation_id, user_id, &request.format).await {
        Ok(content) => Ok(ApiResponse::success(content)),
        Err(e) => Err(e.into()),
    }
}

// ===== Prompt 模板 =====
// 注：prompt_templates 为公共模板表，无 user_id 隔离需求，保持原样。

#[tauri::command]
pub async fn prompt_template_list(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<PromptTemplate>>, String> {
    crate::commands::common::require_auth(&state).await?;
    match chat_service::prompt_template_list(&state.pool).await {
        Ok(templates) => Ok(ApiResponse::success(templates)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn prompt_template_create(
    state: State<'_, AppState>,
    request: CreatePromptTemplateRequest,
) -> Result<ApiResponse<PromptTemplate>, String> {
    crate::commands::common::require_auth(&state).await?;
    match chat_service::prompt_template_create(
        &state.pool, &request.title, &request.category, &request.content,
    ).await {
        Ok(template) => Ok(ApiResponse::success(template)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn prompt_template_update(
    state: State<'_, AppState>,
    request: UpdatePromptTemplateRequest,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match chat_service::prompt_template_update(
        &state.pool, request.id,
        request.title.as_deref(),
        request.category.as_deref(),
        request.content.as_deref(),
    ).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn prompt_template_delete(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match chat_service::prompt_template_delete(&state.pool, id).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}
