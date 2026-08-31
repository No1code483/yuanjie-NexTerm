use std::sync::Arc;

use sqlx::SqlitePool;
use tauri::State;
use tokio::sync::Mutex;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::xin::{PersonaTrait, SpeakingStyle};
use crate::services::xin_context_service::{
    estimate_tokens, BuiltinSkills, ChatMessage, ChatRole, CompactionResult, ContextWindow,
    SkillInfo, SkillResult, XinCompactor, XinContextManager, XinPromptBuilder, XinPromptConfig,
};
use crate::services::xin_dialogue_service::XinChatResponse;
use crate::services::xin_knowledge_fusion_service::{
    RetrievalDecision, UnifiedResult, XinKnowledgeFusion,
};
use crate::services::xin_post_process_service::{
    PostProcessEnhancedResult, QualityScoreResult, SafetyCheckResult, XinPostProcessor,
};
use crate::services::xin_checkpoint_service::{
    CheckpointConfig, CheckpointSummary, RestoreResult,
    XinCheckpointService,
};
use crate::services::xin_conversation_search_service::{
    ConversationSearchQuery, ConversationSearchResponse,
    XinConversationSearchService,
};
use crate::services::xin_conversation_review_service::{
    ConversationReview, GrowthTrajectory, HeatmapData, ReviewRequest,
    TopicTrendResponse, XinConversationReviewService,
};
use crate::services::xin_compaction_service::{
    CompactionConfig, CompactionRecord, CompactionResponse,
    XinCompactionService,
};
use crate::services::xin_commit_service::{
    CommitmentCandidate, CommitmentRecord, XinCommitService,
};
use crate::services::xin_michelin_service::{
    EvalResult, EvalReport, XinMichelinService,
};
use crate::services::xin_dream_service::{
    DreamConfig, DreamState, LightDreamConfig, LightDreamResult, DeepDreamConfig,
    DeepDreamResult, RemDreamConfig, RemDreamResult, XinDreamService,
};
use crate::services::xin_personality_evolution_service::{
    AppliedAdjustment, EvolutionDecision, EvolutionSnapshot, EvolutionTimelineEntry,
    PersonalityVariant, VariantCompareResult, XinPersonalityEvolutionService,
};
use crate::services::xin_proactive_service::{
    ProactiveAction, QuietHours, XinProactiveService,
};
use crate::services::xin_tool_service::{ToolCall, ToolExecutor, ToolRegistry};

static SKILLS: once_cell::sync::Lazy<Arc<Mutex<BuiltinSkills>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(BuiltinSkills::new())));

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn xin_v3_estimate_tokens(
    state: State<'_, AppState>,
    text: String,
) -> Result<ApiResponse<usize>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(estimate_tokens(&text)))
}

#[tauri::command]
pub async fn xin_v3_context_new(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<ApiResponse<ContextWindow>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(XinContextManager::new_context(&model_id)))
}

#[tauri::command]
pub async fn xin_v3_context_add_message(
    state: State<'_, AppState>,
    model_id: String,
    messages_json: String,
    role: String,
    content: String,
    name: Option<String>,
) -> Result<ApiResponse<ContextWindow>, String> {
    crate::commands::common::require_auth(&state).await?;
    let messages: Vec<ChatMessage> =
        serde_json::from_str(&messages_json).unwrap_or_default();

    let role = match role.as_str() {
        "system" => ChatRole::System,
        "user" => ChatRole::User,
        "assistant" => ChatRole::Assistant,
        _ => ChatRole::User,
    };

    let msg = ChatMessage {
        role,
        content,
        name,
        timestamp: None,
    };

    let mut ctx = XinContextManager::new_context(&model_id);
    for existing in messages {
        XinContextManager::add_message(&mut ctx, existing);
    }
    XinContextManager::add_message(&mut ctx, msg);

    Ok(ApiResponse::success(ctx))
}

#[tauri::command]
pub async fn xin_v3_context_trim(
    state: State<'_, AppState>,
    model_id: String,
    messages_json: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let messages: Vec<ChatMessage> =
        serde_json::from_str(&messages_json).unwrap_or_default();

    let mut ctx = XinContextManager::new_context(&model_id);
    for msg in messages {
        XinContextManager::add_message(&mut ctx, msg);
    }

    let removed = XinContextManager::trim_to_budget(&mut ctx);
    Ok(ApiResponse::success(serde_json::json!({
        "context": ctx,
        "removed_count": removed
    })))
}

#[tauri::command]
pub async fn xin_v3_context_stats(
    state: State<'_, AppState>,
    model_id: String,
    messages_json: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let messages: Vec<ChatMessage> =
        serde_json::from_str(&messages_json).unwrap_or_default();

    let mut ctx = XinContextManager::new_context(&model_id);
    for msg in messages {
        XinContextManager::add_message(&mut ctx, msg);
    }

    let role_counts = XinContextManager::count_by_role(&ctx);
    let is_over = XinContextManager::is_over_budget(&ctx);
    let remaining = XinContextManager::token_budget_remaining(&ctx);

    Ok(ApiResponse::success(serde_json::json!({
        "total_messages": ctx.messages.len(),
        "used_tokens": ctx.used_tokens,
        "max_tokens": ctx.max_tokens,
        "remaining_tokens": remaining,
        "is_over_budget": is_over,
        "role_counts": role_counts
    })))
}

#[tauri::command]
pub async fn xin_v3_compact(
    state: State<'_, AppState>,
    messages_json: String,
    model_id: String,
) -> Result<ApiResponse<CompactionResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let messages: Vec<ChatMessage> =
        serde_json::from_str(&messages_json).unwrap_or_default();
    Ok(ApiResponse::success(XinCompactor::compact(&messages, &model_id)))
}

#[tauri::command]
pub async fn xin_v3_recovery_briefing(
    state: State<'_, AppState>,
    messages_json: String,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let messages: Vec<ChatMessage> =
        serde_json::from_str(&messages_json).unwrap_or_default();
    Ok(ApiResponse::success(XinCompactor::generate_recovery_briefing(
        &messages,
    )))
}

#[tauri::command]
pub async fn xin_v3_build_prompt(
    state: State<'_, AppState>,
    include_skills: bool,
    include_mood: bool,
    include_time: bool,
    custom_instructions: Option<String>,
) -> Result<ApiResponse<String>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let persona = state.xiaoxin_service.get_active_persona(user_id).await;
    let config = XinPromptConfig {
        persona,
        include_skills,
        include_mood,
        include_time,
        custom_instructions,
        user_emotion_guide: None,
        persona_memory: None,
    };
    let mut prompt = XinPromptBuilder::build_system_prompt(&config);

    if include_skills {
        let skills = SKILLS.lock().await;
        prompt.push_str("\n\n");
        prompt.push_str(&skills.build_skills_prompt());
    }

    Ok(ApiResponse::success(prompt))
}

#[tauri::command]
pub async fn xin_v3_build_context_header(
    state: State<'_, AppState>,
    summary: Option<String>,
    recent_topics: Vec<String>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(XinPromptBuilder::build_context_header(
        summary.as_deref(),
        &recent_topics,
    )))
}

#[tauri::command]
pub async fn xin_v3_list_skills(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<SkillInfo>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let skills = SKILLS.lock().await;
    Ok(ApiResponse::success(skills.list_all()))
}

#[tauri::command]
pub async fn xin_v3_execute_skill(
    state: State<'_, AppState>,
    skill_id: String,
    input: String,
) -> Result<ApiResponse<SkillResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let skills = SKILLS.lock().await;
    match skills.execute(&skill_id, &input) {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Ok(ApiResponse::error(500, &e)),
    }
}

#[tauri::command]
pub async fn xin_v3_dialogue_create(
    state: State<'_, AppState>,
    persona_id: Option<String>,
    model_id: String,
    title: Option<String>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let persona = if let Some(pid) = persona_id {
        state.xiaoxin_service.get_persona(user_id, &pid).await
            .ok_or_else(|| format!("人格不存在: {}", pid))?
    } else {
        state.xiaoxin_service.get_active_persona(user_id).await
    };

    let summary = state
        .xin_dialogue_service
        .create_conversation(user_id, &persona, &model_id, title.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(ApiResponse::success(serde_json::json!(summary)))
}

#[tauri::command]
pub async fn xin_v3_dialogue_get(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let persona = state.xiaoxin_service.get_active_persona(user_id).await;
    let summary = state
        .xin_dialogue_service
        .get_conversation(user_id, &conversation_id, &persona)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ApiResponse::success(serde_json::json!(summary)))
}

#[tauri::command]
pub async fn xin_v3_dialogue_list(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let persona = state.xiaoxin_service.get_active_persona(user_id).await;
    let summaries = state
        .xin_dialogue_service
        .list_conversations(user_id, &persona)
        .await
        .map_err(|e| e.to_string())?;

    let result: Vec<serde_json::Value> = summaries
        .into_iter()
        .map(|s| serde_json::json!(s))
        .collect();

    Ok(ApiResponse::success(result))
}

#[tauri::command]
pub async fn xin_v3_dialogue_delete(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<ApiResponse<String>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    state
        .xin_dialogue_service
        .delete_conversation(user_id, &conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ApiResponse::success("ok".to_string()))
}

#[tauri::command]
pub async fn xin_v3_dialogue_search(
    state: State<'_, AppState>,
    query: ConversationSearchQuery,
) -> Result<ApiResponse<ConversationSearchResponse>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let response = XinConversationSearchService::search(&state.pool, user_id, &query)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(response))
}

#[tauri::command]
pub async fn xin_v3_dialogue_send(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    persona_id: Option<String>,
    model_id: Option<i64>,
    conversation_id: Option<String>,
    message: String,
    skills_enabled: Option<bool>,
    stream: Option<bool>,
    attachments: Option<Vec<crate::services::xin_multimodal_service::Attachment>>,
) -> Result<ApiResponse<XinChatResponse>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    // A5 Phase 3 Task 2: 云端 AI 离线守卫
    //
    // 设计依据：.trae/rules/项目核心设计意图.md §四（小欣走云端 API，不切换本地 ollama）
    // 离线时直接返回 Offline 错误（前端显示「AI 服务不可用，请连接网络」），
    // 不进入 send_message 流程，避免触发云端 API 请求。
    crate::services::ai_model_service::AiModelService::check_online_before_call(&state)
        .await
        .map_err(|e| e.to_string())?;

    // D3.1: model_id 为空时，回退到第一个可用模型
    let resolved_model_id = if let Some(mid) = model_id {
        mid
    } else {
        let models = crate::db::repositories::ai_repo::get_all_models(&state.pool, user_id)
            .await
            .map_err(|e| e.to_string())?;
        models
            .first()
            .map(|m| m.id)
            .ok_or_else(|| "未配置任何 AI 模型，请先在设置中添加模型".to_string())?
    };

    let persona = if let Some(pid) = persona_id {
        state.xiaoxin_service.get_persona(user_id, &pid).await
            .ok_or_else(|| format!("人格不存在: {}", pid))?
    } else {
        state.xiaoxin_service.get_active_persona(user_id).await
    };

    let response = state
        .xin_dialogue_service
        .send_message(
            &app_handle,
            &persona,
            user_id,
            resolved_model_id,
            conversation_id.as_deref(),
            &message,
            skills_enabled.unwrap_or(true),
            stream.unwrap_or(true),
            attachments,
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(ApiResponse::success(response))
}

#[tauri::command]
pub async fn xin_v3_dialogue_stop(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    state
        .xin_dialogue_service
        .stop_generation(&conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ApiResponse::success("stopped".to_string()))
}

#[tauri::command]
pub async fn xin_v3_analyze_intent(
    state: State<'_, AppState>,
    message: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let result = XinPostProcessor::analyze_intent(&message);
    Ok(ApiResponse::success(serde_json::to_value(&result).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_knowledge_fusion(
    state: State<'_, AppState>,
    message: String,
    max_results: Option<usize>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let result = XinKnowledgeFusion::retrieve_relevant(
        &state.pool,
        user_id,
        &message,
        max_results.unwrap_or(5),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(serde_json::to_value(&result).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_fusion_should_retrieve(
    state: State<'_, AppState>,
    message: String,
) -> Result<ApiResponse<RetrievalDecision>, String> {
    crate::commands::common::require_auth(&state).await?;
    let decision = XinKnowledgeFusion::should_retrieve(&message);
    Ok(ApiResponse::success(decision))
}

#[tauri::command]
pub async fn xin_v3_fusion_memory_query(
    state: State<'_, AppState>,
    query: String,
    max_results: Option<usize>,
) -> Result<ApiResponse<Vec<UnifiedResult>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let results = XinKnowledgeFusion::query_memories(
        &state.pool,
        user_id,
        &query,
        max_results.unwrap_or(5),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(results))
}

#[tauri::command]
pub async fn xin_v3_fusion_multi_source(
    state: State<'_, AppState>,
    kb_json: serde_json::Value,
    memory_json: serde_json::Value,
    max_total: Option<usize>,
) -> Result<ApiResponse<Vec<UnifiedResult>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let kb: Vec<UnifiedResult> =
        serde_json::from_value(kb_json).map_err(|e| e.to_string())?;
    let mem: Vec<UnifiedResult> =
        serde_json::from_value(memory_json).map_err(|e| e.to_string())?;
    let fused = XinKnowledgeFusion::fuse_multi_source(kb, mem, max_total.unwrap_or(8));
    Ok(ApiResponse::success(fused))
}

#[tauri::command]
pub async fn xin_v3_fusion_unified_context(
    state: State<'_, AppState>,
    results_json: serde_json::Value,
    max_chars: Option<usize>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let results: Vec<UnifiedResult> =
        serde_json::from_value(results_json).map_err(|e| e.to_string())?;
    let ctx = XinKnowledgeFusion::build_unified_context(&results, max_chars.unwrap_or(2000));
    Ok(ApiResponse::success(ctx))
}

#[tauri::command]
pub async fn xin_v3_fusion_extract_keywords(
    state: State<'_, AppState>,
    message: String,
) -> Result<ApiResponse<Vec<String>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let keywords = XinKnowledgeFusion::extract_query_keywords(&message);
    Ok(ApiResponse::success(keywords))
}

#[tauri::command]
pub async fn xin_v3_list_tools(
    state: State<'_, AppState>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let registry = ToolRegistry::new();
    let tools = registry.list();
    Ok(ApiResponse::success(serde_json::to_value(&tools).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_parse_tool_calls(
    state: State<'_, AppState>,
    text: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let calls = ToolExecutor::parse_tool_calls(&text);
    Ok(ApiResponse::success(serde_json::to_value(&calls).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_execute_tool(
    state: State<'_, AppState>,
    tool_calls_json: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let calls: Vec<ToolCall> =
        serde_json::from_str(&tool_calls_json).map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    for call in &calls {
        let result = ToolExecutor::execute(&state.pool, user_id, call)
            .await
            .map_err(|e| e.to_string())?;
        results.push(result);
    }
    Ok(ApiResponse::success(serde_json::to_value(&results).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_get_tools_prompt(
    state: State<'_, AppState>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let registry = ToolRegistry::new();
    Ok(ApiResponse::success(registry.build_tools_prompt()))
}

#[tauri::command]
pub async fn xin_v3_parse_attachment(
    state: State<'_, AppState>,
    attachment: crate::services::xin_multimodal_service::Attachment,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let result =
        crate::services::xin_multimodal_service::XinMultimodalService::parse_attachment(&attachment)
            .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(serde_json::to_value(&result).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_enhance_output(
    state: State<'_, AppState>,
    text: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let enhancement =
        crate::services::xin_multimodal_service::XinMultimodalService::enhance_output(&text);
    Ok(ApiResponse::success(
        crate::services::xin_multimodal_service::XinMultimodalService::format_for_display(
            &text,
            &enhancement,
        ),
    ))
}

#[tauri::command]
pub async fn xin_v3_detect_code_languages(
    state: State<'_, AppState>,
    text: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let langs = crate::services::xin_multimodal_service::XinMultimodalService::detect_code_languages(&text);
    Ok(ApiResponse::success(serde_json::to_value(&langs).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_analyze_sentiment(
    state: State<'_, AppState>,
    text: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let result = XinPostProcessor::analyze_sentiment(&text);
    Ok(ApiResponse::success(serde_json::to_value(&result).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_extract_topics(
    state: State<'_, AppState>,
    user_message: String,
    assistant_response: String,
    previous_topics: Option<Vec<String>>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let prev = previous_topics.unwrap_or_default();
    let result = XinPostProcessor::extract_topics(&user_message, &assistant_response, &prev);
    Ok(ApiResponse::success(serde_json::to_value(&result).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_post_process(
    state: State<'_, AppState>,
    user_message: String,
    assistant_response: String,
    previous_topics: Option<Vec<String>>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let prev = previous_topics.unwrap_or_default();
    let result = XinPostProcessor::run_pipeline(&user_message, &assistant_response, &prev);
    Ok(ApiResponse::success(serde_json::to_value(&result).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_post_score_quality(
    state: State<'_, AppState>,
    user_message: String,
    assistant_response: String,
) -> Result<ApiResponse<QualityScoreResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let result = XinPostProcessor::score_quality(&user_message, &assistant_response);
    Ok(ApiResponse::success(result))
}

#[tauri::command]
pub async fn xin_v3_post_adjust_tone(
    state: State<'_, AppState>,
    response: String,
    warmth: Option<f64>,
    formality: Option<f64>,
    enthusiasm: Option<f64>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut traits = std::collections::HashMap::new();
    traits.insert("warmth".to_string(), warmth.unwrap_or(0.6));
    traits.insert("formality".to_string(), formality.unwrap_or(0.5));
    traits.insert("enthusiasm".to_string(), enthusiasm.unwrap_or(0.6));
    let result = XinPostProcessor::adjust_tone(&response, &traits);
    Ok(ApiResponse::success(serde_json::to_value(&result).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_post_check_factuality(
    state: State<'_, AppState>,
    response: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let result = XinPostProcessor::check_factuality(&response);
    Ok(ApiResponse::success(serde_json::to_value(&result).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_post_safety_filter(
    state: State<'_, AppState>,
    response: String,
) -> Result<ApiResponse<SafetyCheckResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let result = XinPostProcessor::safety_filter(&response);
    Ok(ApiResponse::success(result))
}

#[tauri::command]
pub async fn xin_v3_post_enhanced_pipeline(
    state: State<'_, AppState>,
    user_message: String,
    assistant_response: String,
    previous_topics: Option<Vec<String>>,
    warmth: Option<f64>,
    formality: Option<f64>,
    enthusiasm: Option<f64>,
) -> Result<ApiResponse<PostProcessEnhancedResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let prev = previous_topics.unwrap_or_default();
    let mut traits = std::collections::HashMap::new();
    traits.insert("warmth".to_string(), warmth.unwrap_or(0.6));
    traits.insert("formality".to_string(), formality.unwrap_or(0.5));
    traits.insert("enthusiasm".to_string(), enthusiasm.unwrap_or(0.6));
    let result = XinPostProcessor::run_enhanced_pipeline(
        &user_message,
        &assistant_response,
        &prev,
        Some(traits),
    );
    Ok(ApiResponse::success(result))
}

#[tauri::command]
pub async fn xin_v3_dream_check_due(
    state: State<'_, AppState>,
    phase: String,
    dream_state: DreamState,
    config: DreamConfig,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(XinDreamService::check_dreaming_due(
        &phase, &dream_state, &config,
    )))
}

#[tauri::command]
pub async fn xin_v3_dream_run_light(
    state: State<'_, AppState>,
    conversation_texts: Vec<String>,
    config: LightDreamConfig,
) -> Result<ApiResponse<LightDreamResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(XinDreamService::run_light_dreaming(
        &[],
        &conversation_texts,
        &config,
    )))
}

#[tauri::command]
pub async fn xin_v3_dream_run_deep(
    state: State<'_, AppState>,
    candidates: serde_json::Value,
    memories: serde_json::Value,
    config: DeepDreamConfig,
) -> Result<ApiResponse<DeepDreamResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let candidates: Vec<crate::services::xin_dream_service::DreamCandidate> =
        serde_json::from_value(candidates).map_err(|e| e.to_string())?;
    let memories: Vec<crate::models::xin::UserMemory> =
        serde_json::from_value(memories).map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(XinDreamService::run_deep_dreaming(
        &candidates,
        &memories,
        &config,
    )))
}

#[tauri::command]
pub async fn xin_v3_dream_run_rem(
    state: State<'_, AppState>,
    memories: serde_json::Value,
    config: RemDreamConfig,
) -> Result<ApiResponse<RemDreamResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let memories: Vec<crate::models::xin::UserMemory> =
        serde_json::from_value(memories).map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(XinDreamService::run_rem_dreaming(
        &memories, &[], &config,
    )))
}

#[tauri::command]
pub async fn xin_v3_dream_calc_health(
    state: State<'_, AppState>,
    memories: serde_json::Value,
) -> Result<ApiResponse<f64>, String> {
    crate::commands::common::require_auth(&state).await?;
    let memories: Vec<crate::models::xin::UserMemory> =
        serde_json::from_value(memories).map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(XinDreamService::calc_memory_health(
        &memories,
    )))
}

#[tauri::command]
pub async fn xin_v3_dream_auto_promote(
    state: State<'_, AppState>,
    candidates: serde_json::Value,
) -> Result<ApiResponse<Vec<crate::models::xin::UserMemory>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let candidates: Vec<crate::services::xin_dream_service::DreamCandidate> =
        serde_json::from_value(candidates).map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(XinDreamService::auto_promote_to_memories(
        &candidates,
    )))
}

#[tauri::command]
pub async fn xin_v3_dream_default_config(
    state: State<'_, AppState>,
) -> Result<ApiResponse<DreamConfig>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(DreamConfig::default()))
}

#[tauri::command]
pub async fn xin_v3_commit_extract(
    state: State<'_, AppState>,
    user_msg: String,
    assistant_msg: String,
    conversation_id: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let result = XinCommitService::extract_from_dialogue(&user_msg, &assistant_msg, &conversation_id);
    Ok(ApiResponse::success(serde_json::to_value(&result).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_commit_create(
    state: State<'_, AppState>,
    candidates: serde_json::Value,
) -> Result<ApiResponse<Vec<CommitmentRecord>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let candidates: Vec<CommitmentCandidate> =
        serde_json::from_value(candidates).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now();
    let records: Vec<CommitmentRecord> = candidates
        .iter()
        .map(|c| XinCommitService::create_commitment(c, now))
        .collect();
    Ok(ApiResponse::success(records))
}

#[tauri::command]
pub async fn xin_v3_commit_list_pending(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<CommitmentRecord>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(vec![]))
}

#[tauri::command]
pub async fn xin_v3_commit_check_due(
    state: State<'_, AppState>,
    commitments: serde_json::Value,
) -> Result<ApiResponse<Vec<CommitmentRecord>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let commitments: Vec<CommitmentRecord> =
        serde_json::from_value(commitments).map_err(|e| e.to_string())?;
    let due = XinCommitService::check_now_due(&commitments);
    Ok(ApiResponse::success(due.into_iter().cloned().collect()))
}

#[tauri::command]
pub async fn xin_v3_commit_mark_done(
    state: State<'_, AppState>,
    mut commitment: CommitmentRecord,
) -> Result<ApiResponse<CommitmentRecord>, String> {
    crate::commands::common::require_auth(&state).await?;
    XinCommitService::mark_done(&mut commitment);
    Ok(ApiResponse::success(commitment))
}

#[tauri::command]
pub async fn xin_v3_commit_snooze(
    state: State<'_, AppState>,
    mut commitment: CommitmentRecord,
    hours: i64,
) -> Result<ApiResponse<CommitmentRecord>, String> {
    crate::commands::common::require_auth(&state).await?;
    XinCommitService::snooze(&mut commitment, hours);
    Ok(ApiResponse::success(commitment))
}

#[tauri::command]
pub async fn xin_v3_commit_dismiss(
    state: State<'_, AppState>,
    mut commitment: CommitmentRecord,
) -> Result<ApiResponse<CommitmentRecord>, String> {
    crate::commands::common::require_auth(&state).await?;
    XinCommitService::dismiss(&mut commitment);
    Ok(ApiResponse::success(commitment))
}

#[tauri::command]
pub async fn xin_v3_commit_stats(
    state: State<'_, AppState>,
    commitments: serde_json::Value,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let commitments: Vec<CommitmentRecord> =
        serde_json::from_value(commitments).map_err(|e| e.to_string())?;
    let stats = XinCommitService::get_stats(&commitments);
    Ok(ApiResponse::success(serde_json::to_value(&stats).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_eval_score(
    state: State<'_, AppState>,
    user_msg: String,
    assistant_msg: String,
    response_time_ms: u32,
) -> Result<ApiResponse<EvalResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let result = XinMichelinService::evaluate_dialogue(&user_msg, &assistant_msg, response_time_ms);
    Ok(ApiResponse::success(result))
}

#[tauri::command]
pub async fn xin_v3_eval_history(
    state: State<'_, AppState>,
    results: serde_json::Value,
    count: usize,
) -> Result<ApiResponse<Vec<EvalResult>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let results: Vec<EvalResult> = serde_json::from_value(results).map_err(|e| e.to_string())?;
    let filtered = XinMichelinService::filter_recent(&results, count);
    Ok(ApiResponse::success(filtered))
}

#[tauri::command]
pub async fn xin_v3_eval_stats(
    state: State<'_, AppState>,
    results: serde_json::Value,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let results: Vec<EvalResult> = serde_json::from_value(results).map_err(|e| e.to_string())?;
    let stats = XinMichelinService::aggregate_stats(&results);
    Ok(ApiResponse::success(serde_json::to_value(&stats).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_eval_report(
    state: State<'_, AppState>,
    results: serde_json::Value,
) -> Result<ApiResponse<EvalReport>, String> {
    crate::commands::common::require_auth(&state).await?;
    let results: Vec<EvalResult> = serde_json::from_value(results).map_err(|e| e.to_string())?;
    let report = XinMichelinService::generate_report(&results);
    Ok(ApiResponse::success(report))
}

#[tauri::command]
pub async fn xin_v3_evolution_rules(
    state: State<'_, AppState>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let rules = XinPersonalityEvolutionService::builtin_rules();
    Ok(ApiResponse::success(serde_json::to_value(&rules).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_evolution_analyze(
    state: State<'_, AppState>,
    eval_results: serde_json::Value,
    style_json: serde_json::Value,
) -> Result<ApiResponse<EvolutionDecision>, String> {
    crate::commands::common::require_auth(&state).await?;
    let results: Vec<EvalResult> =
        serde_json::from_value(eval_results).map_err(|e| e.to_string())?;
    let style: SpeakingStyle =
        serde_json::from_value(style_json).map_err(|e| e.to_string())?;
    let decision = XinPersonalityEvolutionService::analyze_evolution(&results, &style);
    Ok(ApiResponse::success(decision))
}

#[tauri::command]
pub async fn xin_v3_evolution_apply(
    state: State<'_, AppState>,
    style_json: serde_json::Value,
    decision_json: serde_json::Value,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let style: SpeakingStyle =
        serde_json::from_value(style_json).map_err(|e| e.to_string())?;
    let decision: EvolutionDecision =
        serde_json::from_value(decision_json).map_err(|e| e.to_string())?;
    let new_style = XinPersonalityEvolutionService::apply_decision(&style, &decision);
    Ok(ApiResponse::success(serde_json::to_value(&new_style).map_err(|e| e.to_string())?))
}

#[tauri::command]
pub async fn xin_v3_evolution_variant_create(
    state: State<'_, AppState>,
    base_persona_id: String,
    name: String,
    style_json: serde_json::Value,
    traits_json: serde_json::Value,
    adjustments_json: serde_json::Value,
    description: String,
) -> Result<ApiResponse<PersonalityVariant>, String> {
    crate::commands::common::require_auth(&state).await?;
    let base_style: SpeakingStyle =
        serde_json::from_value(style_json).map_err(|e| e.to_string())?;
    let base_traits: Vec<PersonaTrait> =
        serde_json::from_value(traits_json).map_err(|e| e.to_string())?;
    let adjustments: Vec<AppliedAdjustment> =
        serde_json::from_value(adjustments_json).map_err(|e| e.to_string())?;
    let variant = XinPersonalityEvolutionService::create_variant(
        &base_persona_id,
        &name,
        &base_style,
        &base_traits,
        &adjustments,
        &description,
    );
    Ok(ApiResponse::success(variant))
}

#[tauri::command]
pub async fn xin_v3_evolution_variant_compare(
    state: State<'_, AppState>,
    variant_a: serde_json::Value,
    variant_b: serde_json::Value,
    eval_a: serde_json::Value,
    eval_b: serde_json::Value,
) -> Result<ApiResponse<VariantCompareResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let va: PersonalityVariant =
        serde_json::from_value(variant_a).map_err(|e| e.to_string())?;
    let vb: PersonalityVariant =
        serde_json::from_value(variant_b).map_err(|e| e.to_string())?;
    let ea: Vec<EvalResult> =
        serde_json::from_value(eval_a).map_err(|e| e.to_string())?;
    let eb: Vec<EvalResult> =
        serde_json::from_value(eval_b).map_err(|e| e.to_string())?;
    let result = XinPersonalityEvolutionService::compare_variants(&va, &vb, &ea, &eb);
    Ok(ApiResponse::success(result))
}

#[tauri::command]
pub async fn xin_v3_evolution_snapshot(
    state: State<'_, AppState>,
    persona_id: String,
    variant_id: Option<String>,
    style_json: serde_json::Value,
    traits_json: serde_json::Value,
    label: String,
) -> Result<ApiResponse<EvolutionSnapshot>, String> {
    crate::commands::common::require_auth(&state).await?;
    let style: SpeakingStyle =
        serde_json::from_value(style_json).map_err(|e| e.to_string())?;
    let traits: Vec<PersonaTrait> =
        serde_json::from_value(traits_json).map_err(|e| e.to_string())?;
    let snap = XinPersonalityEvolutionService::snapshot(
        &persona_id,
        variant_id.as_deref(),
        &style,
        &traits,
        &label,
    );
    Ok(ApiResponse::success(snap))
}

#[tauri::command]
pub async fn xin_v3_evolution_timeline(
    state: State<'_, AppState>,
    persona_id: String,
    previous_json: serde_json::Value,
    new_json: serde_json::Value,
    reason: String,
    eval_results: Option<serde_json::Value>,
) -> Result<ApiResponse<EvolutionTimelineEntry>, String> {
    crate::commands::common::require_auth(&state).await?;
    let previous: SpeakingStyle =
        serde_json::from_value(previous_json).map_err(|e| e.to_string())?;
    let new: SpeakingStyle =
        serde_json::from_value(new_json).map_err(|e| e.to_string())?;
    let eval: Option<Vec<EvalResult>> = eval_results
        .map(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        .transpose()?;
    let entry = XinPersonalityEvolutionService::create_timeline_entry(
        &persona_id,
        &previous,
        &new,
        &reason,
        eval.as_deref(),
    );
    Ok(ApiResponse::success(entry))
}

#[tauri::command]
pub async fn xin_v3_proactive_pending(
    state: State<'_, AppState>,
    care_check_in: Option<String>,
    commitments: Option<serde_json::Value>,
    habits: Option<serde_json::Value>,
    persona: Option<serde_json::Value>,
) -> Result<ApiResponse<Vec<ProactiveAction>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut actions = Vec::new();

    if let Some(reason) = care_check_in {
        let p = persona
            .clone()
            .map(|v| serde_json::from_value::<crate::models::xin::Persona>(v).ok())
            .flatten()
            .unwrap_or_else(|| crate::models::xin::Persona {
                id: "default".into(),
                name: "小欣".into(),
                description: "".into(),
                traits: vec![],
                speaking_style: SpeakingStyle::default(),
                base_mood: "calm".into(),
                avatar_emoji: "🤖".into(),
                is_builtin: true,
            });
        actions.push(XinProactiveService::generate_care_action(&p, &reason));
    }

    if let Some(ref commits_json) = commitments {
        if let Ok(commits) =
            serde_json::from_value::<Vec<crate::services::xin_commit_service::CommitmentRecord>>(
                commits_json.clone(),
            )
        {
            let urgency = XinProactiveService::check_commitment_urgency(&commits);
            actions.extend(urgency);
        }
    }

    if let Some(ref habits_json) = habits {
        if let Ok(habit_list) = serde_json::from_value::<Vec<serde_json::Value>>(habits_json.clone()) {
            let reminders = XinProactiveService::build_habit_reminders(&habit_list);
            actions.extend(reminders);
        }
    }

    let sorted = XinProactiveService::get_pending_actions(actions);
    Ok(ApiResponse::success(sorted))
}

#[tauri::command]
pub async fn xin_v3_proactive_care_check(
    state: State<'_, AppState>,
    results_json: serde_json::Value,
) -> Result<ApiResponse<Option<String>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let results: Vec<EvalResult> =
        serde_json::from_value(results_json).map_err(|e| e.to_string())?;
    let reason = XinProactiveService::should_send_care_check_in(&results);
    Ok(ApiResponse::success(reason))
}

#[tauri::command]
pub async fn xin_v3_proactive_urgency(
    state: State<'_, AppState>,
    commitments_json: serde_json::Value,
) -> Result<ApiResponse<Vec<ProactiveAction>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let commitments: Vec<crate::services::xin_commit_service::CommitmentRecord> =
        serde_json::from_value(commitments_json).map_err(|e| e.to_string())?;
    let actions = XinProactiveService::check_commitment_urgency(&commitments);
    Ok(ApiResponse::success(actions))
}

#[tauri::command]
pub async fn xin_v3_proactive_quiet_hours(
    state: State<'_, AppState>,
    quiet_hours: Option<serde_json::Value>,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    let qh = quiet_hours
        .map(|v| serde_json::from_value::<QuietHours>(v).unwrap_or_default())
        .unwrap_or_default();
    Ok(ApiResponse::success(XinProactiveService::is_quiet_hours(&qh)))
}

#[tauri::command]
pub async fn xin_v3_proactive_daily_nudge(
    state: State<'_, AppState>,
    persona_json: serde_json::Value,
    commitments_json: serde_json::Value,
) -> Result<ApiResponse<Option<String>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let persona: crate::models::xin::Persona =
        serde_json::from_value(persona_json).map_err(|e| e.to_string())?;
    let commitments: Vec<crate::services::xin_commit_service::CommitmentRecord> =
        serde_json::from_value(commitments_json).map_err(|e| e.to_string())?;
    let nudge = XinProactiveService::generate_daily_nudge(&persona, &commitments);
    Ok(ApiResponse::success(nudge))
}

#[tauri::command]
pub async fn xin_v3_proactive_morning_context(
    state: State<'_, AppState>,
    persona_json: serde_json::Value,
    commitments_json: serde_json::Value,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let persona: crate::models::xin::Persona =
        serde_json::from_value(persona_json).map_err(|e| e.to_string())?;
    let commitments: Vec<crate::services::xin_commit_service::CommitmentRecord> =
        serde_json::from_value(commitments_json).map_err(|e| e.to_string())?;
    let context = XinProactiveService::generate_morning_context(&persona, &commitments);
    Ok(ApiResponse::success(context))
}

#[tauri::command]
pub async fn xin_v3_checkpoint_save(
    state: State<'_, AppState>,
    dialogue_state: tauri::State<'_, crate::services::xin_dialogue_service::XinDialogueService>,
    conversation_id: String,
    title: Option<String>,
) -> Result<ApiResponse<CheckpointSummary>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let contexts = dialogue_state.active_contexts().await;
    let context = contexts
        .get(&conversation_id)
        .ok_or_else(|| "活跃会话不存在".to_string())?;

    let msg_count = context.messages.iter().filter(|m| m.role != crate::services::xin_context_service::ChatRole::System).count() as i64;

    let saved = XinCheckpointService::save_checkpoint(
        dialogue_state.pool(),
        user_id,
        &conversation_id,
        context,
        None,
        title.as_deref(),
        msg_count,
        context.used_tokens as i64,
    )
    .await
    .map_err(|e| e.to_string())?;

    let config = CheckpointConfig::default();
    let _ = XinCheckpointService::enforce_limit(dialogue_state.pool(), user_id, &conversation_id, &config).await;

    Ok(ApiResponse::success(CheckpointSummary {
        id: saved.id,
        conversation_id: saved.conversation_id,
        checkpoint_type: saved.checkpoint_type,
        message_count: saved.message_count,
        total_tokens: saved.total_tokens,
        title: saved.title,
        has_partial_response: saved.partial_response.is_some(),
        created_at: saved.created_at,
    }))
}

#[tauri::command]
pub async fn xin_v3_checkpoint_list(
    state: State<'_, AppState>,
    pool: tauri::State<'_, SqlitePool>,
    conversation_id: String,
) -> Result<ApiResponse<Vec<CheckpointSummary>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let list = XinCheckpointService::list_checkpoints(&pool, user_id, &conversation_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(list))
}

#[tauri::command]
pub async fn xin_v3_checkpoint_restore(
    state: State<'_, AppState>,
    pool: tauri::State<'_, SqlitePool>,
    checkpoint_id: String,
) -> Result<ApiResponse<RestoreResult>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let result = XinCheckpointService::restore_checkpoint(&pool, user_id, &checkpoint_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(result))
}

#[tauri::command]
pub async fn xin_v3_checkpoint_delete(
    state: State<'_, AppState>,
    pool: tauri::State<'_, SqlitePool>,
    checkpoint_id: String,
) -> Result<ApiResponse<String>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    XinCheckpointService::delete_checkpoint(&pool, user_id, &checkpoint_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success("已删除".to_string()))
}

#[tauri::command]
pub async fn xin_v3_checkpoint_cleanup(
    state: State<'_, AppState>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<ApiResponse<usize>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let config = CheckpointConfig::default();
    let count = XinCheckpointService::cleanup_old_checkpoints(&pool, user_id, &config)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(count))
}

#[tauri::command]
pub async fn xin_v3_review_generate(
    state: State<'_, AppState>,
    pool: tauri::State<'_, SqlitePool>,
    request: ReviewRequest,
) -> Result<ApiResponse<ConversationReview>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let review = XinConversationReviewService::generate_review(&pool, user_id, &request)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(review))
}

#[tauri::command]
pub async fn xin_v3_review_topic_trends(
    state: State<'_, AppState>,
    pool: tauri::State<'_, SqlitePool>,
    persona_id: Option<String>,
    period_type: String,
    buckets: Option<usize>,
) -> Result<ApiResponse<TopicTrendResponse>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let buckets = buckets.unwrap_or(7);
    let trends = XinConversationReviewService::get_topic_trends(
        &pool, user_id, persona_id, &period_type, buckets,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(trends))
}

#[tauri::command]
pub async fn xin_v3_review_growth_trajectory(
    state: State<'_, AppState>,
    pool: tauri::State<'_, SqlitePool>,
    persona_id: Option<String>,
    period_type: String,
    buckets: Option<usize>,
    eval_results_json: Option<serde_json::Value>,
) -> Result<ApiResponse<GrowthTrajectory>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let buckets = buckets.unwrap_or(10);
    let eval_results = if let Some(json) = eval_results_json {
        serde_json::from_value::<Vec<crate::services::xin_michelin_service::EvalResult>>(json)
            .ok()
    } else {
        None
    };
    let trajectory = XinConversationReviewService::get_growth_trajectory(
        &pool, user_id, persona_id, &period_type, buckets, eval_results,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(trajectory))
}

#[tauri::command]
pub async fn xin_v3_review_heatmap(
    state: State<'_, AppState>,
    pool: tauri::State<'_, SqlitePool>,
    persona_id: Option<String>,
    period_type: String,
) -> Result<ApiResponse<HeatmapData>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let heatmap = XinConversationReviewService::get_heatmap(&pool, user_id, persona_id, &period_type)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(heatmap))
}

#[tauri::command]
pub async fn xin_v3_compaction_get_config(
    state: State<'_, AppState>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<ApiResponse<CompactionConfig>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let config = XinCompactionService::get_config(&pool, user_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(config))
}

#[tauri::command]
pub async fn xin_v3_compaction_update_config(
    state: State<'_, AppState>,
    pool: tauri::State<'_, SqlitePool>,
    config: CompactionConfig,
) -> Result<ApiResponse<CompactionConfig>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let updated = XinCompactionService::update_config(&pool, user_id, &config)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(updated))
}

#[tauri::command]
pub async fn xin_v3_compaction_get_records(
    state: State<'_, AppState>,
    pool: tauri::State<'_, SqlitePool>,
    conversation_id: String,
    limit: Option<usize>,
) -> Result<ApiResponse<Vec<CompactionRecord>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let records = XinCompactionService::get_records(&pool, user_id, &conversation_id, limit)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(records))
}

#[tauri::command]
pub async fn xin_v3_compaction_needs_check(
    state: State<'_, AppState>,
    pool: tauri::State<'_, SqlitePool>,
    context_json: String,
) -> Result<ApiResponse<bool>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let context: ContextWindow =
        serde_json::from_str(&context_json).map_err(|e| e.to_string())?;
    let needs = XinCompactionService::needs_compaction(&pool, user_id, &context)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(needs))
}

#[tauri::command]
pub async fn xin_v3_compaction_auto(
    state: State<'_, AppState>,
    pool: tauri::State<'_, SqlitePool>,
    conversation_id: String,
    context_json: String,
    model_id: String,
) -> Result<ApiResponse<CompactionResponse>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let mut context: ContextWindow =
        serde_json::from_str(&context_json).map_err(|e| e.to_string())?;
    let response =
        XinCompactionService::auto_compact(&pool, user_id, &conversation_id, &mut context, &model_id)
            .await
            .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(response))
}

#[tauri::command]
pub async fn xin_v3_compaction_manual(
    state: State<'_, AppState>,
    pool: tauri::State<'_, SqlitePool>,
    conversation_id: String,
    context_json: String,
    model_id: String,
    guidance: Option<String>,
) -> Result<ApiResponse<CompactionResponse>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let mut context: ContextWindow =
        serde_json::from_str(&context_json).map_err(|e| e.to_string())?;
    let response = XinCompactionService::manual_compact(
        &pool,
        user_id,
        &conversation_id,
        &mut context,
        &model_id,
        guidance.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(response))
}