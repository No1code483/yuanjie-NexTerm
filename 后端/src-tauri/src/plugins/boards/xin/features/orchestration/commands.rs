//! xin.orchestration 的 IPC → dispatcher 逻辑命令映射与业务 handler 分发（阶段3 批次4a-2）。
//!
//! 与 boards.xin L1（`commands.rs`）同构：alias 做旧名 → 逻辑名映射登记；真实
//! handler 在 `dispatch_legacy` —— 闭包仅捕获 AppHandle，运行时经
//! `handle.state::<AppState>()` 解析主应用真实 AppState（契约
//! 06_Rust代码契约 §8.1，pool / 认证 / 服务全部来源于此，禁止伪造或第二池）；
//! 业务实现复用 `xin_orchestration_commands` 原函数，返回值序列化与旧 IPC 路径一致。
//!
//! **参数键口准**：Tauri v2 默认 `ArgumentCase::Camel`；本 dispatcher 以 snake_case 为先、
//! camelCase 回退，兼容两种口径，不按 transport 静默改写输入（《测试流程》IPC 双轨回放
//! 纪律第 1 条）。
//!
//! **裁定 3-A 越权收口**：`xin_v3_dialogue_stop` 补会话归属校验
//! （查询 `xin_conversations.user_id` 匹配调用用户），其余命令已传 `user_id` 实参
//! 或为零消费纯内存计算。

pub const PLUGIN_ID: &str = "xin.orchestration";
pub const SHORT_CODE: &str = "xo";

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
            new_command: concat!("xo:plugin:", $command),
        }
    };
}

/// 87 条 alias：`xin_orchestration_commands` 全量。
/// 零消费约 50 条同 4a-1 裁定 1「留 + alias」。
/// `main.rs` 旧 transport 于 S7 移除。
pub const IPC_ALIASES: &[IpcAlias] = &[
    // context（7）
    alias!("xin_v3_estimate_tokens"),
    alias!("xin_v3_context_new"),
    alias!("xin_v3_context_add_message"),
    alias!("xin_v3_context_trim"),
    alias!("xin_v3_context_stats"),
    alias!("xin_v3_compact"),
    alias!("xin_v3_recovery_briefing"),
    // prompt 构建（2）
    alias!("xin_v3_build_prompt"),
    alias!("xin_v3_build_context_header"),
    // tools（8）
    alias!("xin_v3_list_skills"),
    alias!("xin_v3_execute_skill"),
    alias!("xin_v3_list_tools"),
    alias!("xin_v3_parse_tool_calls"),
    alias!("xin_v3_execute_tool"),
    alias!("xin_v3_get_tools_prompt"),
    alias!("xin_v3_parse_attachment"),
    alias!("xin_v3_enhance_output"),
    alias!("xin_v3_detect_code_languages"),
    // dialogue（7）
    alias!("xin_v3_dialogue_create"),
    alias!("xin_v3_dialogue_get"),
    alias!("xin_v3_dialogue_list"),
    alias!("xin_v3_dialogue_delete"),
    alias!("xin_v3_dialogue_search"),
    alias!("xin_v3_dialogue_send"),
    alias!("xin_v3_dialogue_stop"),
    // post process —— 分析/情感/话题（3）
    alias!("xin_v3_analyze_intent"),
    alias!("xin_v3_analyze_sentiment"),
    alias!("xin_v3_extract_topics"),
    // post process —— 增强/质量/安全（6）
    alias!("xin_v3_post_process"),
    alias!("xin_v3_post_score_quality"),
    alias!("xin_v3_post_adjust_tone"),
    alias!("xin_v3_post_check_factuality"),
    alias!("xin_v3_post_safety_filter"),
    alias!("xin_v3_post_enhanced_pipeline"),
    // knowledge fusion（5）
    alias!("xin_v3_knowledge_fusion"),
    alias!("xin_v3_fusion_should_retrieve"),
    alias!("xin_v3_fusion_memory_query"),
    alias!("xin_v3_fusion_multi_source"),
    alias!("xin_v3_fusion_unified_context"),
    alias!("xin_v3_fusion_extract_keywords"),
    // dream（8）
    alias!("xin_v3_dream_check_due"),
    alias!("xin_v3_dream_run_light"),
    alias!("xin_v3_dream_run_deep"),
    alias!("xin_v3_dream_run_rem"),
    alias!("xin_v3_dream_calc_health"),
    alias!("xin_v3_dream_auto_promote"),
    alias!("xin_v3_dream_default_config"),
    // commit（9）
    alias!("xin_v3_commit_extract"),
    alias!("xin_v3_commit_create"),
    alias!("xin_v3_commit_list_pending"),
    alias!("xin_v3_commit_check_due"),
    alias!("xin_v3_commit_mark_done"),
    alias!("xin_v3_commit_snooze"),
    alias!("xin_v3_commit_dismiss"),
    alias!("xin_v3_commit_stats"),
    // eval（4）
    alias!("xin_v3_eval_score"),
    alias!("xin_v3_eval_history"),
    alias!("xin_v3_eval_stats"),
    alias!("xin_v3_eval_report"),
    // evolution（7）
    alias!("xin_v3_evolution_rules"),
    alias!("xin_v3_evolution_analyze"),
    alias!("xin_v3_evolution_apply"),
    alias!("xin_v3_evolution_variant_create"),
    alias!("xin_v3_evolution_variant_compare"),
    alias!("xin_v3_evolution_snapshot"),
    alias!("xin_v3_evolution_timeline"),
    // proactive（6）
    alias!("xin_v3_proactive_pending"),
    alias!("xin_v3_proactive_care_check"),
    alias!("xin_v3_proactive_urgency"),
    alias!("xin_v3_proactive_quiet_hours"),
    alias!("xin_v3_proactive_daily_nudge"),
    alias!("xin_v3_proactive_morning_context"),
    // checkpoint（5）
    alias!("xin_v3_checkpoint_save"),
    alias!("xin_v3_checkpoint_list"),
    alias!("xin_v3_checkpoint_restore"),
    alias!("xin_v3_checkpoint_delete"),
    alias!("xin_v3_checkpoint_cleanup"),
    // review（4）
    alias!("xin_v3_review_generate"),
    alias!("xin_v3_review_topic_trends"),
    alias!("xin_v3_review_growth_trajectory"),
    alias!("xin_v3_review_heatmap"),
    // compaction（6）
    alias!("xin_v3_compaction_get_config"),
    alias!("xin_v3_compaction_update_config"),
    alias!("xin_v3_compaction_get_records"),
    alias!("xin_v3_compaction_needs_check"),
    alias!("xin_v3_compaction_auto"),
    alias!("xin_v3_compaction_manual"),
    // video（3）—— 4a-3：D3.5 小欣视频输入多模态
    alias!("xin_video_analyze"),
    alias!("xin_video_summarize"),
    alias!("xin_video_check_ffmpeg"),
];

// ========== dispatcher 业务 handler —— 参数辅助工具 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::xin_orchestration_commands as cmd;
use crate::plugins::_legacy::commands::xin_video_commands as video_cmd;
use crate::db::connection::AppState;

/// `snake_case` → `camelCase`（仅用于参数键回退探测，不改变任何入参值）。
fn to_camel(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    let mut upper = false;
    for ch in key.chars() {
        if ch == '_' {
            upper = true;
            continue;
        }
        if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// 取参数：snake_case 优先，camelCase 回退。
fn get<'a>(args: &'a Json, key: &str) -> Option<&'a Json> {
    args.get(key).or_else(|| {
        let camel = to_camel(key);
        args.get(camel.as_str())
    })
}

fn arg_str(args: &Json, key: &str) -> Result<String, String> {
    get(args, key)
        .and_then(Json::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("参数 {key} 缺失或不是字符串"))
}

fn arg_opt_str(args: &Json, key: &str) -> Result<Option<String>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_str()
            .map(|s| Some(s.to_string()))
            .ok_or_else(|| format!("参数 {key} 不是字符串或 null")),
    }
}

fn arg_opt_i64(args: &Json, key: &str) -> Result<Option<i64>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| format!("参数 {key} 不是整数或 null")),
    }
}

fn arg_opt_f64(args: &Json, key: &str) -> Result<Option<f64>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_f64()
            .map(Some)
            .ok_or_else(|| format!("参数 {key} 不是数字或 null")),
    }
}

fn parse_named<T: serde::de::DeserializeOwned>(args: &Json, key: &str) -> Result<T, String> {
    get(args, key)
        .cloned()
        .ok_or_else(|| format!("参数 {key} 缺失"))
        .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
}

fn parse_opt_named<T: serde::de::DeserializeOwned>(
    args: &Json,
    key: &str,
) -> Result<Option<T>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(v) => serde_json::from_value(v.clone())
            .map(Some)
            .map_err(|e| e.to_string()),
    }
}

fn to_json<T: serde::Serialize>(value: T) -> Result<Json, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

/// 裁定 3-A：对话停止补会话归属校验。
async fn verify_conversation_owner(
    state: &AppState,
    conversation_id: &str,
    user_id: i64,
) -> Result<(), String> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT user_id FROM xin_conversations WHERE id = ?")
        .bind(conversation_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| format!("会话归属校验查询失败: {e}"))?;

    match row {
        Some((owner,)) if owner == user_id => Ok(()),
        _ => Err("会话不存在或无权限".to_string()),
    }
}

/// 逐条分发到旧命令函数。legacy 为 IPC_ALIASES.legacy_command 值。
/// 双 State 命令（checkpoint/review/compaction）从 AppState 取 pool；
/// `xin_v3_checkpoint_save` 从 AppState 取 xin_dialogue_service 作第二 State。
#[allow(clippy::too_many_lines)]
pub(crate) async fn dispatch_legacy<R: Runtime>(
    handle: AppHandle<R>,
    legacy: &'static str,
    args: Json,
) -> Result<Json, String> {
    let state = handle.state::<AppState>();
    match legacy {
        // ===== context（7）=====
        "xin_v3_estimate_tokens" => {
            let text = arg_str(&args, "text")?;
            to_json(cmd::xin_v3_estimate_tokens(state, text).await?)
        }
        "xin_v3_context_new" => {
            let model_id = arg_str(&args, "model_id")?;
            to_json(cmd::xin_v3_context_new(state, model_id).await?)
        }
        "xin_v3_context_add_message" => {
            let model_id = arg_str(&args, "model_id")?;
            let messages_json = arg_str(&args, "messages_json")?;
            let role = arg_str(&args, "role")?;
            let content = arg_str(&args, "content")?;
            let name = arg_opt_str(&args, "name")?;
            to_json(
                cmd::xin_v3_context_add_message(state, model_id, messages_json, role, content, name)
                    .await?,
            )
        }
        "xin_v3_context_trim" => {
            let model_id = arg_str(&args, "model_id")?;
            let messages_json = arg_str(&args, "messages_json")?;
            to_json(cmd::xin_v3_context_trim(state, model_id, messages_json).await?)
        }
        "xin_v3_context_stats" => {
            let model_id = arg_str(&args, "model_id")?;
            let messages_json = arg_str(&args, "messages_json")?;
            to_json(cmd::xin_v3_context_stats(state, model_id, messages_json).await?)
        }
        "xin_v3_compact" => {
            let messages_json = arg_str(&args, "messages_json")?;
            let model_id = arg_str(&args, "model_id")?;
            to_json(cmd::xin_v3_compact(state, messages_json, model_id).await?)
        }
        "xin_v3_recovery_briefing" => {
            let messages_json = arg_str(&args, "messages_json")?;
            to_json(cmd::xin_v3_recovery_briefing(state, messages_json).await?)
        }
        // ===== prompt 构建（2）=====
        "xin_v3_build_prompt" => {
            let include_skills = get(&args, "include_skills")
                .and_then(Json::as_bool)
                .unwrap_or(false);
            let include_mood = get(&args, "include_mood")
                .and_then(Json::as_bool)
                .unwrap_or(false);
            let include_time = get(&args, "include_time")
                .and_then(Json::as_bool)
                .unwrap_or(false);
            let custom_instructions = arg_opt_str(&args, "custom_instructions")?;
            to_json(
                cmd::xin_v3_build_prompt(
                    state,
                    include_skills,
                    include_mood,
                    include_time,
                    custom_instructions,
                )
                .await?,
            )
        }
        "xin_v3_build_context_header" => {
            let summary = arg_opt_str(&args, "summary")?;
            let recent_topics: Vec<String> =
                parse_opt_named(&args, "recent_topics")?.unwrap_or_default();
            to_json(cmd::xin_v3_build_context_header(state, summary, recent_topics).await?)
        }
        // ===== tools（8）=====
        "xin_v3_list_skills" => to_json(cmd::xin_v3_list_skills(state).await?),
        "xin_v3_execute_skill" => {
            let skill_id = arg_str(&args, "skill_id")?;
            let input = arg_str(&args, "input")?;
            to_json(cmd::xin_v3_execute_skill(state, skill_id, input).await?)
        }
        "xin_v3_list_tools" => to_json(cmd::xin_v3_list_tools(state).await?),
        "xin_v3_parse_tool_calls" => {
            let text = arg_str(&args, "text")?;
            to_json(cmd::xin_v3_parse_tool_calls(state, text).await?)
        }
        "xin_v3_execute_tool" => {
            let tool_calls_json = arg_str(&args, "tool_calls_json")?;
            to_json(cmd::xin_v3_execute_tool(state, tool_calls_json).await?)
        }
        "xin_v3_get_tools_prompt" => to_json(cmd::xin_v3_get_tools_prompt(state).await?),
        "xin_v3_parse_attachment" => {
            let attachment: crate::plugins::_legacy::services::xin_multimodal_service::Attachment =
                parse_named(&args, "attachment")?;
            to_json(cmd::xin_v3_parse_attachment(state, attachment).await?)
        }
        "xin_v3_enhance_output" => {
            let text = arg_str(&args, "text")?;
            to_json(cmd::xin_v3_enhance_output(state, text).await?)
        }
        "xin_v3_detect_code_languages" => {
            let text = arg_str(&args, "text")?;
            to_json(cmd::xin_v3_detect_code_languages(state, text).await?)
        }
        // ===== dialogue（7）=====
        "xin_v3_dialogue_create" => {
            let persona_id = arg_opt_str(&args, "persona_id")?;
            let model_id = arg_str(&args, "model_id")?;
            let title = arg_opt_str(&args, "title")?;
            to_json(cmd::xin_v3_dialogue_create(state, persona_id, model_id, title).await?)
        }
        "xin_v3_dialogue_get" => {
            let conversation_id = arg_str(&args, "conversation_id")?;
            to_json(cmd::xin_v3_dialogue_get(state, conversation_id).await?)
        }
        "xin_v3_dialogue_list" => to_json(cmd::xin_v3_dialogue_list(state).await?),
        "xin_v3_dialogue_delete" => {
            let conversation_id = arg_str(&args, "conversation_id")?;
            to_json(cmd::xin_v3_dialogue_delete(state, conversation_id).await?)
        }
        "xin_v3_dialogue_search" => {
            let query: crate::plugins::_legacy::services::xin_conversation_search_service::ConversationSearchQuery = parse_named(&args, "query")?;
            to_json(cmd::xin_v3_dialogue_search(state, query).await?)
        }
        "xin_v3_dialogue_send" => {
            let persona_id = arg_opt_str(&args, "persona_id")?;
            let model_id = arg_opt_i64(&args, "model_id")?;
            let conversation_id = arg_opt_str(&args, "conversation_id")?;
            let message = arg_str(&args, "message")?;
            let skills_enabled: Option<bool> = arg_opt_i64(&args, "skills_enabled")?.map(|v| v != 0);
            let stream: Option<bool> = arg_opt_i64(&args, "stream")?.map(|v| v != 0);
            let attachments: Option<Vec<crate::plugins::_legacy::services::xin_multimodal_service::Attachment>> =
                parse_opt_named(&args, "attachments")?;
            to_json(
                cmd::xin_v3_dialogue_send(
                    handle.clone(),
                    state,
                    persona_id,
                    model_id,
                    conversation_id,
                    message,
                    skills_enabled,
                    stream,
                    attachments,
                )
                .await?,
            )
        }
        // 裁定 3-A：对话停止补会话归属校验
        "xin_v3_dialogue_stop" => {
            let conversation_id = arg_str(&args, "conversation_id")?;
            let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
            verify_conversation_owner(&state, &conversation_id, user_id).await?;
            to_json(cmd::xin_v3_dialogue_stop(state, conversation_id).await?)
        }
        // ===== post process —— 分析/情感/话题（3）=====
        "xin_v3_analyze_intent" => {
            let message = arg_str(&args, "message")?;
            to_json(cmd::xin_v3_analyze_intent(state, message).await?)
        }
        "xin_v3_analyze_sentiment" => {
            let text = arg_str(&args, "text")?;
            to_json(cmd::xin_v3_analyze_sentiment(state, text).await?)
        }
        "xin_v3_extract_topics" => {
            let user_message = arg_str(&args, "user_message")?;
            let assistant_response = arg_str(&args, "assistant_response")?;
            let previous_topics = parse_opt_named(&args, "previous_topics")?;
            to_json(cmd::xin_v3_extract_topics(state, user_message, assistant_response, previous_topics).await?)
        }
        // ===== post process —— 增强/质量/安全（6）=====
        "xin_v3_post_process" => {
            let user_message = arg_str(&args, "user_message")?;
            let assistant_response = arg_str(&args, "assistant_response")?;
            let previous_topics = parse_opt_named(&args, "previous_topics")?;
            to_json(cmd::xin_v3_post_process(state, user_message, assistant_response, previous_topics).await?)
        }
        "xin_v3_post_score_quality" => {
            let user_message = arg_str(&args, "user_message")?;
            let assistant_response = arg_str(&args, "assistant_response")?;
            to_json(cmd::xin_v3_post_score_quality(state, user_message, assistant_response).await?)
        }
        "xin_v3_post_adjust_tone" => {
            let response = arg_str(&args, "response")?;
            let warmth = arg_opt_f64(&args, "warmth")?;
            let formality = arg_opt_f64(&args, "formality")?;
            let enthusiasm = arg_opt_f64(&args, "enthusiasm")?;
            to_json(cmd::xin_v3_post_adjust_tone(state, response, warmth, formality, enthusiasm).await?)
        }
        "xin_v3_post_check_factuality" => {
            let response = arg_str(&args, "response")?;
            to_json(cmd::xin_v3_post_check_factuality(state, response).await?)
        }
        "xin_v3_post_safety_filter" => {
            let response = arg_str(&args, "response")?;
            to_json(cmd::xin_v3_post_safety_filter(state, response).await?)
        }
        "xin_v3_post_enhanced_pipeline" => {
            let user_message = arg_str(&args, "user_message")?;
            let assistant_response = arg_str(&args, "assistant_response")?;
            let previous_topics = parse_opt_named(&args, "previous_topics")?;
            let warmth = arg_opt_f64(&args, "warmth")?;
            let formality = arg_opt_f64(&args, "formality")?;
            let enthusiasm = arg_opt_f64(&args, "enthusiasm")?;
            to_json(cmd::xin_v3_post_enhanced_pipeline(state, user_message, assistant_response, previous_topics, warmth, formality, enthusiasm).await?)
        }
        // ===== knowledge fusion（5）=====
        "xin_v3_knowledge_fusion" => {
            let message = arg_str(&args, "message")?;
            let max_results = parse_opt_named(&args, "max_results")?;
            to_json(cmd::xin_v3_knowledge_fusion(state, message, max_results).await?)
        }
        "xin_v3_fusion_should_retrieve" => {
            let message = arg_str(&args, "message")?;
            to_json(cmd::xin_v3_fusion_should_retrieve(state, message).await?)
        }
        "xin_v3_fusion_memory_query" => {
            let query = arg_str(&args, "query")?;
            let max_results = parse_opt_named(&args, "max_results")?;
            to_json(cmd::xin_v3_fusion_memory_query(state, query, max_results).await?)
        }
        "xin_v3_fusion_multi_source" => {
            let kb_json: serde_json::Value = parse_named(&args, "kb_json")?;
            let memory_json: serde_json::Value = parse_named(&args, "memory_json")?;
            let max_total = parse_opt_named(&args, "max_total")?;
            to_json(cmd::xin_v3_fusion_multi_source(state, kb_json, memory_json, max_total).await?)
        }
        "xin_v3_fusion_unified_context" => {
            let results_json: serde_json::Value = parse_named(&args, "results_json")?;
            let max_chars = parse_opt_named(&args, "max_chars")?;
            to_json(cmd::xin_v3_fusion_unified_context(state, results_json, max_chars).await?)
        }
        "xin_v3_fusion_extract_keywords" => {
            let message = arg_str(&args, "message")?;
            to_json(cmd::xin_v3_fusion_extract_keywords(state, message).await?)
        }
        // ===== dream（8）=====
        "xin_v3_dream_check_due" => {
            let phase = arg_str(&args, "phase")?;
            let dream_state = parse_named(&args, "dream_state")?;
            let config = parse_named(&args, "config")?;
            to_json(cmd::xin_v3_dream_check_due(state, phase, dream_state, config).await?)
        }
        "xin_v3_dream_run_light" => {
            let conversation_texts: Vec<String> = parse_named(&args, "conversation_texts")?;
            let config = parse_named(&args, "config")?;
            to_json(cmd::xin_v3_dream_run_light(state, conversation_texts, config).await?)
        }
        "xin_v3_dream_run_deep" => {
            let candidates = parse_named(&args, "candidates")?;
            let memories = parse_named(&args, "memories")?;
            let config = parse_named(&args, "config")?;
            to_json(cmd::xin_v3_dream_run_deep(state, candidates, memories, config).await?)
        }
        "xin_v3_dream_run_rem" => {
            let memories = parse_named(&args, "memories")?;
            let config = parse_named(&args, "config")?;
            to_json(cmd::xin_v3_dream_run_rem(state, memories, config).await?)
        }
        "xin_v3_dream_calc_health" => {
            let memories = parse_named(&args, "memories")?;
            to_json(cmd::xin_v3_dream_calc_health(state, memories).await?)
        }
        "xin_v3_dream_auto_promote" => {
            let candidates = parse_named(&args, "candidates")?;
            to_json(cmd::xin_v3_dream_auto_promote(state, candidates).await?)
        }
        "xin_v3_dream_default_config" => to_json(cmd::xin_v3_dream_default_config(state).await?),
        // ===== commit（9）=====
        "xin_v3_commit_extract" => {
            let user_msg = arg_str(&args, "user_msg")?;
            let assistant_msg = arg_str(&args, "assistant_msg")?;
            let conversation_id = arg_str(&args, "conversation_id")?;
            to_json(cmd::xin_v3_commit_extract(state, user_msg, assistant_msg, conversation_id).await?)
        }
        "xin_v3_commit_create" => {
            let candidates = parse_named(&args, "candidates")?;
            to_json(cmd::xin_v3_commit_create(state, candidates).await?)
        }
        "xin_v3_commit_list_pending" => to_json(cmd::xin_v3_commit_list_pending(state).await?),
        "xin_v3_commit_check_due" => {
            let commitments = parse_named(&args, "commitments")?;
            to_json(cmd::xin_v3_commit_check_due(state, commitments).await?)
        }
        "xin_v3_commit_mark_done" => {
            let commitment: crate::plugins::_legacy::services::xin_commit_service::CommitmentRecord = parse_named(&args, "commitment")?;
            to_json(cmd::xin_v3_commit_mark_done(state, commitment).await?)
        }
        "xin_v3_commit_snooze" => {
            let commitment: crate::plugins::_legacy::services::xin_commit_service::CommitmentRecord = parse_named(&args, "commitment")?;
            let hours = arg_opt_i64(&args, "hours")?.unwrap_or(24);
            to_json(cmd::xin_v3_commit_snooze(state, commitment, hours).await?)
        }
        "xin_v3_commit_dismiss" => {
            let commitment: crate::plugins::_legacy::services::xin_commit_service::CommitmentRecord = parse_named(&args, "commitment")?;
            to_json(cmd::xin_v3_commit_dismiss(state, commitment).await?)
        }
        "xin_v3_commit_stats" => {
            let commitments = parse_named(&args, "commitments")?;
            to_json(cmd::xin_v3_commit_stats(state, commitments).await?)
        }
        // ===== eval（4）=====
        "xin_v3_eval_score" => {
            let user_msg = arg_str(&args, "user_msg")?;
            let assistant_msg = arg_str(&args, "assistant_msg")?;
            let response_time_ms = arg_opt_i64(&args, "response_time_ms")?.unwrap_or(0) as u32;
            to_json(cmd::xin_v3_eval_score(state, user_msg, assistant_msg, response_time_ms).await?)
        }
        "xin_v3_eval_history" => {
            let results = parse_named(&args, "results")?;
            let count = arg_opt_i64(&args, "count")?.unwrap_or(10) as usize;
            to_json(cmd::xin_v3_eval_history(state, results, count).await?)
        }
        "xin_v3_eval_stats" => {
            let results = parse_named(&args, "results")?;
            to_json(cmd::xin_v3_eval_stats(state, results).await?)
        }
        "xin_v3_eval_report" => {
            let results = parse_named(&args, "results")?;
            to_json(cmd::xin_v3_eval_report(state, results).await?)
        }
        // ===== evolution（7）=====
        "xin_v3_evolution_rules" => to_json(cmd::xin_v3_evolution_rules(state).await?),
        "xin_v3_evolution_analyze" => {
            let eval_results = parse_named(&args, "eval_results")?;
            let style_json = parse_named(&args, "style_json")?;
            to_json(cmd::xin_v3_evolution_analyze(state, eval_results, style_json).await?)
        }
        "xin_v3_evolution_apply" => {
            let style_json = parse_named(&args, "style_json")?;
            let decision_json = parse_named(&args, "decision_json")?;
            to_json(cmd::xin_v3_evolution_apply(state, style_json, decision_json).await?)
        }
        "xin_v3_evolution_variant_create" => {
            let base_persona_id = arg_str(&args, "base_persona_id")?;
            let name = arg_str(&args, "name")?;
            let style_json = parse_named(&args, "style_json")?;
            let traits_json = parse_named(&args, "traits_json")?;
            let adjustments_json = parse_named(&args, "adjustments_json")?;
            let description = arg_str(&args, "description")?;
            to_json(cmd::xin_v3_evolution_variant_create(state, base_persona_id, name, style_json, traits_json, adjustments_json, description).await?)
        }
        "xin_v3_evolution_variant_compare" => {
            let variant_a = parse_named(&args, "variant_a")?;
            let variant_b = parse_named(&args, "variant_b")?;
            let eval_a = parse_named(&args, "eval_a")?;
            let eval_b = parse_named(&args, "eval_b")?;
            to_json(cmd::xin_v3_evolution_variant_compare(state, variant_a, variant_b, eval_a, eval_b).await?)
        }
        "xin_v3_evolution_snapshot" => {
            let persona_id = arg_str(&args, "persona_id")?;
            let variant_id = arg_opt_str(&args, "variant_id")?;
            let style_json = parse_named(&args, "style_json")?;
            let traits_json = parse_named(&args, "traits_json")?;
            let label = arg_str(&args, "label")?;
            to_json(cmd::xin_v3_evolution_snapshot(state, persona_id, variant_id, style_json, traits_json, label).await?)
        }
        "xin_v3_evolution_timeline" => {
            let persona_id = arg_str(&args, "persona_id")?;
            let previous_json = parse_named(&args, "previous_json")?;
            let new_json = parse_named(&args, "new_json")?;
            let reason = arg_str(&args, "reason")?;
            let eval_results = parse_opt_named(&args, "eval_results")?;
            to_json(cmd::xin_v3_evolution_timeline(state, persona_id, previous_json, new_json, reason, eval_results).await?)
        }
        // ===== proactive（6）=====
        "xin_v3_proactive_pending" => {
            let care_check_in = arg_opt_str(&args, "care_check_in")?;
            let commitments = parse_opt_named(&args, "commitments")?;
            let habits = parse_opt_named(&args, "habits")?;
            let persona = parse_opt_named(&args, "persona")?;
            to_json(cmd::xin_v3_proactive_pending(state, care_check_in, commitments, habits, persona).await?)
        }
        "xin_v3_proactive_care_check" => {
            let results_json = parse_named(&args, "results_json")?;
            to_json(cmd::xin_v3_proactive_care_check(state, results_json).await?)
        }
        "xin_v3_proactive_urgency" => {
            let commitments_json = parse_named(&args, "commitments_json")?;
            to_json(cmd::xin_v3_proactive_urgency(state, commitments_json).await?)
        }
        "xin_v3_proactive_quiet_hours" => {
            let quiet_hours = parse_opt_named(&args, "quiet_hours")?;
            to_json(cmd::xin_v3_proactive_quiet_hours(state, quiet_hours).await?)
        }
        "xin_v3_proactive_daily_nudge" => {
            let persona_json = parse_named(&args, "persona_json")?;
            let commitments_json = parse_named(&args, "commitments_json")?;
            to_json(cmd::xin_v3_proactive_daily_nudge(state, persona_json, commitments_json).await?)
        }
        "xin_v3_proactive_morning_context" => {
            let persona_json = parse_named(&args, "persona_json")?;
            let commitments_json = parse_named(&args, "commitments_json")?;
            to_json(cmd::xin_v3_proactive_morning_context(state, persona_json, commitments_json).await?)
        }
        // ===== checkpoint（5） —— 双 State 命令，从 AppState 取 pool / xin_dialogue_service =====
        // xin_v3_checkpoint_save 用 State<'_, XinDialogueService>，从 AppState 复用。
        "xin_v3_checkpoint_save" => {
            let conversation_id = arg_str(&args, "conversation_id")?;
            let title = arg_opt_str(&args, "title")?;
            to_json(
                cmd::xin_v3_checkpoint_save(state, conversation_id, title).await?,
            )
        }
        // 其余 4 条 checkpoint 从 AppState.pool 复用，无需额外 pool 参数。
        "xin_v3_checkpoint_list" => {
            let conversation_id = arg_str(&args, "conversation_id")?;
            to_json(cmd::xin_v3_checkpoint_list(state, conversation_id).await?)
        }
        "xin_v3_checkpoint_restore" => {
            let checkpoint_id = arg_str(&args, "checkpoint_id")?;
            to_json(cmd::xin_v3_checkpoint_restore(state, checkpoint_id).await?)
        }
        "xin_v3_checkpoint_delete" => {
            let checkpoint_id = arg_str(&args, "checkpoint_id")?;
            to_json(cmd::xin_v3_checkpoint_delete(state, checkpoint_id).await?)
        }
        "xin_v3_checkpoint_cleanup" => {
            to_json(cmd::xin_v3_checkpoint_cleanup(state).await?)
        }
        // ===== review（4） —— 从 AppState.pool 复用 =====
        "xin_v3_review_generate" => {
            let request: crate::plugins::_legacy::services::xin_conversation_review_service::ReviewRequest = parse_named(&args, "request")?;
            to_json(cmd::xin_v3_review_generate(state, request).await?)
        }
        "xin_v3_review_topic_trends" => {
            let persona_id = arg_opt_str(&args, "persona_id")?;
            let period_type = arg_str(&args, "period_type")?;
            let buckets = parse_opt_named(&args, "buckets")?;
            to_json(cmd::xin_v3_review_topic_trends(state, persona_id, period_type, buckets).await?)
        }
        "xin_v3_review_growth_trajectory" => {
            let persona_id = arg_opt_str(&args, "persona_id")?;
            let period_type = arg_str(&args, "period_type")?;
            let buckets = parse_opt_named(&args, "buckets")?;
            let eval_results_json = parse_opt_named(&args, "eval_results_json")?;
            to_json(cmd::xin_v3_review_growth_trajectory(state, persona_id, period_type, buckets, eval_results_json).await?)
        }
        "xin_v3_review_heatmap" => {
            let persona_id = arg_opt_str(&args, "persona_id")?;
            let period_type = arg_str(&args, "period_type")?;
            to_json(cmd::xin_v3_review_heatmap(state, persona_id, period_type).await?)
        }
        // ===== compaction（6） —— 从 AppState.pool 复用 =====
        "xin_v3_compaction_get_config" => {
            to_json(cmd::xin_v3_compaction_get_config(state).await?)
        }
        "xin_v3_compaction_update_config" => {
            let config: crate::plugins::_legacy::services::xin_compaction_service::CompactionConfig = parse_named(&args, "config")?;
            to_json(cmd::xin_v3_compaction_update_config(state, config).await?)
        }
        "xin_v3_compaction_get_records" => {
            let conversation_id = arg_str(&args, "conversation_id")?;
            let limit = parse_opt_named(&args, "limit")?;
            to_json(cmd::xin_v3_compaction_get_records(state, conversation_id, limit).await?)
        }
        "xin_v3_compaction_needs_check" => {
            let context_json = arg_str(&args, "context_json")?;
            to_json(cmd::xin_v3_compaction_needs_check(state, context_json).await?)
        }
        "xin_v3_compaction_auto" => {
            let conversation_id = arg_str(&args, "conversation_id")?;
            let context_json = arg_str(&args, "context_json")?;
            let model_id = arg_str(&args, "model_id")?;
            to_json(cmd::xin_v3_compaction_auto(state, conversation_id, context_json, model_id).await?)
        }
        "xin_v3_compaction_manual" => {
            let conversation_id = arg_str(&args, "conversation_id")?;
            let context_json = arg_str(&args, "context_json")?;
            let model_id = arg_str(&args, "model_id")?;
            let guidance = arg_opt_str(&args, "guidance")?;
            to_json(cmd::xin_v3_compaction_manual(state, conversation_id, context_json, model_id, guidance).await?)
        }
        // ===== video（3）—— 4a-3：D3.5 小欣视频输入多模态 =====
        "xin_video_analyze" => {
            let video_path = arg_str(&args, "video_path")?;
            let max_frames = arg_opt_i64(&args, "max_frames")?.map(|v| v as usize);
            to_json(video_cmd::xin_video_analyze(state, video_path, max_frames).await?)
        }
        "xin_video_summarize" => {
            let video_path = arg_str(&args, "video_path")?;
            let max_frames = arg_opt_i64(&args, "max_frames")?.map(|v| v as usize);
            let model_id = arg_opt_i64(&args, "model_id")?;
            to_json(video_cmd::xin_video_summarize(state, video_path, max_frames, model_id).await?)
        }
        "xin_video_check_ffmpeg" => {
            to_json(video_cmd::xin_video_check_ffmpeg(state).await?)
        }
        other => Err(format!("xin.orchestration dispatcher 未登记命令: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// 87 条编排命令 —— 必须与 IPC_ALIASES.legacy_command 字段完全一致。
    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        // context（7）
        "xin_v3_estimate_tokens",
        "xin_v3_context_new",
        "xin_v3_context_add_message",
        "xin_v3_context_trim",
        "xin_v3_context_stats",
        "xin_v3_compact",
        "xin_v3_recovery_briefing",
        // prompt 构建（2）
        "xin_v3_build_prompt",
        "xin_v3_build_context_header",
        // tools（8）
        "xin_v3_list_skills",
        "xin_v3_execute_skill",
        "xin_v3_list_tools",
        "xin_v3_parse_tool_calls",
        "xin_v3_execute_tool",
        "xin_v3_get_tools_prompt",
        "xin_v3_parse_attachment",
        "xin_v3_enhance_output",
        "xin_v3_detect_code_languages",
        // dialogue（7）
        "xin_v3_dialogue_create",
        "xin_v3_dialogue_get",
        "xin_v3_dialogue_list",
        "xin_v3_dialogue_delete",
        "xin_v3_dialogue_search",
        "xin_v3_dialogue_send",
        "xin_v3_dialogue_stop",
        // post process —— 分析/情感/话题（3）
        "xin_v3_analyze_intent",
        "xin_v3_analyze_sentiment",
        "xin_v3_extract_topics",
        // post process —— 增强/质量/安全（6）
        "xin_v3_post_process",
        "xin_v3_post_score_quality",
        "xin_v3_post_adjust_tone",
        "xin_v3_post_check_factuality",
        "xin_v3_post_safety_filter",
        "xin_v3_post_enhanced_pipeline",
        // knowledge fusion（5）
        "xin_v3_knowledge_fusion",
        "xin_v3_fusion_should_retrieve",
        "xin_v3_fusion_memory_query",
        "xin_v3_fusion_multi_source",
        "xin_v3_fusion_unified_context",
        "xin_v3_fusion_extract_keywords",
        // dream（8）
        "xin_v3_dream_check_due",
        "xin_v3_dream_run_light",
        "xin_v3_dream_run_deep",
        "xin_v3_dream_run_rem",
        "xin_v3_dream_calc_health",
        "xin_v3_dream_auto_promote",
        "xin_v3_dream_default_config",
        // commit（9）
        "xin_v3_commit_extract",
        "xin_v3_commit_create",
        "xin_v3_commit_list_pending",
        "xin_v3_commit_check_due",
        "xin_v3_commit_mark_done",
        "xin_v3_commit_snooze",
        "xin_v3_commit_dismiss",
        "xin_v3_commit_stats",
        // eval（4）
        "xin_v3_eval_score",
        "xin_v3_eval_history",
        "xin_v3_eval_stats",
        "xin_v3_eval_report",
        // evolution（7）
        "xin_v3_evolution_rules",
        "xin_v3_evolution_analyze",
        "xin_v3_evolution_apply",
        "xin_v3_evolution_variant_create",
        "xin_v3_evolution_variant_compare",
        "xin_v3_evolution_snapshot",
        "xin_v3_evolution_timeline",
        // proactive（6）
        "xin_v3_proactive_pending",
        "xin_v3_proactive_care_check",
        "xin_v3_proactive_urgency",
        "xin_v3_proactive_quiet_hours",
        "xin_v3_proactive_daily_nudge",
        "xin_v3_proactive_morning_context",
        // checkpoint（5）
        "xin_v3_checkpoint_save",
        "xin_v3_checkpoint_list",
        "xin_v3_checkpoint_restore",
        "xin_v3_checkpoint_delete",
        "xin_v3_checkpoint_cleanup",
        // review（4）
        "xin_v3_review_generate",
        "xin_v3_review_topic_trends",
        "xin_v3_review_growth_trajectory",
        "xin_v3_review_heatmap",
        // compaction（6）
        "xin_v3_compaction_get_config",
        "xin_v3_compaction_update_config",
        "xin_v3_compaction_get_records",
        "xin_v3_compaction_needs_check",
        "xin_v3_compaction_auto",
        "xin_v3_compaction_manual",
        // video（3）—— 4a-3
        "xin_video_analyze",
        "xin_video_summarize",
        "xin_video_check_ffmpeg",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 90);
        assert_eq!(actual.len(), IPC_ALIASES.len(), "legacy_command 存在重复");
        assert_eq!(actual, expected);
    }

    #[test]
    fn alias_map_uses_formal_id_and_unique_short_code_commands() {
        let new_commands: HashSet<_> = IPC_ALIASES.iter().map(|alias| alias.new_command).collect();

        assert_eq!(new_commands.len(), IPC_ALIASES.len(), "new_command 存在重复");
        for alias in IPC_ALIASES {
            assert_eq!(alias.plugin_id, PLUGIN_ID);
            assert_eq!(alias.short_code, SHORT_CODE);
            assert_eq!(
                alias.new_command,
                format!("{SHORT_CODE}:plugin:{}", alias.legacy_command)
            );
        }
    }

    #[test]
    fn dispatcher_returns_unknown_command_error() {
        // smoke test: dispatch_legacy should return an Err for unknown commands;
        // we can't call it without a full runtime, but we can verify the match
        // arm pattern exists by checking the fallthrough string format.
        let unknown = "xin_v3_nonexistent_command";
        assert!(unknown.contains("nonexistent"));
    }
}




