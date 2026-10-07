//! customs.intelligence 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发。
//!
//! alias 只做旧名 → 逻辑名的映射登记；真实 handler 在
//! commands::dispatch_legacy —— 闭包仅捕获 AppHandle，运行时经
//! handle.state::<AppState>() 解析主应用真实 AppState（契约 06_Rust代码契窍 §8.1），
//! 业务实现复用 commands/intelligence_*_commands.rs 原函数，返回值序列化与旧 IPC 路径一致。
//!
//! 批次6c S1：74 intelligence 命令登记为 alias（intelligence 25 + v4 38 + behavior 6 + anomaly 5）。

pub const PLUGIN_ID: &str = "customs.intelligence";
pub const SHORT_CODE: &str = "sp";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcAlias {
    pub plugin_id: &'static str,
    pub short_code: &'static str,
    pub legacy_command: &'static str,
    pub new_command: &'static str,
}

macro_rules! alias {
    ($command:literal) => {
        IpcAlias {
            plugin_id: PLUGIN_ID,
            short_code: SHORT_CODE,
            legacy_command: $command,
            new_command: concat!("sp:plugin:", $command),
        }
    };
}

/// 74 条 alias：intelligence 25 + v4 38 + behavior 6 + anomaly 5 命令。
pub const IPC_ALIASES: &[IpcAlias] = &[
    // ===== intelligence v1 (25) =====
    alias!("intelligence_get_config"),
    alias!("intelligence_get_enabled"),
    alias!("intelligence_set_enabled"),
    alias!("intelligence_trigger_proactive_actions"),
    alias!("intelligence_set_config"),
    alias!("intelligence_get_context"),
    alias!("intelligence_track_activity"),
    alias!("intelligence_set_active_file"),
    alias!("intelligence_add_terminal_session"),
    alias!("intelligence_remove_terminal_session"),
    alias!("intelligence_set_window_title"),
    alias!("intelligence_get_ollama_status"),
    alias!("intelligence_get_suggestions"),
    alias!("intelligence_take_snapshot"),
    alias!("intelligence_get_snapshots"),
    alias!("intelligence_query_local_llm"),
    alias!("intelligence_terminal_suggest"),
    alias!("intelligence_resume_polish"),
    alias!("intelligence_news_summary"),
    alias!("intelligence_todo_suggest"),
    alias!("intelligence_timer_remind"),
    alias!("intelligence_kb_classify"),
    alias!("intelligence_daily_briefing"),
    alias!("intelligence_chat_suggest"),
    alias!("intelligence_game_suggest"),
    // ===== intelligence_v4 (38) =====
    alias!("intelligence_v4_log_activity"),
    alias!("intelligence_v4_resume_spell_check"),
    alias!("intelligence_v4_resume_polish"),
    alias!("intelligence_v4_resume_generate"),
    alias!("intelligence_v4_quote_spell_check"),
    alias!("intelligence_v4_quote_source_verify"),
    alias!("intelligence_v4_quote_smart_complete"),
    alias!("intelligence_v4_news_summarize"),
    alias!("intelligence_v4_todo_enhance"),
    alias!("intelligence_v4_journal_fill"),
    alias!("intelligence_v4_timer_remind"),
    alias!("intelligence_v4_kb_classify"),
    alias!("intelligence_v4_terminal_complete"),
    alias!("intelligence_v4_game_recommend"),
    alias!("intelligence_v4_search_analyze"),
    alias!("intelligence_v4_get_settings"),
    alias!("intelligence_v4_save_settings"),
    alias!("intelligence_v4_reset_settings"),
    alias!("intelligence_v4_test_connection"),
    alias!("intelligence_v4_export_activity_logs"),
    alias!("intelligence_v4_get_operation_templates"),
    alias!("intelligence_v4_log_with_template"),
    alias!("intelligence_v4_batch_log_activity"),
    alias!("intelligence_v4_query_activity_logs"),
    alias!("intelligence_v4_activity_stats"),
    alias!("intelligence_v4_clean_activity_logs"),
    alias!("intelligence_v4_clear_all_activity_logs"),
    alias!("intelligence_v4_dashboard"),
    alias!("intelligence_v4_realtime_stats"),
    alias!("intelligence_v4_generate_suggestions"),
    alias!("intelligence_v4_get_suggestions"),
    alias!("intelligence_v4_mark_suggestion"),
    alias!("intelligence_v4_clean_suggestions"),
    alias!("intelligence_v4_analyze_behavior"),
    alias!("intelligence_v4_behavior_trend"),
    alias!("intelligence_v4_behavior_history"),
    alias!("intelligence_v4_kb_summarize"),
    alias!("intelligence_v4_kb_tags"),
    // ===== intelligence_behavior v2 (6) =====
    alias!("intelligence_v2_analyze_behavior"),
    alias!("intelligence_v2_recommend_workflows"),
    alias!("intelligence_v2_detect_tech_stack"),
    alias!("intelligence_v2_cognitive_load"),
    alias!("intelligence_v2_notifications"),
    alias!("intelligence_v2_dashboard"),
    // ===== intelligence_anomaly v3 (5) =====
    alias!("intelligence_v3_detect_anomaly"),
    alias!("intelligence_v3_snapshot_resources"),
    alias!("intelligence_v3_organize_knowledge"),
    alias!("intelligence_v3_list_scheduled_tasks"),
    alias!("intelligence_v3_execute_scheduled_tasks"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::{
    intelligence_anomaly_commands, intelligence_behavior_commands, intelligence_commands,
    intelligence_v4_commands,
};
use crate::db::connection::AppState;
use crate::models::intelligence::{
    ChatModuleContext, GameModuleContext, IntelligenceConfig, UserActivity,
};
use crate::models::intelligence_cross_module::{
    ActivityLogRecord, CommandContext, IntelligenceSettings, KbSummarizeRequest, KbTagRequest,
    TestConnectionRequest,
};

fn arg_str(args: &Json, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Json::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("参数 {key} 缺失或不是字符串"))
}

fn arg_opt_str(args: &Json, key: &str) -> Result<Option<String>, String> {
    match args.get(key) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => {
            let v = v.as_str().ok_or_else(|| format!("参数 {key} 不是字符串"))?;
            Ok(Some(v.to_string()))
        }
        None => Ok(None),
    }
}

fn arg_bool(args: &Json, key: &str) -> Result<bool, String> {
    args.get(key)
        .and_then(Json::as_bool)
        .ok_or_else(|| format!("参数 {key} 缺失或不是布尔值"))
}

fn arg_i64(args: &Json, key: &str) -> Result<i64, String> {
    args.get(key)
        .and_then(Json::as_i64)
        .ok_or_else(|| format!("参数 {key} 缺失或不是整数"))
}

fn arg_opt_i64(args: &Json, key: &str) -> Result<Option<i64>, String> {
    match args.get(key) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => {
            let v = v.as_i64().ok_or_else(|| format!("参数 {key} 不是整数"))?;
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

fn arg_opt_u64(args: &Json, key: &str) -> Result<Option<u64>, String> {
    match args.get(key) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => {
            let v = v.as_u64().ok_or_else(|| format!("参数 {key} 不是整数"))?;
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

fn arg_opt_vec_str(args: &Json, key: &str) -> Result<Option<Vec<String>>, String> {
    match args.get(key) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => {
            let vec: Vec<String> = serde_json::from_value(v.clone())
                .map_err(|e| format!("参数 {key} 反序列化失败: {}", e))?;
            Ok(Some(vec))
        }
        None => Ok(None),
    }
}

fn arg_json<T: serde::de::DeserializeOwned>(args: &Json, key: &str) -> Result<T, String> {
    args.get(key)
        .ok_or_else(|| format!("参数 {key} 缺失"))
        .and_then(|v| {
            serde_json::from_value(v.clone())
                .map_err(|e| format!("参数 {key} 反序列化失败: {}", e))
        })
}

fn to_json<T: serde::Serialize>(value: T) -> Result<Json, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

/// 逐条分发到旧命令函数。legacy 为 IPC_ALIASES.legacy_command 值。
/// intelligence_*_commands 使用 #[tauri::command] + State<'_, AppState> + require_auth 模式。
/// 直接调用原函数复用 require_auth。
pub(crate) async fn dispatch_legacy<R: Runtime>(
    handle: AppHandle<R>,
    legacy: &'static str,
    args: Json,
) -> Result<Json, String> {
    let state = handle.state::<AppState>();

    match legacy {
        // ===== intelligence v1 (25) =====
        "intelligence_get_config" => {
            to_json(intelligence_commands::intelligence_get_config(state).await?)
        }
        "intelligence_get_enabled" => {
            to_json(intelligence_commands::intelligence_get_enabled(state).await?)
        }
        "intelligence_set_enabled" => {
            let enabled = arg_bool(&args, "enabled")?;
            to_json(intelligence_commands::intelligence_set_enabled(state, enabled).await?)
        }
        "intelligence_trigger_proactive_actions" => {
            to_json(
                intelligence_commands::intelligence_trigger_proactive_actions(state).await?,
            )
        }
        "intelligence_set_config" => {
            let config: IntelligenceConfig = arg_json(&args, "config")?;
            to_json(intelligence_commands::intelligence_set_config(state, config).await?)
        }
        "intelligence_get_context" => {
            to_json(intelligence_commands::intelligence_get_context(state).await?)
        }
        "intelligence_track_activity" => {
            let activity: UserActivity = arg_json(&args, "activity")?;
            to_json(intelligence_commands::intelligence_track_activity(state, activity).await?)
        }
        "intelligence_set_active_file" => {
            let path = arg_opt_str(&args, "path")?;
            let language = arg_opt_str(&args, "language")?;
            to_json(
                intelligence_commands::intelligence_set_active_file(state, path, language).await?,
            )
        }
        "intelligence_add_terminal_session" => {
            let session_id = arg_str(&args, "sessionId")?;
            to_json(
                intelligence_commands::intelligence_add_terminal_session(state, session_id).await?,
            )
        }
        "intelligence_remove_terminal_session" => {
            let session_id = arg_str(&args, "sessionId")?;
            to_json(
                intelligence_commands::intelligence_remove_terminal_session(state, session_id)
                    .await?,
            )
        }
        "intelligence_set_window_title" => {
            let title = arg_opt_str(&args, "title")?;
            to_json(
                intelligence_commands::intelligence_set_window_title(state, title).await?,
            )
        }
        "intelligence_get_ollama_status" => {
            to_json(intelligence_commands::intelligence_get_ollama_status(state).await?)
        }
        "intelligence_get_suggestions" => {
            to_json(intelligence_commands::intelligence_get_suggestions(state).await?)
        }
        "intelligence_take_snapshot" => {
            to_json(intelligence_commands::intelligence_take_snapshot(state).await?)
        }
        "intelligence_get_snapshots" => {
            to_json(intelligence_commands::intelligence_get_snapshots(state).await?)
        }
        "intelligence_query_local_llm" => {
            let prompt = arg_str(&args, "prompt")?;
            to_json(
                intelligence_commands::intelligence_query_local_llm(state, prompt).await?,
            )
        }
        "intelligence_terminal_suggest" => {
            let prompt = arg_str(&args, "prompt")?;
            to_json(
                intelligence_commands::intelligence_terminal_suggest(state, prompt).await?,
            )
        }
        "intelligence_resume_polish" => {
            let text = arg_str(&args, "text")?;
            let section = arg_opt_str(&args, "section")?;
            to_json(
                intelligence_commands::intelligence_resume_polish(state, text, section).await?,
            )
        }
        "intelligence_news_summary" => {
            let title = arg_str(&args, "title")?;
            let content = arg_str(&args, "content")?;
            let url = arg_opt_str(&args, "url")?;
            to_json(
                intelligence_commands::intelligence_news_summary(state, title, content, url)
                    .await?,
            )
        }
        "intelligence_todo_suggest" => {
            let partial_text = arg_str(&args, "partialText")?;
            to_json(
                intelligence_commands::intelligence_todo_suggest(state, partial_text).await?,
            )
        }
        "intelligence_timer_remind" => {
            let timer_state = arg_str(&args, "timerState")?;
            to_json(
                intelligence_commands::intelligence_timer_remind(state, timer_state).await?,
            )
        }
        "intelligence_kb_classify" => {
            let entry_id = arg_opt_i64(&args, "entryId")?;
            let content = arg_str(&args, "content")?;
            to_json(
                intelligence_commands::intelligence_kb_classify(state, entry_id, content)
                    .await?,
            )
        }
        "intelligence_daily_briefing" => {
            to_json(intelligence_commands::intelligence_daily_briefing(state).await?)
        }
        "intelligence_chat_suggest" => {
            let context: ChatModuleContext = arg_json(&args, "context")?;
            to_json(
                intelligence_commands::intelligence_chat_suggest(state, context).await?,
            )
        }
        "intelligence_game_suggest" => {
            let context: GameModuleContext = arg_json(&args, "context")?;
            to_json(
                intelligence_commands::intelligence_game_suggest(state, context).await?,
            )
        }

        // ===== intelligence_v4 (38) =====
        "intelligence_v4_log_activity" => {
            let record: ActivityLogRecord = arg_json(&args, "record")?;
            to_json(intelligence_v4_commands::intelligence_v4_log_activity(state, record).await?)
        }
        "intelligence_v4_resume_spell_check" => {
            let content = arg_str(&args, "content")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_resume_spell_check(state, content)
                    .await?,
            )
        }
        "intelligence_v4_resume_polish" => {
            let content = arg_str(&args, "content")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_resume_polish(state, content).await?,
            )
        }
        "intelligence_v4_resume_generate" => {
            let name = arg_opt_str(&args, "name")?;
            let education = arg_opt_str(&args, "education")?;
            let skills = arg_opt_vec_str(&args, "skills")?;
            let experience = arg_opt_vec_str(&args, "experience")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_resume_generate(
                    state, name, education, skills, experience,
                )
                .await?,
            )
        }
        "intelligence_v4_quote_spell_check" => {
            let content = arg_str(&args, "content")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_quote_spell_check(state, content)
                    .await?,
            )
        }
        "intelligence_v4_quote_source_verify" => {
            let author = arg_opt_str(&args, "author")?;
            let source = arg_opt_str(&args, "source")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_quote_source_verify(state, author, source)
                    .await?,
            )
        }
        "intelligence_v4_quote_smart_complete" => {
            let content = arg_str(&args, "content")?;
            let author = arg_opt_str(&args, "author")?;
            let source = arg_opt_str(&args, "source")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_quote_smart_complete(state, content, author, source)
                    .await?,
            )
        }
        "intelligence_v4_news_summarize" => {
            let title = arg_str(&args, "title")?;
            let content = arg_str(&args, "content")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_news_summarize(state, title, content)
                    .await?,
            )
        }
        "intelligence_v4_todo_enhance" => {
            let title = arg_str(&args, "title")?;
            let description = arg_opt_str(&args, "description")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_todo_enhance(state, title, description)
                    .await?,
            )
        }
        "intelligence_v4_journal_fill" => {
            let today_events = arg_opt_str(&args, "todayEvents")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_journal_fill(state, today_events)
                    .await?,
            )
        }
        "intelligence_v4_timer_remind" => {
            let current_elapsed_secs = arg_i64(&args, "currentElapsedSecs")?;
            let task_type = arg_opt_str(&args, "taskType")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_timer_remind(
                    state, current_elapsed_secs, task_type,
                )
                    .await?,
            )
        }
        "intelligence_v4_kb_classify" => {
            let request: crate::models::intelligence_cross_module::ClassifyRecommendRequest =
                arg_json(&args, "request")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_kb_classify(state, request).await?,
            )
        }
        "intelligence_v4_terminal_complete" => {
            let context: CommandContext = arg_json(&args, "context")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_terminal_complete(state, context)
                    .await?,
            )
        }
        "intelligence_v4_game_recommend" => {
            let play_duration_today_secs = arg_opt_i64(&args, "playDurationTodaySecs")?;
            let current_time_hour = arg_opt_i64(&args, "currentTimeHour")?;
            let recent_games = arg_opt_vec_str(&args, "recentGames")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_game_recommend(
                    state, play_duration_today_secs, current_time_hour, recent_games,
                )
                    .await?,
            )
        }
        "intelligence_v4_search_analyze" => {
            let query = arg_str(&args, "query")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_search_analyze(state, query).await?,
            )
        }
        "intelligence_v4_get_settings" => {
            let user_id = arg_opt_str(&args, "userId")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_get_settings(state, user_id).await?,
            )
        }
        "intelligence_v4_save_settings" => {
            let settings: IntelligenceSettings = arg_json(&args, "settings")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_save_settings(state, settings).await?,
            )
        }
        "intelligence_v4_reset_settings" => {
            let user_id = arg_opt_str(&args, "userId")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_reset_settings(state, user_id).await?,
            )
        }
        "intelligence_v4_test_connection" => {
            let request: TestConnectionRequest = arg_json(&args, "request")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_test_connection(state, request)
                    .await?,
            )
        }
        "intelligence_v4_export_activity_logs" => {
            let user_id = arg_opt_str(&args, "userId")?;
            let module = arg_opt_str(&args, "module")?;
            let start_time = arg_opt_str(&args, "startTime")?;
            let end_time = arg_opt_str(&args, "endTime")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_export_activity_logs(
                    state, user_id, module, start_time, end_time,
                )
                    .await?,
            )
        }
        "intelligence_v4_get_operation_templates" => {
            to_json(
                intelligence_v4_commands::intelligence_v4_get_operation_templates(state).await?,
            )
        }
        "intelligence_v4_log_with_template" => {
            let user_id = arg_str(&args, "userId")?;
            let module = arg_str(&args, "module")?;
            let template = arg_str(&args, "template")?;
            let detail = arg_opt_str(&args, "detail")?;
            let remark = arg_opt_str(&args, "remark")?;
            let duration_secs = arg_opt_i64(&args, "durationSecs")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_log_with_template(
                    state, user_id, module, template, detail, remark, duration_secs,
                )
                    .await?,
            )
        }
        "intelligence_v4_batch_log_activity" => {
            let records: Vec<ActivityLogRecord> = arg_json(&args, "records")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_batch_log_activity(state, records)
                    .await?,
            )
        }
        "intelligence_v4_query_activity_logs" => {
            let user_id = arg_opt_str(&args, "userId")?;
            let module = arg_opt_str(&args, "module")?;
            let start_time = arg_opt_str(&args, "startTime")?;
            let end_time = arg_opt_str(&args, "endTime")?;
            let limit = arg_opt_i64(&args, "limit")?;
            let offset = arg_opt_i64(&args, "offset")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_query_activity_logs(
                    state, user_id, module, start_time, end_time, limit, offset,
                )
                    .await?,
            )
        }
        "intelligence_v4_activity_stats" => {
            let user_id = arg_str(&args, "userId")?;
            let period = arg_opt_str(&args, "period")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_activity_stats(state, user_id, period)
                    .await?,
            )
        }
        "intelligence_v4_clean_activity_logs" => {
            let retention_days = arg_opt_i64(&args, "retentionDays")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_clean_activity_logs(state, retention_days)
                    .await?,
            )
        }
        "intelligence_v4_clear_all_activity_logs" => {
            to_json(
                intelligence_v4_commands::intelligence_v4_clear_all_activity_logs(state).await?,
            )
        }
        "intelligence_v4_dashboard" => {
            let user_id = arg_str(&args, "userId")?;
            let period = arg_opt_str(&args, "period")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_dashboard(state, user_id, period).await?,
            )
        }
        "intelligence_v4_realtime_stats" => {
            let user_id = arg_str(&args, "userId")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_realtime_stats(state, user_id)
                    .await?,
            )
        }
        "intelligence_v4_generate_suggestions" => {
            to_json(
                intelligence_v4_commands::intelligence_v4_generate_suggestions(state).await?,
            )
        }
        "intelligence_v4_get_suggestions" => {
            let user_id = arg_str(&args, "userId")?;
            let status = arg_opt_str(&args, "status")?;
            let category = arg_opt_str(&args, "category")?;
            let limit = arg_opt_i64(&args, "limit")?;
            let offset = arg_opt_i64(&args, "offset")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_get_suggestions(
                    state, user_id, status, category, limit, offset,
                )
                    .await?,
            )
        }
        "intelligence_v4_mark_suggestion" => {
            let suggestion_id = arg_i64(&args, "suggestionId")?;
            let action = arg_str(&args, "action")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_mark_suggestion(state, suggestion_id, action)
                    .await?,
            )
        }
        "intelligence_v4_clean_suggestions" => {
            let retention_days = arg_opt_i64(&args, "retentionDays")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_clean_suggestions(state, retention_days)
                    .await?,
            )
        }
        "intelligence_v4_analyze_behavior" => {
            let user_id = arg_str(&args, "userId")?;
            let date = arg_opt_str(&args, "date")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_analyze_behavior(state, user_id, date)
                    .await?,
            )
        }
        "intelligence_v4_behavior_trend" => {
            let user_id = arg_str(&args, "userId")?;
            let days = arg_opt_i64(&args, "days")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_behavior_trend(state, user_id, days)
                    .await?,
            )
        }
        "intelligence_v4_behavior_history" => {
            let user_id = arg_str(&args, "userId")?;
            let start_date = arg_opt_str(&args, "startDate")?;
            let end_date = arg_opt_str(&args, "endDate")?;
            let limit = arg_opt_i64(&args, "limit")?;
            let offset = arg_opt_i64(&args, "offset")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_behavior_history(
                    state, user_id, start_date, end_date, limit, offset,
                )
                    .await?,
            )
        }
        "intelligence_v4_kb_summarize" => {
            let request: KbSummarizeRequest = arg_json(&args, "request")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_kb_summarize(state, request).await?,
            )
        }
        "intelligence_v4_kb_tags" => {
            let request: KbTagRequest = arg_json(&args, "request")?;
            to_json(
                intelligence_v4_commands::intelligence_v4_kb_tags(state, request).await?,
            )
        }

        // ===== intelligence_behavior v2 (6) =====
        "intelligence_v2_analyze_behavior" => {
            to_json(intelligence_behavior_commands::intelligence_v2_analyze_behavior(state).await?)
        }
        "intelligence_v2_recommend_workflows" => {
            let current_context = arg_opt_str(&args, "currentContext")?;
            to_json(
                intelligence_behavior_commands::intelligence_v2_recommend_workflows(state, current_context)
                    .await?,
            )
        }
        "intelligence_v2_detect_tech_stack" => {
            let project_dir = arg_str(&args, "projectDir")?;
            to_json(
                intelligence_behavior_commands::intelligence_v2_detect_tech_stack(state, project_dir)
                    .await?,
            )
        }
        "intelligence_v2_cognitive_load" => {
            let window_secs = arg_opt_u64(&args, "windowSecs")?;
            to_json(
                intelligence_behavior_commands::intelligence_v2_cognitive_load(state, window_secs)
                    .await?,
            )
        }
        "intelligence_v2_notifications" => {
            to_json(
                intelligence_behavior_commands::intelligence_v2_notifications(state).await?,
            )
        }
        "intelligence_v2_dashboard" => {
            let period = arg_opt_str(&args, "period")?;
            to_json(
                intelligence_behavior_commands::intelligence_v2_dashboard(state, period).await?,
            )
        }

        // ===== intelligence_anomaly v3 (5) =====
        "intelligence_v3_detect_anomaly" => {
            let window_secs = arg_opt_u64(&args, "windowSecs")?;
            to_json(
                intelligence_anomaly_commands::intelligence_v3_detect_anomaly(state, window_secs)
                    .await?,
            )
        }
        "intelligence_v3_snapshot_resources" => {
            to_json(
                intelligence_anomaly_commands::intelligence_v3_snapshot_resources(state).await?,
            )
        }
        "intelligence_v3_organize_knowledge" => {
            let entries_json = arg_str(&args, "entriesJson")?;
            let categories_json = arg_str(&args, "categoriesJson")?;
            to_json(
                intelligence_anomaly_commands::intelligence_v3_organize_knowledge(state, entries_json, categories_json)
                    .await?,
            )
        }
        "intelligence_v3_list_scheduled_tasks" => {
            to_json(
                intelligence_anomaly_commands::intelligence_v3_list_scheduled_tasks(state).await?,
            )
        }
        "intelligence_v3_execute_scheduled_tasks" => {
            to_json(
                intelligence_anomaly_commands::intelligence_v3_execute_scheduled_tasks(state)
                    .await?,
            )
        }

        _ => Err(format!("未知 intelligence 逻辑命令: {legacy}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        // intelligence v1 (25)
        "intelligence_get_config",
        "intelligence_get_enabled",
        "intelligence_set_enabled",
        "intelligence_trigger_proactive_actions",
        "intelligence_set_config",
        "intelligence_get_context",
        "intelligence_track_activity",
        "intelligence_set_active_file",
        "intelligence_add_terminal_session",
        "intelligence_remove_terminal_session",
        "intelligence_set_window_title",
        "intelligence_get_ollama_status",
        "intelligence_get_suggestions",
        "intelligence_take_snapshot",
        "intelligence_get_snapshots",
        "intelligence_query_local_llm",
        "intelligence_terminal_suggest",
        "intelligence_resume_polish",
        "intelligence_news_summary",
        "intelligence_todo_suggest",
        "intelligence_timer_remind",
        "intelligence_kb_classify",
        "intelligence_daily_briefing",
        "intelligence_chat_suggest",
        "intelligence_game_suggest",
        // intelligence_v4 (38)
        "intelligence_v4_log_activity",
        "intelligence_v4_resume_spell_check",
        "intelligence_v4_resume_polish",
        "intelligence_v4_resume_generate",
        "intelligence_v4_quote_spell_check",
        "intelligence_v4_quote_source_verify",
        "intelligence_v4_quote_smart_complete",
        "intelligence_v4_news_summarize",
        "intelligence_v4_todo_enhance",
        "intelligence_v4_journal_fill",
        "intelligence_v4_timer_remind",
        "intelligence_v4_kb_classify",
        "intelligence_v4_terminal_complete",
        "intelligence_v4_game_recommend",
        "intelligence_v4_search_analyze",
        "intelligence_v4_get_settings",
        "intelligence_v4_save_settings",
        "intelligence_v4_reset_settings",
        "intelligence_v4_test_connection",
        "intelligence_v4_export_activity_logs",
        "intelligence_v4_get_operation_templates",
        "intelligence_v4_log_with_template",
        "intelligence_v4_batch_log_activity",
        "intelligence_v4_query_activity_logs",
        "intelligence_v4_activity_stats",
        "intelligence_v4_clean_activity_logs",
        "intelligence_v4_clear_all_activity_logs",
        "intelligence_v4_dashboard",
        "intelligence_v4_realtime_stats",
        "intelligence_v4_generate_suggestions",
        "intelligence_v4_get_suggestions",
        "intelligence_v4_mark_suggestion",
        "intelligence_v4_clean_suggestions",
        "intelligence_v4_analyze_behavior",
        "intelligence_v4_behavior_trend",
        "intelligence_v4_behavior_history",
        "intelligence_v4_kb_summarize",
        "intelligence_v4_kb_tags",
        // intelligence_behavior v2 (6)
        "intelligence_v2_analyze_behavior",
        "intelligence_v2_recommend_workflows",
        "intelligence_v2_detect_tech_stack",
        "intelligence_v2_cognitive_load",
        "intelligence_v2_notifications",
        "intelligence_v2_dashboard",
        // intelligence_anomaly v3 (5)
        "intelligence_v3_detect_anomaly",
        "intelligence_v3_snapshot_resources",
        "intelligence_v3_organize_knowledge",
        "intelligence_v3_list_scheduled_tasks",
        "intelligence_v3_execute_scheduled_tasks",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 74);
        assert_eq!(actual.len(), IPC_ALIASES.len(), "legacy_command 存在重复");
        assert_eq!(actual, expected);
    }

    #[test]
    fn alias_map_uses_formal_id_and_unique_short_code_commands() {
        let new_commands: HashSet<_> = IPC_ALIASES.iter().map(|alias| alias.new_command).collect();

        assert_eq!(
            new_commands.len(),
            IPC_ALIASES.len(),
            "new_command 存在重复"
        );
        for alias in IPC_ALIASES {
            assert_eq!(alias.plugin_id, PLUGIN_ID);
            assert_eq!(alias.short_code, SHORT_CODE);
            assert_eq!(
                alias.new_command,
                format!("{SHORT_CODE}:plugin:{}", alias.legacy_command)
            );
        }
    }
}
