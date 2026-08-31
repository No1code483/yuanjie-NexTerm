use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::intelligence_cross_module::{
    ActivityLog, ActivityLogExport, ActivityLogPage, ActivityLogRecord, ActivityStats,
    BehaviorAnalysis, BehaviorPattern, BehaviorTrend, ClassifyRecommendation,
    ClassifyRecommendRequest, CommandCompletion, CommandContext, DashboardData,
    GameRecommendResult, IntelligenceSettings, JournalSmartFill, NewsSummaryResult,
    OperationTemplate, QuoteCheckResult, RealtimeStats, ResumePolishResult,
    SearchAnalysisResult, SmartTodoEnhance, Suggestion, SuggestionFeedback, SuggestionPage,
    TestConnectionRequest, TestConnectionResult, TimerSmartReminder,
    KbSummarizeRequest, KbSummarizeResult, KbTagRequest, KbTagResult,
};
use crate::services::intelligence_cross_module;
use crate::services::intelligence_v4_service;

// ========== 活动感知 ==========

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn intelligence_v4_log_activity(
    state: State<'_, AppState>,
    record: ActivityLogRecord,
) -> Result<ApiResponse<ActivityLog>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::log_activity(&state.pool, record)
        .await
        .map_err(|e| e.to_string())
}

// ========== 跨模块智能 ==========

#[tauri::command]
pub async fn intelligence_v4_resume_spell_check(
    state: State<'_, AppState>,
    content: String,
) -> Result<ApiResponse<ResumePolishResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_cross_module::resume_spell_check(content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_resume_polish(
    state: State<'_, AppState>,
    content: String,
) -> Result<ApiResponse<ResumePolishResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_cross_module::resume_polish(content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_resume_generate(
    state: State<'_, AppState>,
    name: Option<String>,
    education: Option<String>,
    skills: Option<Vec<String>>,
    experience: Option<Vec<String>>,
) -> Result<ApiResponse<ResumePolishResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_cross_module::resume_generate_template(name, education, skills, experience)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_quote_spell_check(
    state: State<'_, AppState>,
    content: String,
) -> Result<ApiResponse<QuoteCheckResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_cross_module::quote_spell_check(content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_quote_source_verify(
    state: State<'_, AppState>,
    author: Option<String>,
    source: Option<String>,
) -> Result<ApiResponse<QuoteCheckResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_cross_module::quote_source_verify(author, source)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_quote_smart_complete(
    state: State<'_, AppState>,
    content: String,
    author: Option<String>,
    source: Option<String>,
) -> Result<ApiResponse<QuoteCheckResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_cross_module::quote_smart_complete(content, author, source)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_news_summarize(
    state: State<'_, AppState>,
    title: String,
    content: String,
) -> Result<ApiResponse<NewsSummaryResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_cross_module::news_generate_summary(title, content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_todo_enhance(
    state: State<'_, AppState>,
    title: String,
    description: Option<String>,
) -> Result<ApiResponse<SmartTodoEnhance>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_cross_module::todo_smart_enhance(title, description)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_journal_fill(
    state: State<'_, AppState>,
    today_events: Option<String>,
) -> Result<ApiResponse<JournalSmartFill>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_cross_module::journal_smart_fill(today_events)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_timer_remind(
    state: State<'_, AppState>,
    current_elapsed_secs: i64,
    task_type: Option<String>,
) -> Result<ApiResponse<TimerSmartReminder>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_cross_module::timer_smart_remind(current_elapsed_secs, task_type)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_kb_classify(
    state: State<'_, AppState>,
    request: ClassifyRecommendRequest,
) -> Result<ApiResponse<Vec<ClassifyRecommendation>>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_cross_module::kb_classify_recommend(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_terminal_complete(
    state: State<'_, AppState>,
    context: CommandContext,
) -> Result<ApiResponse<Vec<CommandCompletion>>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_cross_module::terminal_command_suggest(context)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_game_recommend(
    state: State<'_, AppState>,
    play_duration_today_secs: Option<i64>,
    current_time_hour: Option<i64>,
    recent_games: Option<Vec<String>>,
) -> Result<ApiResponse<GameRecommendResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_cross_module::game_smart_recommend(
        play_duration_today_secs,
        current_time_hour,
        recent_games,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_search_analyze(
    state: State<'_, AppState>,
    query: String,
) -> Result<ApiResponse<SearchAnalysisResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_cross_module::search_smart_analyze(query)
        .map_err(|e| e.to_string())
}

// ========== 设置管理 ==========

async fn resolve_uid(state: &State<'_, AppState>, user_id: Option<String>) -> String {
    if let Some(id) = user_id.filter(|s| !s.is_empty()) {
        return id;
    }
    state.current_user.read().await
        .map(|id| id.to_string())
        .unwrap_or_else(|| "default".to_string())
}

#[tauri::command]
pub async fn intelligence_v4_get_settings(
    state: State<'_, AppState>,
    user_id: Option<String>,
) -> Result<ApiResponse<IntelligenceSettings>, String> {
    crate::commands::common::require_auth(&state).await?;
    let uid = resolve_uid(&state, user_id).await;
    intelligence_v4_service::get_settings(&state.pool, uid)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_save_settings(
    state: State<'_, AppState>,
    mut settings: IntelligenceSettings,
) -> Result<ApiResponse<IntelligenceSettings>, String> {
    crate::commands::common::require_auth(&state).await?;
    if settings.user_id.is_empty() {
        settings.user_id = resolve_uid(&state, None).await;
    }
    tracing::info!(user_id = %settings.user_id, extra_config = ?settings.extra_config, "保存底层智能设置");
    intelligence_v4_service::save_settings(&state.pool, settings)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_reset_settings(
    state: State<'_, AppState>,
    user_id: Option<String>,
) -> Result<ApiResponse<IntelligenceSettings>, String> {
    crate::commands::common::require_auth(&state).await?;
    let uid = resolve_uid(&state, user_id).await;
    intelligence_v4_service::reset_settings(&state.pool, uid)
        .await
        .map_err(|e| e.to_string())
}

// ========== 连接测试 ==========

#[tauri::command]
pub async fn intelligence_v4_test_connection(
    state: State<'_, AppState>,
    request: TestConnectionRequest,
) -> Result<ApiResponse<TestConnectionResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::test_connection(
        &request.provider,
        &request.endpoint,
        request.api_key.as_deref(),
        request.model.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

// ========== 活动感知增强 ==========

#[tauri::command]
pub async fn intelligence_v4_export_activity_logs(
    state: State<'_, AppState>,
    user_id: Option<String>,
    module: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
) -> Result<ApiResponse<ActivityLogExport>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::export_activity_logs(
        &state.pool,
        user_id,
        module,
        start_time,
        end_time,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_get_operation_templates(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<OperationTemplate>>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::get_operation_templates()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_log_with_template(
    state: State<'_, AppState>,
    user_id: String,
    module: String,
    template: String,
    detail: Option<String>,
    remark: Option<String>,
    duration_secs: Option<i64>,
) -> Result<ApiResponse<ActivityLog>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::log_with_template(
        &state.pool,
        user_id,
        module,
        template,
        detail,
        remark,
        duration_secs,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_batch_log_activity(
    state: State<'_, AppState>,
    records: Vec<ActivityLogRecord>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::batch_log_activity(&state.pool, records)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_query_activity_logs(
    state: State<'_, AppState>,
    user_id: Option<String>,
    module: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<ApiResponse<ActivityLogPage>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::query_activity_logs(
        &state.pool,
        user_id,
        module,
        start_time,
        end_time,
        limit,
        offset,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_activity_stats(
    state: State<'_, AppState>,
    user_id: String,
    period: Option<String>,
) -> Result<ApiResponse<ActivityStats>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::get_activity_stats(
        &state.pool,
        user_id,
        period.unwrap_or_else(|| "today".to_string()),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_clean_activity_logs(
    state: State<'_, AppState>,
    retention_days: Option<i64>,
) -> Result<ApiResponse<u64>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::clean_activity_logs(
        &state.pool,
        retention_days.unwrap_or(30),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_clear_all_activity_logs(
    state: State<'_, AppState>,
) -> Result<ApiResponse<u64>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    // IDOR 修复：将 user_id 转为 String 传入 service，仅删除当前用户的活动日志
    intelligence_v4_service::clear_all_activity_logs(&state.pool, &user_id.to_string())
        .await
        .map_err(|e| e.to_string())
}

// ========== 仪表盘 ==========

#[tauri::command]
pub async fn intelligence_v4_dashboard(
    state: State<'_, AppState>,
    user_id: String,
    period: Option<String>,
) -> Result<ApiResponse<DashboardData>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::get_dashboard(
        &state.pool,
        user_id,
        period.unwrap_or_else(|| "weekly".to_string()),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_realtime_stats(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<ApiResponse<RealtimeStats>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::get_realtime_stats(&state.pool, user_id)
        .await
        .map_err(|e| e.to_string())
}

// ========== 智能建议 ==========

#[tauri::command]
pub async fn intelligence_v4_generate_suggestions(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<ApiResponse<Vec<Suggestion>>, String> {
    crate::commands::common::require_auth(&state).await?;
    // D2.6 可关闭性：关闭底层智能后返回空建议列表（非阻塞，各模块核心功能仍可用）
    // 设计意图 §8.2：关闭后所有模块核心功能必须可用
    if !state.intelligence_service.is_enabled().await {
        return Ok(ApiResponse::success(Vec::new()));
    }
    intelligence_v4_service::generate_suggestions(&state.pool, user_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_get_suggestions(
    state: State<'_, AppState>,
    user_id: String,
    status: Option<String>,
    category: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<ApiResponse<SuggestionPage>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::query_suggestions(&state.pool, user_id, status, category, limit, offset)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_mark_suggestion(
    state: State<'_, AppState>,
    suggestion_id: i64,
    action: String,
) -> Result<ApiResponse<()>, String> {
    // IDOR 修复：使用 user_id 进行资源所有权验证
    let user_id = crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::mark_suggestion(
        &state.pool,
        SuggestionFeedback { suggestion_id, action },
        &user_id.to_string(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_clean_suggestions(
    state: State<'_, AppState>,
    retention_days: Option<i64>,
) -> Result<ApiResponse<u64>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::clean_suggestions(&state.pool, retention_days.unwrap_or(30))
        .await
        .map_err(|e| e.to_string())
}

// ========== 行为分析 ==========

#[tauri::command]
pub async fn intelligence_v4_analyze_behavior(
    state: State<'_, AppState>,
    user_id: String,
    date: Option<String>,
) -> Result<ApiResponse<BehaviorAnalysis>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::analyze_behavior(&state.pool, user_id, date)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_behavior_trend(
    state: State<'_, AppState>,
    user_id: String,
    days: Option<i64>,
) -> Result<ApiResponse<BehaviorTrend>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::get_behavior_trend(&state.pool, user_id, days)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_behavior_history(
    state: State<'_, AppState>,
    user_id: String,
    start_date: Option<String>,
    end_date: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<ApiResponse<(Vec<BehaviorPattern>, i64)>, String> {
    crate::commands::common::require_auth(&state).await?;
    intelligence_v4_service::query_behavior_history(
        &state.pool,
        user_id,
        start_date,
        end_date,
        limit,
        offset,
    )
    .await
    .map_err(|e| e.to_string())
}

// ========== 知识库智能体 ==========

#[tauri::command]
pub async fn intelligence_v4_kb_summarize(
    state: State<'_, AppState>,
    request: KbSummarizeRequest,
) -> Result<ApiResponse<KbSummarizeResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    // D2.6 可关闭性：KB 智能体（LLM 摘要）属于跨模块 AI 增强，关闭底层智能后不可用
    // 设计意图 §8.2：KB CRUD 核心功能不受影响，仅 AI 增强被关闭
    if !state.intelligence_service.is_enabled().await {
        return Err("底层智能已关闭，无法使用 AI 摘要功能。请在设置中开启底层智能。".into());
    }
    intelligence_v4_service::kb_summarize(request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v4_kb_tags(
    state: State<'_, AppState>,
    request: KbTagRequest,
) -> Result<ApiResponse<KbTagResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    // D2.6 可关闭性：KB 智能体（LLM 标签）属于跨模块 AI 增强，关闭底层智能后不可用
    if !state.intelligence_service.is_enabled().await {
        return Err("底层智能已关闭，无法使用 AI 标签推荐功能。请在设置中开启底层智能。".into());
    }
    intelligence_v4_service::kb_tags(request)
        .await
        .map_err(|e| e.to_string())
}