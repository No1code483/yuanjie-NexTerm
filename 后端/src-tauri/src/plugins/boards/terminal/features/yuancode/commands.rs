//! terminal.yuancode 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发（阶段3 批次3c）。
//!
//! 与 boards.terminal / terminal.linux 同构：alias 只做旧名 → 逻辑名的映射登记；真实
//! handler 在 `dispatch_legacy` —— 闭包仅捕获 AppHandle，运行时经 `handle.state::<*>()`
//! 解析主应用真实状态（契约 06_Rust代码契约 §8.1）；业务实现复用 `yuancode_commands` /
//! `git_commands` / `lsp_commands` / `browser_commands` / `yuan_inline_commands` 原函数，
//! 返回值序列化与旧 IPC 路径一致（V1 输入/输出快照等价）。
//!
//! **参数键口径**：Tauri v2 默认 `ArgumentCase::Camel`；本 dispatcher 以 snake_case 为先、
//! camelCase 回退，兼容两种口径，不按 transport 静默改写输入。
//!
//! **有前端消费**（pages/yuan-code/ 41 文件 + YuanCode.tsx 随批 git mv 迁入插件目录，
//! 裁定 C1）→ S3 提供前端便捷方法；实现层（yuancode_service 等）留主应用复用。
//! **AI 补全类**（yuan_complete/complete_stream/analyze/inline）依赖 MEK/云端模型；
//! **browser_create_view 等依赖 tauri::Window**（dispatcher 经 `get_window("main")` 解析
//! 主窗口，与旧 transport 自动注入的调用来源窗口同 label）。

pub const PLUGIN_ID: &str = "terminal.yuancode";
pub const SHORT_CODE: &str = "yc";

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
            new_command: concat!("yc:plugin:", $command),
        }
    };
}

/// 302 条 alias：yuancode_commands 27 + yuan_inline_commands 3 + git_commands 13 +
/// lsp_commands 5 + browser_commands 6 + editor_commands 26（5a） +
/// file_edit_commands 14（5a） + tools_commands 8（5a） + distill_commands 8（5a） +
/// yuan_sandbox_commands 14（5b） + engine_commands 12（5b） +
/// yuan_agent_commands 15（5b） + yuan_safety_commands 5（5b） +
/// yuan_goal_commands 14（5c） + yuan_compact_commands 7（5c） +
/// yuan_io_control_commands 8（5c） + yuan_prompt_commands 12（5c） +
/// yuan_mcp_commands 40（5d） + yuan_skill_commands 26（5d） +
/// yuan_agent_autonomous_commands 3（5e） + agent_v3_commands 9（5e） +
/// cloud_api_commands 6（5e） + model_routing_commands 6（5e） +
/// yuan_code_monitor_commands 3（5e） + collab_session_commands 7（5e） +
/// project_indexer_commands 5（5e）。
/// `main.rs` 旧 transport 于本批 S7 移除。
pub const IPC_ALIASES: &[IpcAlias] = &[
    // ===== yuancode_commands（27） =====
    // 文件树与编辑器（7）
    alias!("yuan_list_files"),
    alias!("yuan_read_file"),
    alias!("yuan_write_file"),
    alias!("yuan_create_item"),
    alias!("yuan_delete_item"),
    alias!("yuan_rename_item"),
    alias!("yuan_highlight"),
    // 执行与 AI（4）
    alias!("yuan_execute"),
    alias!("yuan_complete"),
    alias!("yuan_complete_stream"),
    alias!("yuan_analyze"),
    // 代码片段（5）
    alias!("yuan_save_snippet"),
    alias!("yuan_get_snippets"),
    alias!("yuan_update_snippet"),
    alias!("yuan_delete_snippet"),
    alias!("yuan_search_snippets"),
    // 差异 / 搜索 / 替换 / 文件信息（5）
    alias!("yuan_compute_diff"),
    alias!("yuan_search_files"),
    alias!("yuan_replace_files"),
    alias!("yuan_copy_move"),
    alias!("yuan_get_file_info"),
    // 工作区（4）
    alias!("yuan_save_workspace"),
    alias!("yuan_load_workspace"),
    alias!("yuan_list_workspaces"),
    alias!("yuan_delete_workspace"),
    // 格式化 / 设置（2）
    alias!("yuan_format_code"),
    alias!("yuan_settings_save"),
    // ===== yuan_inline_commands（3） =====
    alias!("yuan_inline_complete"),
    alias!("yuan_inline_available"),
    alias!("yuan_inline_edit"),
    // ===== git_commands（13） =====
    alias!("git_status"),
    alias!("git_diff_file"),
    alias!("git_diff_unstaged"),
    alias!("git_stage_file"),
    alias!("git_stage_all"),
    alias!("git_unstage_file"),
    alias!("git_commit"),
    alias!("git_push"),
    alias!("git_pull"),
    alias!("git_branches"),
    alias!("git_checkout"),
    alias!("git_log"),
    alias!("git_init"),
    // ===== lsp_commands（5） =====
    alias!("lsp_completions"),
    alias!("lsp_hover"),
    alias!("lsp_definition"),
    alias!("lsp_diagnostics"),
    alias!("lsp_detect_language"),
    // ===== browser_commands（6） =====
    alias!("browser_open_window"),
    alias!("browser_create_view"),
    alias!("browser_navigate_view"),
    alias!("browser_resize_view"),
    alias!("browser_close_view"),
    alias!("browser_cleanup"),
    // ===== editor_commands（26，5a）=====
    alias!("editor_open"),
    alias!("editor_save"),
    alias!("editor_auto_save"),
    alias!("editor_close"),
    alias!("editor_get_versions"),
    alias!("editor_get_version"),
    alias!("editor_restore_version"),
    alias!("editor_recover_session"),
    alias!("editor_list_documents"),
    alias!("editor_delete_document"),
    alias!("editor_extract_metadata"),
    alias!("editor_generate_thumbnail"),
    alias!("editor_highlight"),
    alias!("editor_csv_preview"),
    alias!("editor_decrypted_preview"),
    alias!("editor_search"),
    alias!("editor_convert"),
    alias!("editor_convert_content"),
    alias!("editor_canvas_save"),
    alias!("editor_canvas_load"),
    alias!("editor_canvas_delete"),
    alias!("editor_diff_versions"),
    alias!("editor_cleanup_versions"),
    alias!("editor_index_content"),
    alias!("editor_recover_all_sessions"),
    alias!("editor_cleanup_sessions"),
    // ===== file_edit_commands（14，5a）=====
    alias!("fileedit_read"),
    alias!("fileedit_write"),
    alias!("fileedit_get_status"),
    alias!("tableedit_read"),
    alias!("tableedit_write"),
    alias!("tableedit_export_csv"),
    alias!("pptedit_get_slides"),
    alias!("pptedit_update_slide"),
    alias!("pptedit_add_slide"),
    alias!("pptedit_delete_slide"),
    alias!("pptedit_reorder"),
    alias!("pdfedit_save"),
    alias!("imageedit_save"),
    alias!("audioedit_save"),
    // ===== tools_commands（8，5a）=====
    alias!("yuan_tools_list"),
    alias!("yuan_tools_search"),
    alias!("yuan_tools_call"),
    alias!("yuan_plan_create"),
    alias!("yuan_plan_update_step"),
    alias!("yuan_plan_get"),
    alias!("yuan_plan_list_all"),
    alias!("yuan_apply_patch"),
    // ===== distill_commands（8，5a）=====
    alias!("distill_extract_samples"),
    alias!("distill_generate_dataset"),
    alias!("distill_list_samples"),
    alias!("distill_approve_sample"),
    alias!("distill_reject_sample"),
    alias!("distill_local_infer"),
    alias!("distill_export_jsonl"),
    alias!("distill_sample_stats"),
    // ===== yuan_sandbox_commands（14，5b）=====
    alias!("yuan_sandbox_create"),
    alias!("yuan_sandbox_list"),
    alias!("yuan_sandbox_list_by_agent"),
    alias!("yuan_sandbox_get"),
    alias!("yuan_sandbox_execute"),
    alias!("yuan_sandbox_read_file"),
    alias!("yuan_sandbox_write_file"),
    alias!("yuan_sandbox_delete_file"),
    alias!("yuan_sandbox_list_files"),
    alias!("yuan_sandbox_terminate"),
    alias!("yuan_sandbox_cleanup"),
    alias!("yuan_sandbox_count"),
    alias!("yuan_sandbox_save"),
    alias!("yuan_sandbox_history"),
    // ===== engine_commands（12，5b）=====
    alias!("engine_create_session"),
    alias!("engine_get_session"),
    alias!("engine_list_sessions"),
    alias!("engine_destroy_session"),
    alias!("engine_start_turn"),
    alias!("engine_complete_turn"),
    alias!("engine_abort_turn"),
    alias!("engine_pause_session"),
    alias!("engine_resume_session"),
    alias!("engine_get_turns"),
    alias!("engine_stats"),
    alias!("engine_subscribe_events"),
    // ===== yuan_agent_commands（15，5b）=====
    alias!("yuan_agent_spawn"),
    alias!("yuan_agent_fork"),
    alias!("yuan_agent_list"),
    alias!("yuan_agent_list_all"),
    alias!("yuan_agent_status"),
    alias!("yuan_agent_abort"),
    alias!("yuan_agent_transition"),
    alias!("yuan_agent_send_message"),
    alias!("yuan_agent_receive_messages"),
    alias!("yuan_agent_list_roles"),
    alias!("yuan_agent_get_count"),
    alias!("yuan_agent_cleanup"),
    alias!("yuan_agent_list_templates"),
    alias!("yuan_agent_deploy"),
    alias!("yuan_agent_execute"),
    // ===== yuan_safety_commands（5，5b）=====
    alias!("yuan_safety_check"),
    alias!("yuan_safety_quick_check"),
    alias!("yuan_safety_set_profile"),
    alias!("yuan_safety_get_profile"),
    alias!("yuan_safety_list_profiles"),
    // ===== yuan_goal_commands（14，5c）=====
    alias!("yuan_goal_create"),
    alias!("yuan_goal_get"),
    alias!("yuan_goal_list"),
    alias!("yuan_goal_update"),
    alias!("yuan_goal_start"),
    alias!("yuan_goal_pause"),
    alias!("yuan_goal_complete"),
    alias!("yuan_goal_abort"),
    alias!("yuan_goal_delete"),
    alias!("yuan_goal_consume_tokens"),
    alias!("yuan_goal_update_progress"),
    alias!("yuan_goal_save_checkpoint"),
    alias!("yuan_goal_build_continuation"),
    alias!("yuan_goal_checkpoints_count"),
    // ===== yuan_compact_commands（7，5c）=====
    alias!("yuan_compact_config_get"),
    alias!("yuan_compact_config_update"),
    alias!("yuan_compact_estimate"),
    alias!("yuan_compact_check"),
    alias!("yuan_compact_execute"),
    alias!("yuan_compact_session"),
    alias!("yuan_compact_reset"),
    // ===== yuan_io_control_commands（8，5c）=====
    alias!("yuan_io_config_get"),
    alias!("yuan_io_config_update"),
    alias!("yuan_io_stats"),
    alias!("yuan_io_execute"),
    alias!("yuan_io_cancel"),
    alias!("yuan_io_cancel_all"),
    alias!("yuan_io_kill"),
    alias!("yuan_io_stdin"),
    // ===== yuan_prompt_commands（12，5c）=====
    alias!("yuan_prompt_list_templates"),
    alias!("yuan_prompt_get_template"),
    alias!("yuan_prompt_render"),
    alias!("yuan_prompt_set_custom_template"),
    alias!("yuan_prompt_remove_custom_template"),
    alias!("yuan_prompt_set_variable_default"),
    alias!("yuan_agents_discover"),
    alias!("yuan_agents_sources"),
    alias!("yuan_agents_assemble"),
    alias!("yuan_agents_set_max_bytes"),
    alias!("yuan_agents_get_max_bytes"),
    alias!("yuan_prompt_assemble"),
    // ===== yuan_mcp_commands（40，5d）=====
    alias!("yuan_mcp_register_server"),
    alias!("yuan_mcp_connect_server"),
    alias!("yuan_mcp_disconnect_server"),
    alias!("yuan_mcp_list_servers"),
    alias!("yuan_mcp_list_all_tools"),
    alias!("yuan_mcp_server_status"),
    alias!("yuan_mcp_call_tool"),
    alias!("yuan_mcp_spawn"),
    alias!("yuan_mcp_health_check"),
    alias!("yuan_mcp_health_check_all"),
    alias!("yuan_mcp_set_lifecycle_config"),
    alias!("yuan_mcp_get_lifecycle_state"),
    alias!("yuan_mcp_list_shaped_tools"),
    alias!("yuan_mcp_group_tools_by_namespace"),
    alias!("yuan_mcp_list_tools_by_namespace"),
    alias!("yuan_mcp_get_tool_namespaces"),
    alias!("yuan_mcp_set_shaper_config"),
    alias!("yuan_mcp_search_tools"),
    alias!("yuan_mcp_set_tool_enabled"),
    alias!("yuan_mcp_set_enabled_tools"),
    alias!("yuan_mcp_set_disabled_tools"),
    alias!("yuan_mcp_list_filtered_tools"),
    alias!("yuan_mcp_list_resources"),
    alias!("yuan_mcp_read_resource"),
    alias!("yuan_mcp_list_prompts"),
    alias!("yuan_mcp_get_prompt"),
    alias!("yuan_mcp_set_deferred_namespaces"),
    alias!("yuan_mcp_get_deferred_namespaces"),
    alias!("yuan_mcp_load_namespace"),
    alias!("yuan_mcp_unload_namespace"),
    alias!("yuan_mcp_list_persisted_servers"),
    alias!("yuan_mcp_save_server_config"),
    alias!("yuan_mcp_delete_server_config"),
    alias!("yuan_mcp_set_server_enabled"),
    alias!("yuan_mcp_load_persisted_servers"),
    alias!("yuan_mcp_list_builtin_servers"),
    alias!("yuan_mcp_list_builtin_tools"),
    alias!("yuan_mcp_call_builtin_tool"),
    alias!("yuan_mcp_configure_builtin"),
    alias!("yuan_mcp_list_call_history"),
    // ===== yuan_skill_commands（26，5d）=====
    alias!("yuan_skill_add_root"),
    alias!("yuan_skill_set_project_files"),
    alias!("yuan_skill_load_all"),
    alias!("yuan_skill_get"),
    alias!("yuan_skill_list"),
    alias!("yuan_skill_detect_implicit"),
    alias!("yuan_skill_register"),
    alias!("yuan_skill_unregister"),
    alias!("yuan_skill_trigger"),
    alias!("yuan_skill_auto_discover"),
    alias!("yuan_skill_match"),
    alias!("yuan_skill_render_context"),
    alias!("yuan_skill_stats"),
    alias!("yuan_skill_export"),
    alias!("yuan_skill_market_list"),
    alias!("yuan_skill_market_search"),
    alias!("yuan_skill_market_get"),
    alias!("yuan_skill_market_install"),
    alias!("yuan_skill_market_uninstall"),
    alias!("yuan_skill_install"),
    alias!("yuan_skill_uninstall"),
    alias!("yuan_skill_execute"),
    alias!("yuan_skill_builtin_list"),
    alias!("yuan_skill_rate"),
    alias!("yuan_skill_review"),
    alias!("yuan_skill_ratings_get"),
    // ===== yuan_agent_autonomous_commands（3，5e）=====
    alias!("yuan_agent_execute_autonomous"),
    alias!("yuan_multi_file_edit"),
    alias!("yuan_multi_file_preview_diffs"),
    // ===== agent_v3_commands（9，5e）=====
    alias!("yuan_v3_agent_types"),
    alias!("yuan_v3_agent_create"),
    alias!("yuan_v3_agent_plan"),
    alias!("yuan_v3_agent_execute"),
    alias!("yuan_v3_agent_review"),
    alias!("yuan_v3_agent_safety_check"),
    alias!("yuan_v3_agent_status"),
    alias!("yuan_v3_agent_list"),
    alias!("yuan_v3_agent_destroy"),
    // ===== cloud_api_commands（6，5e）=====
    alias!("cloud_api_list"),
    alias!("cloud_api_upsert"),
    alias!("cloud_api_set_enabled"),
    alias!("cloud_api_delete"),
    alias!("cloud_api_providers"),
    alias!("cloud_api_test_connection"),
    // ===== model_routing_commands（6，5e）=====
    alias!("model_routing_list"),
    alias!("model_routing_upsert"),
    alias!("model_routing_set_enabled"),
    alias!("model_routing_delete"),
    alias!("model_routing_resolve"),
    alias!("model_routing_task_types"),
    // ===== yuan_code_monitor_commands（3，5e）=====
    alias!("yuan_code_monitor_record_event"),
    alias!("yuan_code_monitor_status"),
    alias!("yuan_code_monitor_summary"),
    // ===== collab_session_commands（7，5e）=====
    alias!("collab_session_create"),
    alias!("collab_session_list"),
    alias!("collab_session_get"),
    alias!("collab_session_join"),
    alias!("collab_session_leave"),
    alias!("collab_session_close"),
    alias!("collab_session_update_cursor"),
    // ===== project_indexer_commands（5，5e）=====
    alias!("project_index"),
    alias!("project_index_status"),
    alias!("project_search_symbols"),
    alias!("project_find_references"),
    alias!("project_get_related_files"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::{
    agent_v3_commands, browser_commands, cloud_api_commands, collab_session_commands,
    distill_commands, editor_commands, file_edit_commands, git_commands,
    lsp_commands, model_routing_commands, project_indexer_commands, tools_commands,
    yuan_agent_autonomous_commands, yuan_agent_commands, yuan_code_monitor_commands,
    yuan_compact_commands, yuan_goal_commands, yuan_inline_commands,
    yuan_io_control_commands, yuan_mcp_commands, yuan_prompt_commands,
    yuan_safety_commands, yuan_sandbox_commands, yuancode_commands, yuan_skill_commands,
    engine_commands,
};
use crate::db::connection::AppState;
use crate::plugins::_legacy::services::agent_service::AgentService;
use crate::plugins::_legacy::services::compact_service::CompactService;
use crate::plugins::_legacy::services::git_service::GitService;
use crate::plugins::_legacy::services::goal_service::GoalService;
use crate::plugins::_legacy::services::inline_service::InlineService;
use crate::plugins::_legacy::services::io_control_service::IoControlService;
use crate::plugins::_legacy::services::mcp_service::McpService;
use crate::plugins::_legacy::services::prompt_service::PromptService;
use crate::plugins::_legacy::services::safety_service::SafetyService;
use crate::plugins::_legacy::services::sandbox_service::SandboxService;
use crate::plugins::_legacy::services::skill_service::SkillService;
use crate::plugins::_legacy::commands::agent_v3_commands::AgentRegistry;
use crate::plugins::_legacy::commands::engine_commands::EngineState;
use crate::plugins::_legacy::commands::tools_commands::ToolState;

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

fn arg_i64(args: &Json, key: &str) -> Result<i64, String> {
    get(args, key)
        .and_then(Json::as_i64)
        .ok_or_else(|| format!("参数 {key} 缺失或不是整数"))
}

fn arg_u32(args: &Json, key: &str) -> Result<u32, String> {
    get(args, key)
        .and_then(Json::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| format!("参数 {key} 缺失或不是 u32"))
}

fn arg_f64(args: &Json, key: &str) -> Result<f64, String> {
    get(args, key)
        .and_then(Json::as_f64)
        .ok_or_else(|| format!("参数 {key} 缺失或不是数字"))
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

/// 可选布尔参数：snake_case 优先，camelCase 回退，null → None。
fn arg_opt_bool(args: &Json, key: &str) -> Result<Option<bool>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| format!("参数 {key} 不是布尔或 null")),
    }
}

/// 必填布尔参数：snake_case 优先，camelCase 回退。
fn arg_bool(args: &Json, key: &str) -> Result<bool, String> {
    get(args, key)
        .and_then(Json::as_bool)
        .ok_or_else(|| format!("参数 {key} 缺失或不是布尔值"))
}

/// 请求体命令（`request: T` 参数）：取 args["request"] 反序列化。
fn parse_request<T: serde::de::DeserializeOwned>(args: &Json, key: &str) -> Result<T, String> {
    serde_json::from_value(get(args, key).cloned().unwrap_or(Json::Null))
        .map_err(|e| format!("参数 {key} 反序列化失败: {e}"))
}

fn to_json<T: serde::Serialize>(value: T) -> Result<Json, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

/// 逐条分发到旧命令函数。legacy 为 IPC_ALIASES.legacy_command 值。
pub(crate) async fn dispatch_legacy<R: Runtime>(
    handle: AppHandle<R>,
    legacy: &'static str,
    args: Json,
) -> Result<Json, String> {
    match legacy {
        // ===== yuancode_commands：文件树与编辑器 =====
        "yuan_list_files" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_list_files(handle.state::<AppState>(), request).await?)
        }
        "yuan_read_file" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_read_file(handle.state::<AppState>(), request).await?)
        }
        "yuan_write_file" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_write_file(handle.state::<AppState>(), request).await?)
        }
        "yuan_create_item" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_create_item(handle.state::<AppState>(), request).await?)
        }
        "yuan_delete_item" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_delete_item(handle.state::<AppState>(), request).await?)
        }
        "yuan_rename_item" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_rename_item(handle.state::<AppState>(), request).await?)
        }
        "yuan_highlight" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_highlight(handle.state::<AppState>(), request).await?)
        }
        // ===== yuancode_commands：执行与 AI =====
        "yuan_execute" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_execute(handle.state::<AppState>(), request).await?)
        }
        "yuan_complete" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_complete(handle.state::<AppState>(), request).await?)
        }
        "yuan_complete_stream" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuancode_commands::yuan_complete_stream(
                    handle.state::<AppState>(),
                    handle.clone(),
                    request,
                )
                .await?,
            )
        }
        "yuan_analyze" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_analyze(handle.state::<AppState>(), request).await?)
        }
        // ===== yuancode_commands：代码片段 =====
        "yuan_save_snippet" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_save_snippet(handle.state::<AppState>(), request).await?)
        }
        "yuan_get_snippets" => {
            let language = arg_opt_str(&args, "language")?;
            to_json(yuancode_commands::yuan_get_snippets(handle.state::<AppState>(), language).await?)
        }
        "yuan_update_snippet" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_update_snippet(handle.state::<AppState>(), request).await?)
        }
        "yuan_delete_snippet" => {
            let id = arg_i64(&args, "id")?;
            to_json(yuancode_commands::yuan_delete_snippet(handle.state::<AppState>(), id).await?)
        }
        "yuan_search_snippets" => {
            let query = arg_str(&args, "query")?;
            to_json(yuancode_commands::yuan_search_snippets(handle.state::<AppState>(), query).await?)
        }
        // ===== yuancode_commands：差异 / 搜索 / 替换 / 文件信息 =====
        "yuan_compute_diff" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_compute_diff(handle.state::<AppState>(), request).await?)
        }
        "yuan_search_files" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_search_files(handle.state::<AppState>(), request).await?)
        }
        "yuan_replace_files" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_replace_files(handle.state::<AppState>(), request).await?)
        }
        "yuan_copy_move" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_copy_move(handle.state::<AppState>(), request).await?)
        }
        "yuan_get_file_info" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_get_file_info(handle.state::<AppState>(), request).await?)
        }
        // ===== yuancode_commands：工作区 =====
        "yuan_save_workspace" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_save_workspace(handle.state::<AppState>(), request).await?)
        }
        "yuan_load_workspace" => {
            let id = arg_i64(&args, "id")?;
            to_json(yuancode_commands::yuan_load_workspace(handle.state::<AppState>(), id).await?)
        }
        "yuan_list_workspaces" => {
            to_json(yuancode_commands::yuan_list_workspaces(handle.state::<AppState>()).await?)
        }
        "yuan_delete_workspace" => {
            let id = arg_i64(&args, "id")?;
            to_json(yuancode_commands::yuan_delete_workspace(handle.state::<AppState>(), id).await?)
        }
        // ===== yuancode_commands：格式化 / 设置 =====
        "yuan_format_code" => {
            let request = parse_request(&args, "request")?;
            to_json(yuancode_commands::yuan_format_code(handle.state::<AppState>(), request).await?)
        }
        "yuan_settings_save" => {
            let settings = parse_request(&args, "settings")?;
            to_json(yuancode_commands::yuan_settings_save(handle.state::<AppState>(), settings).await?)
        }
        // ===== yuan_inline_commands =====
        "yuan_inline_complete" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_inline_commands::yuan_inline_complete(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<InlineService>(),
                )
                .await?,
            )
        }
        "yuan_inline_available" => {
            let file_path = arg_str(&args, "file_path")?;
            let language = arg_str(&args, "language")?;
            to_json(
                yuan_inline_commands::yuan_inline_available(
                    handle.state::<AppState>(),
                    file_path,
                    language,
                    handle.state::<InlineService>(),
                )
                .await?,
            )
        }
        "yuan_inline_edit" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_inline_commands::yuan_inline_edit(handle.state::<AppState>(), request).await?,
            )
        }
        // ===== git_commands =====
        "git_status" => {
            let request = parse_request(&args, "request")?;
            to_json(
                git_commands::git_status(handle.state::<AppState>(), request, handle.state::<GitService>())
                    .await?,
            )
        }
        "git_diff_file" => {
            let request = parse_request(&args, "request")?;
            to_json(
                git_commands::git_diff_file(handle.state::<AppState>(), request, handle.state::<GitService>())
                    .await?,
            )
        }
        "git_diff_unstaged" => {
            let request = parse_request(&args, "request")?;
            to_json(
                git_commands::git_diff_unstaged(handle.state::<AppState>(), request, handle.state::<GitService>())
                    .await?,
            )
        }
        "git_stage_file" => {
            let request = parse_request(&args, "request")?;
            to_json(
                git_commands::git_stage_file(handle.state::<AppState>(), request, handle.state::<GitService>())
                    .await?,
            )
        }
        "git_stage_all" => {
            let request = parse_request(&args, "request")?;
            to_json(
                git_commands::git_stage_all(handle.state::<AppState>(), request, handle.state::<GitService>())
                    .await?,
            )
        }
        "git_unstage_file" => {
            let request = parse_request(&args, "request")?;
            to_json(
                git_commands::git_unstage_file(handle.state::<AppState>(), request, handle.state::<GitService>())
                    .await?,
            )
        }
        "git_commit" => {
            let request = parse_request(&args, "request")?;
            to_json(
                git_commands::git_commit(handle.state::<AppState>(), request, handle.state::<GitService>())
                    .await?,
            )
        }
        "git_push" => {
            let request = parse_request(&args, "request")?;
            to_json(
                git_commands::git_push(handle.state::<AppState>(), request, handle.state::<GitService>())
                    .await?,
            )
        }
        "git_pull" => {
            let request = parse_request(&args, "request")?;
            to_json(
                git_commands::git_pull(handle.state::<AppState>(), request, handle.state::<GitService>())
                    .await?,
            )
        }
        "git_branches" => {
            let request = parse_request(&args, "request")?;
            to_json(
                git_commands::git_branches(handle.state::<AppState>(), request, handle.state::<GitService>())
                    .await?,
            )
        }
        "git_checkout" => {
            let request = parse_request(&args, "request")?;
            to_json(
                git_commands::git_checkout(handle.state::<AppState>(), request, handle.state::<GitService>())
                    .await?,
            )
        }
        "git_log" => {
            let request = parse_request(&args, "request")?;
            to_json(
                git_commands::git_log(handle.state::<AppState>(), request, handle.state::<GitService>())
                    .await?,
            )
        }
        "git_init" => {
            let request = parse_request(&args, "request")?;
            to_json(
                git_commands::git_init(handle.state::<AppState>(), request, handle.state::<GitService>())
                    .await?,
            )
        }
        // ===== lsp_commands =====
        "lsp_completions" => {
            let file_path = arg_str(&args, "file_path")?;
            let line = arg_u32(&args, "line")?;
            let character = arg_u32(&args, "character")?;
            let workspace_root = arg_str(&args, "workspace_root")?;
            to_json(
                lsp_commands::lsp_completions(
                    handle.state::<AppState>(),
                    handle.state::<lsp_commands::LspState>(),
                    file_path,
                    line,
                    character,
                    workspace_root,
                )
                .await?,
            )
        }
        "lsp_hover" => {
            let file_path = arg_str(&args, "file_path")?;
            let line = arg_u32(&args, "line")?;
            let character = arg_u32(&args, "character")?;
            let workspace_root = arg_str(&args, "workspace_root")?;
            to_json(
                lsp_commands::lsp_hover(
                    handle.state::<AppState>(),
                    handle.state::<lsp_commands::LspState>(),
                    file_path,
                    line,
                    character,
                    workspace_root,
                )
                .await?,
            )
        }
        "lsp_definition" => {
            let file_path = arg_str(&args, "file_path")?;
            let line = arg_u32(&args, "line")?;
            let character = arg_u32(&args, "character")?;
            let workspace_root = arg_str(&args, "workspace_root")?;
            to_json(
                lsp_commands::lsp_definition(
                    handle.state::<AppState>(),
                    handle.state::<lsp_commands::LspState>(),
                    file_path,
                    line,
                    character,
                    workspace_root,
                )
                .await?,
            )
        }
        "lsp_diagnostics" => {
            let file_path = arg_str(&args, "file_path")?;
            let workspace_root = arg_str(&args, "workspace_root")?;
            to_json(
                lsp_commands::lsp_diagnostics(
                    handle.state::<AppState>(),
                    handle.state::<lsp_commands::LspState>(),
                    file_path,
                    workspace_root,
                )
                .await?,
            )
        }
        "lsp_detect_language" => {
            let file_path = arg_str(&args, "file_path")?;
            to_json(
                lsp_commands::lsp_detect_language(
                    handle.state::<AppState>(),
                    handle.state::<lsp_commands::LspState>(),
                    file_path,
                )
                .await?,
            )
        }
        // ===== browser_commands =====
        "browser_open_window" => {
            let url = arg_str(&args, "url")?;
            to_json(
                browser_commands::browser_open_window(handle.state::<AppState>(), handle.clone(), url).await?,
            )
        }
        "browser_create_view" => {
            let url = arg_str(&args, "url")?;
            let x = arg_f64(&args, "x")?;
            let y = arg_f64(&args, "y")?;
            let width = arg_f64(&args, "width")?;
            let height = arg_f64(&args, "height")?;
            let window = handle
                .get_window("main")
                .ok_or_else(|| "主窗口未找到".to_string())?;
            to_json(
                browser_commands::browser_create_view(
                    handle.state::<AppState>(),
                    window,
                    url,
                    x,
                    y,
                    width,
                    height,
                )
                .await?,
            )
        }
        "browser_navigate_view" => {
            let label = arg_str(&args, "label")?;
            let url = arg_str(&args, "url")?;
            let window = handle
                .get_window("main")
                .ok_or_else(|| "主窗口未找到".to_string())?;
            to_json(
                browser_commands::browser_navigate_view(handle.state::<AppState>(), window, label, url).await?,
            )
        }
        "browser_resize_view" => {
            let label = arg_str(&args, "label")?;
            let x = arg_f64(&args, "x")?;
            let y = arg_f64(&args, "y")?;
            let width = arg_f64(&args, "width")?;
            let height = arg_f64(&args, "height")?;
            let window = handle
                .get_window("main")
                .ok_or_else(|| "主窗口未找到".to_string())?;
            to_json(
                browser_commands::browser_resize_view(
                    handle.state::<AppState>(),
                    window,
                    label,
                    x,
                    y,
                    width,
                    height,
                )
                .await?,
            )
        }
        "browser_close_view" => {
            let label = arg_str(&args, "label")?;
            let window = handle
                .get_window("main")
                .ok_or_else(|| "主窗口未找到".to_string())?;
            to_json(
                browser_commands::browser_close_view(handle.state::<AppState>(), window, label).await?,
            )
        }
        "browser_cleanup" => {
            let window = handle
                .get_window("main")
                .ok_or_else(|| "主窗口未找到".to_string())?;
            to_json(browser_commands::browser_cleanup(handle.state::<AppState>(), window).await?)
        }
        // ===== editor_commands（26，5a） =====
        "editor_open" => {
            let request = parse_request(&args, "request")?;
            to_json(editor_commands::editor_open(handle.state::<AppState>(), request).await?)
        }
        "editor_save" => {
            let request = parse_request(&args, "request")?;
            to_json(editor_commands::editor_save(handle.state::<AppState>(), request).await?)
        }
        "editor_auto_save" => {
            let request = parse_request(&args, "request")?;
            to_json(editor_commands::editor_auto_save(handle.state::<AppState>(), request).await?)
        }
        "editor_close" => {
            let doc_uuid = arg_str(&args, "doc_uuid")?;
            to_json(editor_commands::editor_close(handle.state::<AppState>(), doc_uuid).await?)
        }
        "editor_get_versions" => {
            let doc_uuid = arg_str(&args, "doc_uuid")?;
            to_json(editor_commands::editor_get_versions(handle.state::<AppState>(), doc_uuid).await?)
        }
        "editor_get_version" => {
            let doc_uuid = arg_str(&args, "doc_uuid")?;
            let version_num = arg_i64(&args, "version_num")?;
            to_json(
                editor_commands::editor_get_version(handle.state::<AppState>(), doc_uuid, version_num)
                    .await?,
            )
        }
        "editor_restore_version" => {
            let doc_uuid = arg_str(&args, "doc_uuid")?;
            let version_num = arg_i64(&args, "version_num")?;
            to_json(
                editor_commands::editor_restore_version(
                    handle.state::<AppState>(),
                    doc_uuid,
                    version_num,
                )
                .await?,
            )
        }
        "editor_recover_session" => {
            let doc_uuid = arg_str(&args, "doc_uuid")?;
            to_json(
                editor_commands::editor_recover_session(handle.state::<AppState>(), doc_uuid).await?,
            )
        }
        "editor_list_documents" => {
            to_json(editor_commands::editor_list_documents(handle.state::<AppState>()).await?)
        }
        "editor_delete_document" => {
            let doc_uuid = arg_str(&args, "doc_uuid")?;
            to_json(
                editor_commands::editor_delete_document(handle.state::<AppState>(), doc_uuid).await?,
            )
        }
        "editor_extract_metadata" => {
            let doc_uuid = arg_str(&args, "doc_uuid")?;
            let content_type = arg_str(&args, "content_type")?;
            to_json(
                editor_commands::editor_extract_metadata(
                    handle.state::<AppState>(),
                    doc_uuid,
                    content_type,
                )
                .await?,
            )
        }
        "editor_generate_thumbnail" => {
            let request = parse_request(&args, "request")?;
            to_json(
                editor_commands::editor_generate_thumbnail(handle.state::<AppState>(), request)
                    .await?,
            )
        }
        "editor_highlight" => {
            let request = parse_request(&args, "request")?;
            to_json(
                editor_commands::editor_highlight(handle.state::<AppState>(), request).await?,
            )
        }
        "editor_csv_preview" => {
            let request = parse_request(&args, "request")?;
            to_json(
                editor_commands::editor_csv_preview(handle.state::<AppState>(), request).await?,
            )
        }
        "editor_decrypted_preview" => {
            let doc_uuid = arg_str(&args, "doc_uuid")?;
            let content_type = arg_str(&args, "content_type")?;
            to_json(
                editor_commands::editor_decrypted_preview(
                    handle.state::<AppState>(),
                    doc_uuid,
                    content_type,
                )
                .await?,
            )
        }
        "editor_search" => {
            let request = parse_request(&args, "request")?;
            to_json(
                editor_commands::editor_search(handle.state::<AppState>(), request).await?,
            )
        }
        "editor_convert" => {
            let request = parse_request(&args, "request")?;
            to_json(
                editor_commands::editor_convert(handle.state::<AppState>(), request).await?,
            )
        }
        "editor_convert_content" => {
            let content = arg_str(&args, "content")?;
            let source_format = arg_str(&args, "source_format")?;
            let target_format = arg_str(&args, "target_format")?;
            to_json(
                editor_commands::editor_convert_content(
                    handle.state::<AppState>(),
                    content,
                    source_format,
                    target_format,
                )
                .await?,
            )
        }
        "editor_canvas_save" => {
            let request = parse_request(&args, "request")?;
            to_json(
                editor_commands::editor_canvas_save(handle.state::<AppState>(), request).await?,
            )
        }
        "editor_canvas_load" => {
            let doc_uuid = arg_str(&args, "doc_uuid")?;
            to_json(
                editor_commands::editor_canvas_load(handle.state::<AppState>(), doc_uuid).await?,
            )
        }
        "editor_canvas_delete" => {
            let doc_uuid = arg_str(&args, "doc_uuid")?;
            to_json(
                editor_commands::editor_canvas_delete(handle.state::<AppState>(), doc_uuid).await?,
            )
        }
        "editor_diff_versions" => {
            let request = parse_request(&args, "request")?;
            to_json(
                editor_commands::editor_diff_versions(handle.state::<AppState>(), request).await?,
            )
        }
        "editor_cleanup_versions" => {
            let request = parse_request(&args, "request")?;
            to_json(
                editor_commands::editor_cleanup_versions(handle.state::<AppState>(), request)
                    .await?,
            )
        }
        "editor_index_content" => {
            let doc_uuid = arg_str(&args, "doc_uuid")?;
            let title = arg_str(&args, "title")?;
            let content_type = arg_str(&args, "content_type")?;
            let content = arg_str(&args, "content")?;
            to_json(
                editor_commands::editor_index_content(
                    handle.state::<AppState>(),
                    doc_uuid,
                    title,
                    content_type,
                    content,
                )
                .await?,
            )
        }
        "editor_recover_all_sessions" => {
            to_json(
                editor_commands::editor_recover_all_sessions(handle.state::<AppState>()).await?,
            )
        }
        "editor_cleanup_sessions" => {
            let max_age_hours = arg_opt_i64(&args, "max_age_hours")?;
            to_json(
                editor_commands::editor_cleanup_sessions(
                    handle.state::<AppState>(),
                    max_age_hours,
                )
                .await?,
            )
        }
        // ===== file_edit_commands（14，5a） =====
        "fileedit_read" => {
            let request = parse_request(&args, "request")?;
            to_json(file_edit_commands::fileedit_read(handle.state::<AppState>(), request).await?)
        }
        "fileedit_write" => {
            let request = parse_request(&args, "request")?;
            to_json(file_edit_commands::fileedit_write(handle.state::<AppState>(), request).await?)
        }
        "fileedit_get_status" => {
            let path = arg_str(&args, "path")?;
            to_json(
                file_edit_commands::fileedit_get_status(handle.state::<AppState>(), path).await?,
            )
        }
        "tableedit_read" => {
            let request = parse_request(&args, "request")?;
            to_json(file_edit_commands::tableedit_read(handle.state::<AppState>(), request).await?)
        }
        "tableedit_write" => {
            let request = parse_request(&args, "request")?;
            to_json(file_edit_commands::tableedit_write(handle.state::<AppState>(), request).await?)
        }
        "tableedit_export_csv" => {
            let request = parse_request(&args, "request")?;
            to_json(
                file_edit_commands::tableedit_export_csv(handle.state::<AppState>(), request)
                    .await?,
            )
        }
        "pptedit_get_slides" => {
            let path = arg_str(&args, "path")?;
            to_json(
                file_edit_commands::pptedit_get_slides(handle.state::<AppState>(), path).await?,
            )
        }
        "pptedit_update_slide" => {
            let request = parse_request(&args, "request")?;
            to_json(
                file_edit_commands::pptedit_update_slide(handle.state::<AppState>(), request)
                    .await?,
            )
        }
        "pptedit_add_slide" => {
            let path = arg_str(&args, "path")?;
            to_json(
                file_edit_commands::pptedit_add_slide(handle.state::<AppState>(), path).await?,
            )
        }
        "pptedit_delete_slide" => {
            let path = arg_str(&args, "path")?;
            let slide_index = arg_u32(&args, "slide_index")?;
            to_json(
                file_edit_commands::pptedit_delete_slide(
                    handle.state::<AppState>(),
                    path,
                    slide_index as usize,
                )
                .await?,
            )
        }
        "pptedit_reorder" => {
            let request = parse_request(&args, "request")?;
            to_json(
                file_edit_commands::pptedit_reorder(handle.state::<AppState>(), request).await?,
            )
        }
        "pdfedit_save" => {
            let request = parse_request(&args, "request")?;
            to_json(file_edit_commands::pdfedit_save(handle.state::<AppState>(), request).await?)
        }
        "imageedit_save" => {
            let request = parse_request(&args, "request")?;
            to_json(file_edit_commands::imageedit_save(handle.state::<AppState>(), request).await?)
        }
        "audioedit_save" => {
            let request = parse_request(&args, "request")?;
            to_json(file_edit_commands::audioedit_save(handle.state::<AppState>(), request).await?)
        }
        // ===== tools_commands（8，5a）=====
        "yuan_tools_list" => {
            to_json(
                tools_commands::yuan_tools_list(handle.state::<AppState>(), handle.state::<ToolState>())
                    .await?,
            )
        }
        "yuan_tools_search" => {
            let request = parse_request(&args, "request")?;
            to_json(
                tools_commands::yuan_tools_search(
                    handle.state::<AppState>(),
                    handle.state::<ToolState>(),
                    request,
                )
                .await?,
            )
        }
        "yuan_tools_call" => {
            let request = parse_request(&args, "request")?;
            to_json(
                tools_commands::yuan_tools_call(
                    handle.state::<AppState>(),
                    handle.state::<ToolState>(),
                    request,
                )
                .await?,
            )
        }
        "yuan_plan_create" => {
            let title = arg_str(&args, "title")?;
            let description = arg_str(&args, "description")?;
            let steps: Vec<serde_json::Value> = match get(&args, "steps") {
                Some(v) if !v.is_null() => serde_json::from_value(v.clone()).unwrap_or_default(),
                _ => vec![],
            };
            let session_id = arg_str(&args, "session_id")?;
            let parsed_steps: Vec<crate::models::tool::PlanStep> = serde_json::from_value(Json::Array(steps))
                .unwrap_or_default();
            to_json(
                tools_commands::yuan_plan_create(
                    handle.state::<AppState>(),
                    handle.state::<ToolState>(),
                    title,
                    description,
                    parsed_steps,
                    session_id,
                )
                .await?,
            )
        }
        "yuan_plan_update_step" => {
            let plan_id = arg_str(&args, "plan_id")?;
            let step_id = arg_str(&args, "step_id")?;
            let status = arg_str(&args, "status")?;
            let result = arg_opt_str(&args, "result")?;
            to_json(
                tools_commands::yuan_plan_update_step(
                    handle.state::<AppState>(),
                    handle.state::<ToolState>(),
                    plan_id,
                    step_id,
                    status,
                    result,
                )
                .await?,
            )
        }
        "yuan_plan_get" => {
            let plan_id = arg_str(&args, "plan_id")?;
            to_json(
                tools_commands::yuan_plan_get(
                    handle.state::<AppState>(),
                    handle.state::<ToolState>(),
                    plan_id,
                )
                .await?,
            )
        }
        "yuan_plan_list_all" => {
            to_json(
                tools_commands::yuan_plan_list_all(handle.state::<AppState>(), handle.state::<ToolState>())
                    .await?,
            )
        }
        "yuan_apply_patch" => {
            let request = parse_request(&args, "request")?;
            to_json(
                tools_commands::yuan_apply_patch(handle.state::<AppState>(), handle.state::<ToolState>(), request)
                    .await?,
            )
        }
        // ===== distill_commands（8，5a，T2 前置）=====
        // distill_命令 State 参数在末尾（遵循原始命令定义顺序），distill_local_infer 为 State 首参。
        "distill_extract_samples" => {
            let source = arg_str(&args, "source")?;
            let limit = arg_opt_i64(&args, "limit")?;
            to_json(
                distill_commands::distill_extract_samples(source, limit, handle.state::<AppState>())
                    .await?,
            )
        }
        "distill_generate_dataset" => {
            let request = parse_request(&args, "request")?;
            to_json(
                distill_commands::distill_generate_dataset(request, handle.state::<AppState>())
                    .await?,
            )
        }
        "distill_list_samples" => {
            let request = parse_request(&args, "request")?;
            to_json(
                distill_commands::distill_list_samples(request, handle.state::<AppState>()).await?,
            )
        }
        "distill_approve_sample" => {
            let id = arg_i64(&args, "id")?;
            let quality_score = arg_opt_f64(&args, "quality_score")?;
            to_json(
                distill_commands::distill_approve_sample(id, quality_score, handle.state::<AppState>())
                    .await?,
            )
        }
        "distill_reject_sample" => {
            let id = arg_i64(&args, "id")?;
            to_json(distill_commands::distill_reject_sample(id, handle.state::<AppState>()).await?)
        }
        "distill_local_infer" => {
            let prompt = arg_str(&args, "prompt")?;
            let system_prompt = arg_opt_str(&args, "system_prompt")?;
            to_json(
                distill_commands::distill_local_infer(handle.state::<AppState>(), prompt, system_prompt)
                    .await?,
            )
        }
        "distill_export_jsonl" => {
            let output_path = arg_str(&args, "output_path")?;
            let status_filter = arg_opt_str(&args, "status_filter")?;
            to_json(
                distill_commands::distill_export_jsonl(
                    output_path,
                    status_filter,
                    handle.state::<AppState>(),
                )
                .await?,
            )
        }
        "distill_sample_stats" => {
            to_json(distill_commands::distill_sample_stats(handle.state::<AppState>()).await?)
        }
        // ===== yuan_sandbox_commands（14，5b）=====
        "yuan_sandbox_create" => {
            let config = parse_request(&args, "config")?;
            to_json(
                yuan_sandbox_commands::yuan_sandbox_create(
                    handle.state::<AppState>(),
                    config,
                    handle.state::<SandboxService>(),
                )
                .await?,
            )
        }
        "yuan_sandbox_list" => {
            to_json(
                yuan_sandbox_commands::yuan_sandbox_list(
                    handle.state::<AppState>(),
                    handle.state::<SandboxService>(),
                )
                .await?,
            )
        }
        "yuan_sandbox_list_by_agent" => {
            let agent_id = arg_str(&args, "agent_id")?;
            to_json(
                yuan_sandbox_commands::yuan_sandbox_list_by_agent(
                    handle.state::<AppState>(),
                    agent_id,
                    handle.state::<SandboxService>(),
                )
                .await?,
            )
        }
        "yuan_sandbox_get" => {
            let sandbox_id = arg_str(&args, "sandbox_id")?;
            to_json(
                yuan_sandbox_commands::yuan_sandbox_get(
                    handle.state::<AppState>(),
                    sandbox_id,
                    handle.state::<SandboxService>(),
                )
                .await?,
            )
        }
        "yuan_sandbox_execute" => {
            let sandbox_id = arg_str(&args, "sandbox_id")?;
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_sandbox_commands::yuan_sandbox_execute(
                    handle.state::<AppState>(),
                    sandbox_id,
                    request,
                    handle.state::<SandboxService>(),
                )
                .await?,
            )
        }
        "yuan_sandbox_read_file" => {
            let sandbox_id = arg_str(&args, "sandbox_id")?;
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_sandbox_commands::yuan_sandbox_read_file(
                    handle.state::<AppState>(),
                    sandbox_id,
                    request,
                    handle.state::<SandboxService>(),
                )
                .await?,
            )
        }
        "yuan_sandbox_write_file" => {
            let sandbox_id = arg_str(&args, "sandbox_id")?;
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_sandbox_commands::yuan_sandbox_write_file(
                    handle.state::<AppState>(),
                    sandbox_id,
                    request,
                    handle.state::<SandboxService>(),
                )
                .await?,
            )
        }
        "yuan_sandbox_delete_file" => {
            let sandbox_id = arg_str(&args, "sandbox_id")?;
            let path = arg_str(&args, "path")?;
            to_json(
                yuan_sandbox_commands::yuan_sandbox_delete_file(
                    handle.state::<AppState>(),
                    sandbox_id,
                    path,
                    handle.state::<SandboxService>(),
                )
                .await?,
            )
        }
        "yuan_sandbox_list_files" => {
            let sandbox_id = arg_str(&args, "sandbox_id")?;
            let path = arg_str(&args, "path")?;
            to_json(
                yuan_sandbox_commands::yuan_sandbox_list_files(
                    handle.state::<AppState>(),
                    sandbox_id,
                    path,
                    handle.state::<SandboxService>(),
                )
                .await?,
            )
        }
        "yuan_sandbox_terminate" => {
            let sandbox_id = arg_str(&args, "sandbox_id")?;
            to_json(
                yuan_sandbox_commands::yuan_sandbox_terminate(
                    handle.state::<AppState>(),
                    sandbox_id,
                    handle.state::<SandboxService>(),
                )
                .await?,
            )
        }
        "yuan_sandbox_cleanup" => {
            to_json(
                yuan_sandbox_commands::yuan_sandbox_cleanup(
                    handle.state::<AppState>(),
                    handle.state::<SandboxService>(),
                )
                .await?,
            )
        }
        "yuan_sandbox_count" => {
            to_json(
                yuan_sandbox_commands::yuan_sandbox_count(
                    handle.state::<AppState>(),
                    handle.state::<SandboxService>(),
                )
                .await?,
            )
        }
        "yuan_sandbox_save" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_sandbox_commands::yuan_sandbox_save(
                    handle.state::<AppState>(),
                    request,
                )
                .await?,
            )
        }
        "yuan_sandbox_history" => {
            to_json(
                yuan_sandbox_commands::yuan_sandbox_history(
                    handle.state::<AppState>(),
                    handle.state::<SandboxService>(),
                )
                .await?,
            )
        }
        // ===== engine_commands（12，5b）=====
        "engine_create_session" => {
            let model = arg_opt_str(&args, "model")?;
            let system_prompt = arg_opt_str(&args, "system_prompt")?;
            let workspace_path = arg_opt_str(&args, "workspace_path")?;
            let token_budget = arg_opt_i64(&args, "token_budget")?.map(|v| v as u64);
            let sandbox_enabled = arg_opt_bool(&args, "sandbox_enabled")?;
            to_json(
                engine_commands::engine_create_session(
                    handle.state::<AppState>(),
                    handle.state::<EngineState>(),
                    model,
                    system_prompt,
                    workspace_path,
                    token_budget,
                    sandbox_enabled,
                )
                .await?,
            )
        }
        "engine_get_session" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(
                engine_commands::engine_get_session(
                    handle.state::<AppState>(),
                    handle.state::<EngineState>(),
                    session_id,
                )
                .await?,
            )
        }
        "engine_list_sessions" => {
            to_json(
                engine_commands::engine_list_sessions(
                    handle.state::<AppState>(),
                    handle.state::<EngineState>(),
                )
                .await?,
            )
        }
        "engine_destroy_session" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(
                engine_commands::engine_destroy_session(
                    handle.state::<AppState>(),
                    handle.state::<EngineState>(),
                    session_id,
                )
                .await?,
            )
        }
        "engine_start_turn" => {
            let session_id = arg_str(&args, "session_id")?;
            let user_input = arg_str(&args, "user_input")?;
            to_json(
                engine_commands::engine_start_turn(
                    handle.state::<AppState>(),
                    handle.state::<EngineState>(),
                    session_id,
                    user_input,
                )
                .await?,
            )
        }
        "engine_complete_turn" => {
            let session_id = arg_str(&args, "session_id")?;
            let response = arg_str(&args, "response")?;
            let input_tokens = arg_opt_i64(&args, "input_tokens")?.map(|v| v as u64);
            let output_tokens = arg_opt_i64(&args, "output_tokens")?.map(|v| v as u64);
            to_json(
                engine_commands::engine_complete_turn(
                    handle.state::<AppState>(),
                    handle.state::<EngineState>(),
                    session_id,
                    response,
                    input_tokens,
                    output_tokens,
                )
                .await?,
            )
        }
        "engine_abort_turn" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(
                engine_commands::engine_abort_turn(
                    handle.state::<AppState>(),
                    handle.state::<EngineState>(),
                    session_id,
                )
                .await?,
            )
        }
        "engine_pause_session" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(
                engine_commands::engine_pause_session(
                    handle.state::<AppState>(),
                    handle.state::<EngineState>(),
                    session_id,
                )
                .await?,
            )
        }
        "engine_resume_session" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(
                engine_commands::engine_resume_session(
                    handle.state::<AppState>(),
                    handle.state::<EngineState>(),
                    session_id,
                )
                .await?,
            )
        }
        "engine_get_turns" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(
                engine_commands::engine_get_turns(
                    handle.state::<AppState>(),
                    handle.state::<EngineState>(),
                    session_id,
                )
                .await?,
            )
        }
        "engine_stats" => {
            to_json(
                engine_commands::engine_stats(
                    handle.state::<AppState>(),
                    handle.state::<EngineState>(),
                )
                .await?,
            )
        }
        "engine_subscribe_events" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(
                engine_commands::engine_subscribe_events(
                    handle.state::<AppState>(),
                    handle.clone(),
                    handle.state::<EngineState>(),
                    session_id,
                )
                .await?,
            )
        }
        // ===== yuan_agent_commands（15，5b）=====
        "yuan_agent_spawn" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_agent_commands::yuan_agent_spawn(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<AgentService>(),
                )
                .await?,
            )
        }
        "yuan_agent_fork" => {
            let parent_agent_id = arg_str(&args, "parent_agent_id")?;
            let new_role = match get(&args, "new_role") {
                Some(v) if !v.is_null() => Some(parse_request(&args, "new_role")?),
                _ => None,
            };
            to_json(
                yuan_agent_commands::yuan_agent_fork(
                    handle.state::<AppState>(),
                    parent_agent_id,
                    new_role,
                    handle.state::<AgentService>(),
                )
                .await?,
            )
        }
        "yuan_agent_list" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(
                yuan_agent_commands::yuan_agent_list(
                    handle.state::<AppState>(),
                    session_id,
                    handle.state::<AgentService>(),
                )
                .await?,
            )
        }
        "yuan_agent_list_all" => {
            to_json(
                yuan_agent_commands::yuan_agent_list_all(
                    handle.state::<AppState>(),
                    handle.state::<AgentService>(),
                )
                .await?,
            )
        }
        "yuan_agent_status" => {
            let agent_id = arg_str(&args, "agent_id")?;
            to_json(
                yuan_agent_commands::yuan_agent_status(
                    handle.state::<AppState>(),
                    agent_id,
                    handle.state::<AgentService>(),
                )
                .await?,
            )
        }
        "yuan_agent_abort" => {
            let agent_id = arg_str(&args, "agent_id")?;
            let reason = arg_str(&args, "reason")?;
            to_json(
                yuan_agent_commands::yuan_agent_abort(
                    handle.state::<AppState>(),
                    agent_id,
                    reason,
                    handle.state::<AgentService>(),
                )
                .await?,
            )
        }
        "yuan_agent_transition" => {
            let agent_id = arg_str(&args, "agent_id")?;
            let target_status = arg_str(&args, "target_status")?;
            to_json(
                yuan_agent_commands::yuan_agent_transition(
                    handle.state::<AppState>(),
                    agent_id,
                    target_status,
                    handle.state::<AgentService>(),
                )
                .await?,
            )
        }
        "yuan_agent_send_message" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_agent_commands::yuan_agent_send_message(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<AgentService>(),
                )
                .await?,
            )
        }
        "yuan_agent_receive_messages" => {
            let agent_id = arg_str(&args, "agent_id")?;
            to_json(
                yuan_agent_commands::yuan_agent_receive_messages(
                    handle.state::<AppState>(),
                    agent_id,
                    handle.state::<AgentService>(),
                )
                .await?,
            )
        }
        "yuan_agent_list_roles" => {
            to_json(
                yuan_agent_commands::yuan_agent_list_roles(
                    handle.state::<AppState>(),
                    handle.state::<AgentService>(),
                )
                .await?,
            )
        }
        "yuan_agent_get_count" => {
            to_json(
                yuan_agent_commands::yuan_agent_get_count(
                    handle.state::<AppState>(),
                    handle.state::<AgentService>(),
                )
                .await?,
            )
        }
        "yuan_agent_cleanup" => {
            to_json(
                yuan_agent_commands::yuan_agent_cleanup(
                    handle.state::<AppState>(),
                    handle.state::<AgentService>(),
                )
                .await?,
            )
        }
        "yuan_agent_list_templates" => {
            to_json(
                yuan_agent_commands::yuan_agent_list_templates(
                    handle.state::<AppState>(),
                )
                .await?,
            )
        }
        "yuan_agent_deploy" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_agent_commands::yuan_agent_deploy(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<AgentService>(),
                )
                .await?,
            )
        }
        "yuan_agent_execute" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_agent_commands::yuan_agent_execute(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<AgentService>(),
                )
                .await?,
            )
        }
        // ===== yuan_safety_commands（5，5b）=====
        "yuan_safety_check" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_safety_commands::yuan_safety_check(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<SafetyService>(),
                )
                .await?,
            )
        }
        "yuan_safety_quick_check" => {
            let content = arg_str(&args, "content")?;
            to_json(
                yuan_safety_commands::yuan_safety_quick_check(
                    handle.state::<AppState>(),
                    content,
                    handle.state::<SafetyService>(),
                )
                .await?,
            )
        }
        "yuan_safety_set_profile" => {
            let profile = parse_request(&args, "profile")?;
            to_json(
                yuan_safety_commands::yuan_safety_set_profile(
                    handle.state::<AppState>(),
                    profile,
                    handle.state::<SafetyService>(),
                )
                .await?,
            )
        }
        "yuan_safety_get_profile" => {
            to_json(
                yuan_safety_commands::yuan_safety_get_profile(
                    handle.state::<AppState>(),
                    handle.state::<SafetyService>(),
                )
                .await?,
            )
        }
        "yuan_safety_list_profiles" => {
            to_json(
                yuan_safety_commands::yuan_safety_list_profiles(
                    handle.state::<AppState>(),
                )
                .await?,
            )
        }
        // ===== yuan_goal_commands（14，5c）=====
        "yuan_goal_create" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_goal_commands::yuan_goal_create(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<GoalService>(),
                )
                .await?,
            )
        }
        "yuan_goal_get" => {
            let goal_id = arg_i64(&args, "goal_id")?;
            to_json(
                yuan_goal_commands::yuan_goal_get(
                    handle.state::<AppState>(),
                    goal_id,
                    handle.state::<GoalService>(),
                )
                .await?,
            )
        }
        "yuan_goal_list" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(
                yuan_goal_commands::yuan_goal_list(
                    handle.state::<AppState>(),
                    session_id,
                    handle.state::<GoalService>(),
                )
                .await?,
            )
        }
        "yuan_goal_update" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_goal_commands::yuan_goal_update(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<GoalService>(),
                )
                .await?,
            )
        }
        "yuan_goal_start" => {
            let goal_id = arg_i64(&args, "goal_id")?;
            to_json(
                yuan_goal_commands::yuan_goal_start(
                    handle.state::<AppState>(),
                    goal_id,
                    handle.state::<GoalService>(),
                )
                .await?,
            )
        }
        "yuan_goal_pause" => {
            let goal_id = arg_i64(&args, "goal_id")?;
            to_json(
                yuan_goal_commands::yuan_goal_pause(
                    handle.state::<AppState>(),
                    goal_id,
                    handle.state::<GoalService>(),
                )
                .await?,
            )
        }
        "yuan_goal_complete" => {
            let goal_id = arg_i64(&args, "goal_id")?;
            let result_json = arg_opt_str(&args, "result_json")?;
            to_json(
                yuan_goal_commands::yuan_goal_complete(
                    handle.state::<AppState>(),
                    goal_id,
                    result_json,
                    handle.state::<GoalService>(),
                )
                .await?,
            )
        }
        "yuan_goal_abort" => {
            let goal_id = arg_i64(&args, "goal_id")?;
            let reason = arg_str(&args, "reason")?;
            to_json(
                yuan_goal_commands::yuan_goal_abort(
                    handle.state::<AppState>(),
                    goal_id,
                    reason,
                    handle.state::<GoalService>(),
                )
                .await?,
            )
        }
        "yuan_goal_delete" => {
            let goal_id = arg_i64(&args, "goal_id")?;
            to_json(
                yuan_goal_commands::yuan_goal_delete(
                    handle.state::<AppState>(),
                    goal_id,
                    handle.state::<GoalService>(),
                )
                .await?,
            )
        }
        "yuan_goal_consume_tokens" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_goal_commands::yuan_goal_consume_tokens(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<GoalService>(),
                )
                .await?,
            )
        }
        "yuan_goal_update_progress" => {
            let goal_id = arg_i64(&args, "goal_id")?;
            let progress_pct = arg_i64(&args, "progress_pct")? as i32;
            to_json(
                yuan_goal_commands::yuan_goal_update_progress(
                    handle.state::<AppState>(),
                    goal_id,
                    progress_pct,
                    handle.state::<GoalService>(),
                )
                .await?,
            )
        }
        "yuan_goal_save_checkpoint" => {
            let goal_id = arg_i64(&args, "goal_id")?;
            let agent_id = arg_opt_str(&args, "agent_id")?;
            let last_message = arg_str(&args, "last_message")?;
            let turn_count = arg_u32(&args, "turn_count")?;
            let tokens_used = arg_i64(&args, "tokens_used")?;
            let sandbox_id = arg_opt_str(&args, "sandbox_id")?;
            to_json(
                yuan_goal_commands::yuan_goal_save_checkpoint(
                    handle.state::<AppState>(),
                    goal_id,
                    agent_id,
                    last_message,
                    turn_count,
                    tokens_used,
                    sandbox_id,
                    handle.state::<GoalService>(),
                )
                .await?,
            )
        }
        "yuan_goal_build_continuation" => {
            let goal_id = arg_i64(&args, "goal_id")?;
            to_json(
                yuan_goal_commands::yuan_goal_build_continuation(
                    handle.state::<AppState>(),
                    goal_id,
                    handle.state::<GoalService>(),
                )
                .await?,
            )
        }
        "yuan_goal_checkpoints_count" => {
            let goal_id = arg_i64(&args, "goal_id")?;
            to_json(
                yuan_goal_commands::yuan_goal_checkpoints_count(
                    handle.state::<AppState>(),
                    goal_id,
                    handle.state::<GoalService>(),
                )
                .await?,
            )
        }
        // ===== yuan_compact_commands（7，5c）=====
        "yuan_compact_config_get" => {
            to_json(
                yuan_compact_commands::yuan_compact_config_get(
                    handle.state::<AppState>(),
                    handle.state::<CompactService>(),
                )
                .await?,
            )
        }
        "yuan_compact_config_update" => {
            let config = parse_request(&args, "config")?;
            to_json(
                yuan_compact_commands::yuan_compact_config_update(
                    handle.state::<AppState>(),
                    config,
                    handle.state::<CompactService>(),
                )
                .await?,
            )
        }
        "yuan_compact_estimate" => {
            let messages: Vec<crate::models::compact::ConversationMessage> =
                parse_request(&args, "messages")?;
            to_json(
                yuan_compact_commands::yuan_compact_estimate(
                    handle.state::<AppState>(),
                    messages,
                    handle.state::<CompactService>(),
                )
                .await?,
            )
        }
        "yuan_compact_check" => {
            let messages: Vec<crate::models::compact::ConversationMessage> =
                parse_request(&args, "messages")?;
            to_json(
                yuan_compact_commands::yuan_compact_check(
                    handle.state::<AppState>(),
                    messages,
                    handle.state::<CompactService>(),
                )
                .await?,
            )
        }
        "yuan_compact_execute" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_compact_commands::yuan_compact_execute(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<CompactService>(),
                )
                .await?,
            )
        }
        "yuan_compact_session" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(
                yuan_compact_commands::yuan_compact_session(
                    handle.state::<AppState>(),
                    session_id,
                    handle.state::<CompactService>(),
                )
                .await?,
            )
        }
        "yuan_compact_reset" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(
                yuan_compact_commands::yuan_compact_reset(
                    handle.state::<AppState>(),
                    session_id,
                    handle.state::<CompactService>(),
                )
                .await?,
            )
        }
        // ===== yuan_io_control_commands（8，5c）=====
        "yuan_io_config_get" => {
            to_json(
                yuan_io_control_commands::yuan_io_config_get(
                    handle.state::<AppState>(),
                    handle.state::<IoControlService>(),
                )
                .await?,
            )
        }
        "yuan_io_config_update" => {
            let config = parse_request(&args, "config")?;
            to_json(
                yuan_io_control_commands::yuan_io_config_update(
                    handle.state::<AppState>(),
                    config,
                    handle.state::<IoControlService>(),
                )
                .await?,
            )
        }
        "yuan_io_stats" => {
            to_json(
                yuan_io_control_commands::yuan_io_stats(
                    handle.state::<AppState>(),
                    handle.state::<IoControlService>(),
                )
                .await?,
            )
        }
        "yuan_io_execute" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_io_control_commands::yuan_io_execute(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<IoControlService>(),
                )
                .await?,
            )
        }
        "yuan_io_cancel" => {
            let execution_id = arg_str(&args, "execution_id")?;
            to_json(
                yuan_io_control_commands::yuan_io_cancel(
                    handle.state::<AppState>(),
                    execution_id,
                    handle.state::<IoControlService>(),
                )
                .await?,
            )
        }
        "yuan_io_cancel_all" => {
            to_json(
                yuan_io_control_commands::yuan_io_cancel_all(
                    handle.state::<AppState>(),
                    handle.state::<IoControlService>(),
                )
                .await?,
            )
        }
        "yuan_io_kill" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_io_control_commands::yuan_io_kill(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<IoControlService>(),
                )
                .await?,
            )
        }
        "yuan_io_stdin" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_io_control_commands::yuan_io_stdin(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<IoControlService>(),
                )
                .await?,
            )
        }
        // ===== yuan_prompt_commands（12，5c）=====
        "yuan_prompt_list_templates" => {
            to_json(
                yuan_prompt_commands::yuan_prompt_list_templates(
                    handle.state::<AppState>(),
                    handle.state::<PromptService>(),
                )
                .await?,
            )
        }
        "yuan_prompt_get_template" => {
            let template_type = arg_str(&args, "template_type")?;
            to_json(
                yuan_prompt_commands::yuan_prompt_get_template(
                    handle.state::<AppState>(),
                    template_type,
                    handle.state::<PromptService>(),
                )
                .await?,
            )
        }
        "yuan_prompt_render" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_prompt_commands::yuan_prompt_render(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<PromptService>(),
                )
                .await?,
            )
        }
        "yuan_prompt_set_custom_template" => {
            let template_type = arg_str(&args, "template_type")?;
            let content = arg_str(&args, "content")?;
            to_json(
                yuan_prompt_commands::yuan_prompt_set_custom_template(
                    handle.state::<AppState>(),
                    template_type,
                    content,
                    handle.state::<PromptService>(),
                )
                .await?,
            )
        }
        "yuan_prompt_remove_custom_template" => {
            let template_type = arg_str(&args, "template_type")?;
            to_json(
                yuan_prompt_commands::yuan_prompt_remove_custom_template(
                    handle.state::<AppState>(),
                    template_type,
                    handle.state::<PromptService>(),
                )
                .await?,
            )
        }
        "yuan_prompt_set_variable_default" => {
            let name = arg_str(&args, "name")?;
            let value = arg_str(&args, "value")?;
            to_json(
                yuan_prompt_commands::yuan_prompt_set_variable_default(
                    handle.state::<AppState>(),
                    name,
                    value,
                    handle.state::<PromptService>(),
                )
                .await?,
            )
        }
        "yuan_agents_discover" => {
            let cwd = arg_opt_str(&args, "cwd")?;
            let project_root = arg_opt_str(&args, "project_root")?;
            to_json(
                yuan_prompt_commands::yuan_agents_discover(
                    handle.state::<AppState>(),
                    cwd,
                    project_root,
                    handle.state::<PromptService>(),
                )
                .await?,
            )
        }
        "yuan_agents_sources" => {
            let cwd = arg_opt_str(&args, "cwd")?;
            let project_root = arg_opt_str(&args, "project_root")?;
            to_json(
                yuan_prompt_commands::yuan_agents_sources(
                    handle.state::<AppState>(),
                    cwd,
                    project_root,
                    handle.state::<PromptService>(),
                )
                .await?,
            )
        }
        "yuan_agents_assemble" => {
            let request = parse_request(&args, "agents_files")?;
            let user_instructions = arg_opt_str(&args, "user_instructions")?;
            let max_bytes = match get(&args, "max_bytes") {
                Some(v) if !v.is_null() => Some(v.as_u64().unwrap_or(0) as usize),
                _ => None,
            };
            to_json(
                yuan_prompt_commands::yuan_agents_assemble(
                    handle.state::<AppState>(),
                    request,
                    user_instructions,
                    max_bytes,
                    handle.state::<PromptService>(),
                )
                .await?,
            )
        }
        "yuan_agents_set_max_bytes" => {
            let max_bytes = arg_u32(&args, "max_bytes")? as usize;
            to_json(
                yuan_prompt_commands::yuan_agents_set_max_bytes(
                    handle.state::<AppState>(),
                    max_bytes,
                    handle.state::<PromptService>(),
                )
                .await?,
            )
        }
        "yuan_agents_get_max_bytes" => {
            to_json(
                yuan_prompt_commands::yuan_agents_get_max_bytes(
                    handle.state::<AppState>(),
                    handle.state::<PromptService>(),
                )
                .await?,
            )
        }
        "yuan_prompt_assemble" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_prompt_commands::yuan_prompt_assemble(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<PromptService>(),
                )
                .await?,
            )
        }
        // ===== yuan_mcp_commands（40，5d）=====
        "yuan_mcp_register_server" => {
            let server = parse_request(&args, "server")?;
            let auto_connect = arg_opt_bool(&args, "auto_connect")?.unwrap_or(false);
            to_json(
                yuan_mcp_commands::yuan_mcp_register_server(
                    handle.state::<AppState>(),
                    server,
                    auto_connect,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_connect_server" => {
            let server_id = arg_str(&args, "server_id")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_connect_server(
                    handle.state::<AppState>(),
                    server_id,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_disconnect_server" => {
            let server_id = arg_str(&args, "server_id")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_disconnect_server(
                    handle.state::<AppState>(),
                    server_id,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_list_servers" => {
            to_json(
                yuan_mcp_commands::yuan_mcp_list_servers(
                    handle.state::<AppState>(),
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_list_all_tools" => {
            to_json(
                yuan_mcp_commands::yuan_mcp_list_all_tools(
                    handle.state::<AppState>(),
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_server_status" => {
            let server_id = arg_str(&args, "server_id")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_server_status(
                    handle.state::<AppState>(),
                    server_id,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_call_tool" => {
            let server_id = arg_str(&args, "server_id")?;
            let tool_name = arg_str(&args, "tool_name")?;
            let arguments = match get(&args, "arguments") {
                Some(v) if !v.is_null() => Some(v.clone()),
                _ => None,
            };
            to_json(
                yuan_mcp_commands::yuan_mcp_call_tool(
                    handle.state::<AppState>(),
                    server_id,
                    tool_name,
                    arguments,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_spawn" => {
            let server_id = arg_str(&args, "server_id")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_spawn(
                    handle.state::<AppState>(),
                    server_id,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_health_check" => {
            let server_id = arg_str(&args, "server_id")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_health_check(
                    handle.state::<AppState>(),
                    server_id,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_health_check_all" => {
            to_json(
                yuan_mcp_commands::yuan_mcp_health_check_all(
                    handle.state::<AppState>(),
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_set_lifecycle_config" => {
            let server_id = arg_str(&args, "server_id")?;
            let config = parse_request(&args, "config")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_set_lifecycle_config(
                    handle.state::<AppState>(),
                    server_id,
                    config,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_get_lifecycle_state" => {
            let server_id = arg_str(&args, "server_id")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_get_lifecycle_state(
                    handle.state::<AppState>(),
                    server_id,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_list_shaped_tools" => {
            to_json(
                yuan_mcp_commands::yuan_mcp_list_shaped_tools(
                    handle.state::<AppState>(),
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_group_tools_by_namespace" => {
            to_json(
                yuan_mcp_commands::yuan_mcp_group_tools_by_namespace(
                    handle.state::<AppState>(),
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_list_tools_by_namespace" => {
            let namespace = arg_str(&args, "namespace")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_list_tools_by_namespace(
                    handle.state::<AppState>(),
                    namespace,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_get_tool_namespaces" => {
            to_json(
                yuan_mcp_commands::yuan_mcp_get_tool_namespaces(
                    handle.state::<AppState>(),
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_set_shaper_config" => {
            let preferred_servers = match get(&args, "preferred_servers") {
                Some(v) if !v.is_null() => Some(serde_json::from_value(v.clone()).unwrap_or_default()),
                _ => None,
            };
            let enable_namespace = arg_opt_bool(&args, "enable_namespace")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_set_shaper_config(
                    handle.state::<AppState>(),
                    preferred_servers,
                    enable_namespace,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_search_tools" => {
            let query = arg_str(&args, "query")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_search_tools(
                    handle.state::<AppState>(),
                    query,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_set_tool_enabled" => {
            let tool_name = arg_str(&args, "tool_name")?;
            let enabled = arg_opt_bool(&args, "enabled")?.unwrap_or(false);
            to_json(
                yuan_mcp_commands::yuan_mcp_set_tool_enabled(
                    handle.state::<AppState>(),
                    tool_name,
                    enabled,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_set_enabled_tools" => {
            let tools: Vec<String> = serde_json::from_value(get(&args, "tools").cloned().unwrap_or(Json::Array(vec![])))
                .unwrap_or_default();
            to_json(
                yuan_mcp_commands::yuan_mcp_set_enabled_tools(
                    handle.state::<AppState>(),
                    tools,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_set_disabled_tools" => {
            let tools: Vec<String> = serde_json::from_value(get(&args, "tools").cloned().unwrap_or(Json::Array(vec![])))
                .unwrap_or_default();
            to_json(
                yuan_mcp_commands::yuan_mcp_set_disabled_tools(
                    handle.state::<AppState>(),
                    tools,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_list_filtered_tools" => {
            to_json(
                yuan_mcp_commands::yuan_mcp_list_filtered_tools(
                    handle.state::<AppState>(),
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_list_resources" => {
            let server_id = arg_str(&args, "server_id")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_list_resources(
                    handle.state::<AppState>(),
                    server_id,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_read_resource" => {
            let server_id = arg_str(&args, "server_id")?;
            let uri = arg_str(&args, "uri")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_read_resource(
                    handle.state::<AppState>(),
                    server_id,
                    uri,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_list_prompts" => {
            let server_id = arg_str(&args, "server_id")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_list_prompts(
                    handle.state::<AppState>(),
                    server_id,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_get_prompt" => {
            let server_id = arg_str(&args, "server_id")?;
            let name = arg_str(&args, "name")?;
            let arguments = match get(&args, "arguments") {
                Some(v) if !v.is_null() => Some(v.clone()),
                _ => None,
            };
            to_json(
                yuan_mcp_commands::yuan_mcp_get_prompt(
                    handle.state::<AppState>(),
                    server_id,
                    name,
                    arguments,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_set_deferred_namespaces" => {
            let namespaces: Vec<String> = serde_json::from_value(get(&args, "namespaces").cloned().unwrap_or(Json::Array(vec![])))
                .unwrap_or_default();
            to_json(
                yuan_mcp_commands::yuan_mcp_set_deferred_namespaces(
                    handle.state::<AppState>(),
                    namespaces,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_get_deferred_namespaces" => {
            to_json(
                yuan_mcp_commands::yuan_mcp_get_deferred_namespaces(
                    handle.state::<AppState>(),
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_load_namespace" => {
            let namespace = arg_str(&args, "namespace")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_load_namespace(
                    handle.state::<AppState>(),
                    namespace,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_unload_namespace" => {
            let namespace = arg_str(&args, "namespace")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_unload_namespace(
                    handle.state::<AppState>(),
                    namespace,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_list_persisted_servers" => {
            to_json(
                yuan_mcp_commands::yuan_mcp_list_persisted_servers(handle.state::<AppState>()).await?,
            )
        }
        "yuan_mcp_save_server_config" => {
            let server = parse_request(&args, "server")?;
            let auto_connect = arg_opt_bool(&args, "auto_connect")?.unwrap_or(false);
            to_json(
                yuan_mcp_commands::yuan_mcp_save_server_config(
                    server,
                    auto_connect,
                    handle.state::<AppState>(),
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_delete_server_config" => {
            let server_id = arg_str(&args, "server_id")?;
            to_json(
                yuan_mcp_commands::yuan_mcp_delete_server_config(
                    server_id,
                    handle.state::<AppState>(),
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_set_server_enabled" => {
            let server_id = arg_str(&args, "server_id")?;
            let enabled = arg_opt_bool(&args, "enabled")?.unwrap_or(false);
            to_json(
                yuan_mcp_commands::yuan_mcp_set_server_enabled(
                    server_id,
                    enabled,
                    handle.state::<AppState>(),
                )
                .await?,
            )
        }
        "yuan_mcp_load_persisted_servers" => {
            to_json(
                yuan_mcp_commands::yuan_mcp_load_persisted_servers(
                    handle.state::<AppState>(),
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_list_builtin_servers" => {
            to_json(
                yuan_mcp_commands::yuan_mcp_list_builtin_servers(
                    handle.state::<AppState>(),
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_list_builtin_tools" => {
            to_json(
                yuan_mcp_commands::yuan_mcp_list_builtin_tools(
                    handle.state::<AppState>(),
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_call_builtin_tool" => {
            let server_name = arg_str(&args, "server_name")?;
            let tool_name = arg_str(&args, "tool_name")?;
            let arguments = match get(&args, "arguments") {
                Some(v) if !v.is_null() => Some(v.clone()),
                _ => None,
            };
            to_json(
                yuan_mcp_commands::yuan_mcp_call_builtin_tool(
                    handle.state::<AppState>(),
                    server_name,
                    tool_name,
                    arguments,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_configure_builtin" => {
            let server_name = arg_str(&args, "server_name")?;
            let config = match get(&args, "config") {
                Some(v) => v.clone(),
                _ => Json::Null,
            };
            to_json(
                yuan_mcp_commands::yuan_mcp_configure_builtin(
                    handle.state::<AppState>(),
                    server_name,
                    config,
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        "yuan_mcp_list_call_history" => {
            to_json(
                yuan_mcp_commands::yuan_mcp_list_call_history(
                    handle.state::<AppState>(),
                    handle.state::<McpService>(),
                )
                .await?,
            )
        }
        // ===== yuan_skill_commands（26，5d）=====
        "yuan_skill_add_root" => {
            let path = arg_str(&args, "path")?;
            to_json(
                yuan_skill_commands::yuan_skill_add_root(
                    handle.state::<AppState>(),
                    path,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_set_project_files" => {
            let files: Vec<String> = serde_json::from_value(get(&args, "files").cloned().unwrap_or(Json::Array(vec![])))
                .unwrap_or_default();
            to_json(
                yuan_skill_commands::yuan_skill_set_project_files(
                    handle.state::<AppState>(),
                    files,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_load_all" => {
            to_json(
                yuan_skill_commands::yuan_skill_load_all(
                    handle.state::<AppState>(),
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_get" => {
            let name = arg_str(&args, "name")?;
            to_json(
                yuan_skill_commands::yuan_skill_get(
                    handle.state::<AppState>(),
                    name,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_list" => {
            let scope = arg_opt_str(&args, "scope")?;
            to_json(
                yuan_skill_commands::yuan_skill_list(
                    handle.state::<AppState>(),
                    scope,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_detect_implicit" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_skill_commands::yuan_skill_detect_implicit(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_register" => {
            let registration = parse_request(&args, "registration")?;
            to_json(
                yuan_skill_commands::yuan_skill_register(
                    handle.state::<AppState>(),
                    registration,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_unregister" => {
            let name = arg_str(&args, "name")?;
            to_json(
                yuan_skill_commands::yuan_skill_unregister(
                    handle.state::<AppState>(),
                    name,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_trigger" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_skill_commands::yuan_skill_trigger(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_auto_discover" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_skill_commands::yuan_skill_auto_discover(
                    handle.state::<AppState>(),
                    request,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_match" => {
            let query = arg_str(&args, "query")?;
            let limit = arg_opt_i64(&args, "limit")?.unwrap_or(10) as usize;
            to_json(
                yuan_skill_commands::yuan_skill_match(
                    handle.state::<AppState>(),
                    query,
                    limit,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_render_context" => {
            let config = match get(&args, "config") {
                Some(v) if !v.is_null() => Some(parse_request(&args, "config")?),
                _ => None,
            };
            to_json(
                yuan_skill_commands::yuan_skill_render_context(
                    handle.state::<AppState>(),
                    config,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_stats" => {
            to_json(
                yuan_skill_commands::yuan_skill_stats(
                    handle.state::<AppState>(),
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_export" => {
            let data = arg_str(&args, "data")?;
            to_json(yuan_skill_commands::yuan_skill_export(handle.state::<AppState>(), data).await?)
        }
        "yuan_skill_market_list" => {
            to_json(
                yuan_skill_commands::yuan_skill_market_list(
                    handle.state::<AppState>(),
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_market_search" => {
            let query = arg_str(&args, "query")?;
            to_json(
                yuan_skill_commands::yuan_skill_market_search(
                    handle.state::<AppState>(),
                    query,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_market_get" => {
            let name = arg_str(&args, "name")?;
            to_json(
                yuan_skill_commands::yuan_skill_market_get(
                    handle.state::<AppState>(),
                    name,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_market_install" => {
            let name = arg_str(&args, "name")?;
            to_json(
                yuan_skill_commands::yuan_skill_market_install(
                    handle.state::<AppState>(),
                    name,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_market_uninstall" => {
            let name = arg_str(&args, "name")?;
            to_json(
                yuan_skill_commands::yuan_skill_market_uninstall(
                    handle.state::<AppState>(),
                    name,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_install" => {
            let name = arg_str(&args, "name")?;
            to_json(
                yuan_skill_commands::yuan_skill_install(
                    handle.state::<AppState>(),
                    name,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_uninstall" => {
            let name = arg_str(&args, "name")?;
            to_json(
                yuan_skill_commands::yuan_skill_uninstall(
                    handle.state::<AppState>(),
                    name,
                    handle.state::<SkillService>(),
                )
                .await?,
            )
        }
        "yuan_skill_execute" => {
            let skill_name = arg_str(&args, "skill_name")?;
            let input: crate::skills::builtin::SkillInput = parse_request(&args, "input")?;
            let provider = arg_str(&args, "provider")?;
            let model_name = arg_str(&args, "model_name")?;
            to_json(
                yuan_skill_commands::yuan_skill_execute(
                    skill_name,
                    input,
                    provider,
                    model_name,
                    handle.state::<AppState>(),
                )
                .await?,
            )
        }
        "yuan_skill_builtin_list" => {
            to_json(yuan_skill_commands::yuan_skill_builtin_list(handle.state::<AppState>()).await?)
        }
        "yuan_skill_rate" => {
            let skill_name = arg_str(&args, "skill_name")?;
            let rating = arg_i64(&args, "rating")?;
            let review = arg_opt_str(&args, "review")?;
            to_json(
                yuan_skill_commands::yuan_skill_rate(
                    skill_name,
                    rating,
                    review,
                    handle.state::<AppState>(),
                )
                .await?,
            )
        }
        "yuan_skill_review" => {
            let skill_name = arg_str(&args, "skill_name")?;
            let review = arg_str(&args, "review")?;
            to_json(
                yuan_skill_commands::yuan_skill_review(skill_name, review, handle.state::<AppState>()).await?,
            )
        }
        "yuan_skill_ratings_get" => {
            let skill_name = arg_str(&args, "skill_name")?;
            to_json(
                yuan_skill_commands::yuan_skill_ratings_get(skill_name, handle.state::<AppState>()).await?,
            )
        }
        // ===== yuan_agent_autonomous_commands（3，5e）=====
        "yuan_agent_execute_autonomous" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_agent_autonomous_commands::yuan_agent_execute_autonomous(
                    request,
                    handle.clone(),
                    handle.state::<AppState>(),
                )
                .await?,
            )
        }
        "yuan_multi_file_edit" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_agent_autonomous_commands::yuan_multi_file_edit(
                    handle.state::<AppState>(),
                    request,
                )
                .await?,
            )
        }
        "yuan_multi_file_preview_diffs" => {
            let request = parse_request(&args, "request")?;
            to_json(
                yuan_agent_autonomous_commands::yuan_multi_file_preview_diffs(
                    handle.state::<AppState>(),
                    request,
                )
                .await?,
            )
        }
        // ===== agent_v3_commands（9，5e）=====
        "yuan_v3_agent_types" => {
            to_json(agent_v3_commands::yuan_v3_agent_types().await?)
        }
        "yuan_v3_agent_create" => {
            let request = parse_request(&args, "request")?;
            to_json(
                agent_v3_commands::yuan_v3_agent_create(
                    handle.state::<AppState>(),
                    handle.state::<AgentRegistry>(),
                    request,
                )
                .await?,
            )
        }
        "yuan_v3_agent_plan" => {
            let request = parse_request(&args, "request")?;
            to_json(
                agent_v3_commands::yuan_v3_agent_plan(
                    handle.state::<AppState>(),
                    handle.state::<AgentRegistry>(),
                    request,
                )
                .await?,
            )
        }
        "yuan_v3_agent_execute" => {
            let request = parse_request(&args, "request")?;
            to_json(
                agent_v3_commands::yuan_v3_agent_execute(
                    handle.state::<AppState>(),
                    handle.state::<AgentRegistry>(),
                    request,
                )
                .await?,
            )
        }
        "yuan_v3_agent_review" => {
            let request = parse_request(&args, "request")?;
            to_json(
                agent_v3_commands::yuan_v3_agent_review(
                    handle.state::<AppState>(),
                    handle.state::<AgentRegistry>(),
                    request,
                )
                .await?,
            )
        }
        "yuan_v3_agent_safety_check" => {
            let agent_id = arg_str(&args, "agent_id")?;
            to_json(
                agent_v3_commands::yuan_v3_agent_safety_check(
                    handle.state::<AppState>(),
                    handle.state::<AgentRegistry>(),
                    agent_id,
                )
                .await?,
            )
        }
        "yuan_v3_agent_status" => {
            let agent_id = arg_str(&args, "agent_id")?;
            to_json(
                agent_v3_commands::yuan_v3_agent_status(
                    handle.state::<AppState>(),
                    handle.state::<AgentRegistry>(),
                    agent_id,
                )
                .await?,
            )
        }
        "yuan_v3_agent_list" => {
            to_json(
                agent_v3_commands::yuan_v3_agent_list(
                    handle.state::<AppState>(),
                    handle.state::<AgentRegistry>(),
                )
                .await?,
            )
        }
        "yuan_v3_agent_destroy" => {
            let agent_id = arg_str(&args, "agent_id")?;
            to_json(
                agent_v3_commands::yuan_v3_agent_destroy(
                    handle.state::<AppState>(),
                    handle.state::<AgentRegistry>(),
                    agent_id,
                )
                .await?,
            )
        }
        // ===== cloud_api_commands（6，5e）=====
        "cloud_api_list" => {
            to_json(cloud_api_commands::cloud_api_list(handle.state::<AppState>()).await?)
        }
        "cloud_api_upsert" => {
            let request = parse_request(&args, "request")?;
            to_json(cloud_api_commands::cloud_api_upsert(handle.state::<AppState>(), request).await?)
        }
        "cloud_api_set_enabled" => {
            let id = arg_i64(&args, "id")?;
            let is_enabled = arg_bool(&args, "is_enabled")?;
            to_json(
                cloud_api_commands::cloud_api_set_enabled(handle.state::<AppState>(), id, is_enabled)
                    .await?,
            )
        }
        "cloud_api_delete" => {
            let id = arg_i64(&args, "id")?;
            to_json(cloud_api_commands::cloud_api_delete(handle.state::<AppState>(), id).await?)
        }
        "cloud_api_providers" => {
            to_json(cloud_api_commands::cloud_api_providers().await?)
        }
        "cloud_api_test_connection" => {
            let provider = arg_str(&args, "provider")?;
            let model_name = arg_str(&args, "model_name")?;
            to_json(
                cloud_api_commands::cloud_api_test_connection(
                    handle.state::<AppState>(),
                    provider,
                    model_name,
                )
                .await?,
            )
        }
        // ===== model_routing_commands（6，5e）=====
        "model_routing_list" => {
            to_json(model_routing_commands::model_routing_list(handle.state::<AppState>()).await?)
        }
        "model_routing_upsert" => {
            let request = parse_request(&args, "request")?;
            to_json(
                model_routing_commands::model_routing_upsert(handle.state::<AppState>(), request)
                    .await?,
            )
        }
        "model_routing_set_enabled" => {
            let id = arg_i64(&args, "id")?;
            let is_enabled = arg_bool(&args, "is_enabled")?;
            to_json(
                model_routing_commands::model_routing_set_enabled(
                    handle.state::<AppState>(),
                    id,
                    is_enabled,
                )
                .await?,
            )
        }
        "model_routing_delete" => {
            let id = arg_i64(&args, "id")?;
            to_json(
                model_routing_commands::model_routing_delete(handle.state::<AppState>(), id)
                    .await?,
            )
        }
        "model_routing_resolve" => {
            let task_type = arg_str(&args, "task_type")?;
            to_json(
                model_routing_commands::model_routing_resolve(handle.state::<AppState>(), task_type)
                    .await?,
            )
        }
        "model_routing_task_types" => {
            to_json(model_routing_commands::model_routing_task_types().await?)
        }
        // ===== yuan_code_monitor_commands（3，5e）=====
        "yuan_code_monitor_record_event" => {
            let event_type = arg_str(&args, "event_type")?;
            let detail = arg_opt_str(&args, "detail")?;
            to_json(
                yuan_code_monitor_commands::yuan_code_monitor_record_event(
                    handle.state::<AppState>(),
                    event_type,
                    detail,
                )
                .await?,
            )
        }
        "yuan_code_monitor_status" => {
            to_json(
                yuan_code_monitor_commands::yuan_code_monitor_status(handle.state::<AppState>())
                    .await?,
            )
        }
        "yuan_code_monitor_summary" => {
            to_json(
                yuan_code_monitor_commands::yuan_code_monitor_summary(handle.state::<AppState>())
                    .await?,
            )
        }
        // ===== collab_session_commands（7，5e）=====
        "collab_session_create" => {
            let name = arg_str(&args, "name")?;
            let workspace_root = arg_str(&args, "workspace_root")?;
            to_json(
                collab_session_commands::collab_session_create(
                    handle.state::<AppState>(),
                    name,
                    workspace_root,
                )
                .await?,
            )
        }
        "collab_session_list" => {
            to_json(collab_session_commands::collab_session_list(handle.state::<AppState>()).await?)
        }
        "collab_session_get" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(
                collab_session_commands::collab_session_get(
                    handle.state::<AppState>(),
                    session_id,
                )
                .await?,
            )
        }
        "collab_session_join" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(
                collab_session_commands::collab_session_join(
                    handle.state::<AppState>(),
                    session_id,
                )
                .await?,
            )
        }
        "collab_session_leave" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(
                collab_session_commands::collab_session_leave(
                    handle.state::<AppState>(),
                    session_id,
                )
                .await?,
            )
        }
        "collab_session_close" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(
                collab_session_commands::collab_session_close(
                    handle.state::<AppState>(),
                    session_id,
                )
                .await?,
            )
        }
        "collab_session_update_cursor" => {
            let session_id = arg_str(&args, "session_id")?;
            let cursor = match get(&args, "cursor") {
                Some(v) if !v.is_null() => Some(parse_request(&args, "cursor")?),
                _ => None,
            };
            to_json(
                collab_session_commands::collab_session_update_cursor(
                    handle.state::<AppState>(),
                    session_id,
                    cursor,
                )
                .await?,
            )
        }
        // ===== project_indexer_commands（5，5e）=====
        "project_index" => {
            let request = parse_request(&args, "request")?;
            to_json(
                project_indexer_commands::project_index(handle.state::<AppState>(), request)
                    .await?,
            )
        }
        "project_index_status" => {
            let root_path = arg_str(&args, "root_path")?;
            to_json(
                project_indexer_commands::project_index_status(
                    handle.state::<AppState>(),
                    root_path,
                )
                .await?,
            )
        }
        "project_search_symbols" => {
            let request = parse_request(&args, "request")?;
            to_json(
                project_indexer_commands::project_search_symbols(handle.state::<AppState>(), request)
                    .await?,
            )
        }
        "project_find_references" => {
            let request = parse_request(&args, "request")?;
            to_json(
                project_indexer_commands::project_find_references(
                    handle.state::<AppState>(),
                    request,
                )
                .await?,
            )
        }
        "project_get_related_files" => {
            let request = parse_request(&args, "request")?;
            to_json(
                project_indexer_commands::project_get_related_files(
                    handle.state::<AppState>(),
                    request,
                )
                .await?,
            )
        }
        _ => Err(format!("未知 terminal.yuancode 逻辑命令: {legacy}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        // yuancode_commands（27）
        "yuan_list_files",
        "yuan_read_file",
        "yuan_write_file",
        "yuan_create_item",
        "yuan_delete_item",
        "yuan_rename_item",
        "yuan_highlight",
        "yuan_execute",
        "yuan_complete",
        "yuan_complete_stream",
        "yuan_analyze",
        "yuan_save_snippet",
        "yuan_get_snippets",
        "yuan_update_snippet",
        "yuan_delete_snippet",
        "yuan_search_snippets",
        "yuan_compute_diff",
        "yuan_search_files",
        "yuan_replace_files",
        "yuan_copy_move",
        "yuan_get_file_info",
        "yuan_save_workspace",
        "yuan_load_workspace",
        "yuan_list_workspaces",
        "yuan_delete_workspace",
        "yuan_format_code",
        "yuan_settings_save",
        // yuan_inline_commands（3）
        "yuan_inline_complete",
        "yuan_inline_available",
        "yuan_inline_edit",
        // git_commands（13）
        "git_status",
        "git_diff_file",
        "git_diff_unstaged",
        "git_stage_file",
        "git_stage_all",
        "git_unstage_file",
        "git_commit",
        "git_push",
        "git_pull",
        "git_branches",
        "git_checkout",
        "git_log",
        "git_init",
        // lsp_commands（5）
        "lsp_completions",
        "lsp_hover",
        "lsp_definition",
        "lsp_diagnostics",
        "lsp_detect_language",
        // browser_commands（6）
        "browser_open_window",
        "browser_create_view",
        "browser_navigate_view",
        "browser_resize_view",
        "browser_close_view",
        "browser_cleanup",
        // editor_commands（26，5a）
        "editor_open",
        "editor_save",
        "editor_auto_save",
        "editor_close",
        "editor_get_versions",
        "editor_get_version",
        "editor_restore_version",
        "editor_recover_session",
        "editor_list_documents",
        "editor_delete_document",
        "editor_extract_metadata",
        "editor_generate_thumbnail",
        "editor_highlight",
        "editor_csv_preview",
        "editor_decrypted_preview",
        "editor_search",
        "editor_convert",
        "editor_convert_content",
        "editor_canvas_save",
        "editor_canvas_load",
        "editor_canvas_delete",
        "editor_diff_versions",
        "editor_cleanup_versions",
        "editor_index_content",
        "editor_recover_all_sessions",
        "editor_cleanup_sessions",
        // file_edit_commands（14，5a）
        "fileedit_read",
        "fileedit_write",
        "fileedit_get_status",
        "tableedit_read",
        "tableedit_write",
        "tableedit_export_csv",
        "pptedit_get_slides",
        "pptedit_update_slide",
        "pptedit_add_slide",
        "pptedit_delete_slide",
        "pptedit_reorder",
        "pdfedit_save",
        "imageedit_save",
        "audioedit_save",
        // tools_commands（8，5a）
        "yuan_tools_list",
        "yuan_tools_search",
        "yuan_tools_call",
        "yuan_plan_create",
        "yuan_plan_update_step",
        "yuan_plan_get",
        "yuan_plan_list_all",
        "yuan_apply_patch",
        // distill_commands（8，5a，T2 前置）
        "distill_extract_samples",
        "distill_generate_dataset",
        "distill_list_samples",
        "distill_approve_sample",
        "distill_reject_sample",
        "distill_local_infer",
        "distill_export_jsonl",
        "distill_sample_stats",
        // yuan_sandbox_commands（14，5b）
        "yuan_sandbox_create",
        "yuan_sandbox_list",
        "yuan_sandbox_list_by_agent",
        "yuan_sandbox_get",
        "yuan_sandbox_execute",
        "yuan_sandbox_read_file",
        "yuan_sandbox_write_file",
        "yuan_sandbox_delete_file",
        "yuan_sandbox_list_files",
        "yuan_sandbox_terminate",
        "yuan_sandbox_cleanup",
        "yuan_sandbox_count",
        "yuan_sandbox_save",
        "yuan_sandbox_history",
        // engine_commands（12，5b）
        "engine_create_session",
        "engine_get_session",
        "engine_list_sessions",
        "engine_destroy_session",
        "engine_start_turn",
        "engine_complete_turn",
        "engine_abort_turn",
        "engine_pause_session",
        "engine_resume_session",
        "engine_get_turns",
        "engine_stats",
        "engine_subscribe_events",
        // yuan_agent_commands（15，5b）
        "yuan_agent_spawn",
        "yuan_agent_fork",
        "yuan_agent_list",
        "yuan_agent_list_all",
        "yuan_agent_status",
        "yuan_agent_abort",
        "yuan_agent_transition",
        "yuan_agent_send_message",
        "yuan_agent_receive_messages",
        "yuan_agent_list_roles",
        "yuan_agent_get_count",
        "yuan_agent_cleanup",
        "yuan_agent_list_templates",
        "yuan_agent_deploy",
        "yuan_agent_execute",
        // yuan_safety_commands（5，5b）
        "yuan_safety_check",
        "yuan_safety_quick_check",
        "yuan_safety_set_profile",
        "yuan_safety_get_profile",
        "yuan_safety_list_profiles",
        // yuan_goal_commands（14，5c）
        "yuan_goal_create",
        "yuan_goal_get",
        "yuan_goal_list",
        "yuan_goal_update",
        "yuan_goal_start",
        "yuan_goal_pause",
        "yuan_goal_complete",
        "yuan_goal_abort",
        "yuan_goal_delete",
        "yuan_goal_consume_tokens",
        "yuan_goal_update_progress",
        "yuan_goal_save_checkpoint",
        "yuan_goal_build_continuation",
        "yuan_goal_checkpoints_count",
        // yuan_compact_commands（7，5c）
        "yuan_compact_config_get",
        "yuan_compact_config_update",
        "yuan_compact_estimate",
        "yuan_compact_check",
        "yuan_compact_execute",
        "yuan_compact_session",
        "yuan_compact_reset",
        // yuan_io_control_commands（8，5c）
        "yuan_io_config_get",
        "yuan_io_config_update",
        "yuan_io_stats",
        "yuan_io_execute",
        "yuan_io_cancel",
        "yuan_io_cancel_all",
        "yuan_io_kill",
        "yuan_io_stdin",
        // yuan_prompt_commands（12，5c）
        "yuan_prompt_list_templates",
        "yuan_prompt_get_template",
        "yuan_prompt_render",
        "yuan_prompt_set_custom_template",
        "yuan_prompt_remove_custom_template",
        "yuan_prompt_set_variable_default",
        "yuan_agents_discover",
        "yuan_agents_sources",
        "yuan_agents_assemble",
        "yuan_agents_set_max_bytes",
        "yuan_agents_get_max_bytes",
        "yuan_prompt_assemble",
        // yuan_mcp_commands（40，5d）
        "yuan_mcp_register_server",
        "yuan_mcp_connect_server",
        "yuan_mcp_disconnect_server",
        "yuan_mcp_list_servers",
        "yuan_mcp_list_all_tools",
        "yuan_mcp_server_status",
        "yuan_mcp_call_tool",
        "yuan_mcp_spawn",
        "yuan_mcp_health_check",
        "yuan_mcp_health_check_all",
        "yuan_mcp_set_lifecycle_config",
        "yuan_mcp_get_lifecycle_state",
        "yuan_mcp_list_shaped_tools",
        "yuan_mcp_group_tools_by_namespace",
        "yuan_mcp_list_tools_by_namespace",
        "yuan_mcp_get_tool_namespaces",
        "yuan_mcp_set_shaper_config",
        "yuan_mcp_search_tools",
        "yuan_mcp_set_tool_enabled",
        "yuan_mcp_set_enabled_tools",
        "yuan_mcp_set_disabled_tools",
        "yuan_mcp_list_filtered_tools",
        "yuan_mcp_list_resources",
        "yuan_mcp_read_resource",
        "yuan_mcp_list_prompts",
        "yuan_mcp_get_prompt",
        "yuan_mcp_set_deferred_namespaces",
        "yuan_mcp_get_deferred_namespaces",
        "yuan_mcp_load_namespace",
        "yuan_mcp_unload_namespace",
        "yuan_mcp_list_persisted_servers",
        "yuan_mcp_save_server_config",
        "yuan_mcp_delete_server_config",
        "yuan_mcp_set_server_enabled",
        "yuan_mcp_load_persisted_servers",
        "yuan_mcp_list_builtin_servers",
        "yuan_mcp_list_builtin_tools",
        "yuan_mcp_call_builtin_tool",
        "yuan_mcp_configure_builtin",
        "yuan_mcp_list_call_history",
        // yuan_skill_commands（26，5d）
        "yuan_skill_add_root",
        "yuan_skill_set_project_files",
        "yuan_skill_load_all",
        "yuan_skill_get",
        "yuan_skill_list",
        "yuan_skill_detect_implicit",
        "yuan_skill_register",
        "yuan_skill_unregister",
        "yuan_skill_trigger",
        "yuan_skill_auto_discover",
        "yuan_skill_match",
        "yuan_skill_render_context",
        "yuan_skill_stats",
        "yuan_skill_export",
        "yuan_skill_market_list",
        "yuan_skill_market_search",
        "yuan_skill_market_get",
        "yuan_skill_market_install",
        "yuan_skill_market_uninstall",
        "yuan_skill_install",
        "yuan_skill_uninstall",
        "yuan_skill_execute",
        "yuan_skill_builtin_list",
        "yuan_skill_rate",
        "yuan_skill_review",
        "yuan_skill_ratings_get",
        // ===== yuan_agent_autonomous_commands（3，5e）=====
        "yuan_agent_execute_autonomous",
        "yuan_multi_file_edit",
        "yuan_multi_file_preview_diffs",
        // ===== agent_v3_commands（9，5e）=====
        "yuan_v3_agent_types",
        "yuan_v3_agent_create",
        "yuan_v3_agent_plan",
        "yuan_v3_agent_execute",
        "yuan_v3_agent_review",
        "yuan_v3_agent_safety_check",
        "yuan_v3_agent_status",
        "yuan_v3_agent_list",
        "yuan_v3_agent_destroy",
        // ===== cloud_api_commands（6，5e）=====
        "cloud_api_list",
        "cloud_api_upsert",
        "cloud_api_set_enabled",
        "cloud_api_delete",
        "cloud_api_providers",
        "cloud_api_test_connection",
        // ===== model_routing_commands（6，5e）=====
        "model_routing_list",
        "model_routing_upsert",
        "model_routing_set_enabled",
        "model_routing_delete",
        "model_routing_resolve",
        "model_routing_task_types",
        // ===== yuan_code_monitor_commands（3，5e）=====
        "yuan_code_monitor_record_event",
        "yuan_code_monitor_status",
        "yuan_code_monitor_summary",
        // ===== collab_session_commands（7，5e）=====
        "collab_session_create",
        "collab_session_list",
        "collab_session_get",
        "collab_session_join",
        "collab_session_leave",
        "collab_session_close",
        "collab_session_update_cursor",
        // ===== project_indexer_commands（5，5e）=====
        "project_index",
        "project_index_status",
        "project_search_symbols",
        "project_find_references",
        "project_get_related_files",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 302);
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
}
