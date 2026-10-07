//! boards.terminal 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发（阶段3 批次3a）。
//!
//! 与 boards.ai / boards.knowledge 同构：alias 只做旧名 → 逻辑名的映射登记；真实 handler
//! 在 `dispatch_legacy` —— 闭包仅捕获 AppHandle，运行时经 `handle.state::<AppState>()`
//! 解析主应用真实 AppState（契约 06_Rust代码契约 §8.1，pool / 认证 / 服务全部来源于此，
//! 禁止伪造或第二池）；业务实现复用 `terminal_commands` 原函数（含 intelligence_v4 埋点，
//! 不得重复调用），返回值序列化与旧 IPC 路径一致（V1 输入/输出快照等价）。
//!
//! **参数键口径**：Tauri v2 默认 `ArgumentCase::Camel`；本 dispatcher 以 snake_case 为先、
//! camelCase 回退，兼容两种口径，不按 transport 静默改写输入（《测试流程》IPC 双轨回放纪律第 1 条）。
//!
//! **PTY 确定性口径（裁定 T8）**：create/write/resize/kill/wsl/ssh_connect 依赖
//! ConPTY/portable-pty 真实进程，S6 走「失败路径双轨同判」+ CRUD 类全量回放。

pub const PLUGIN_ID: &str = "boards.terminal";
pub const SHORT_CODE: &str = "tm";

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
            new_command: concat!("tm:plugin:", $command),
        }
    };
}

/// 29 条 alias：terminal_commands 全量（裁定 T7：零消费 19 条全部「留 + alias」，
/// 含 SSH 整族 6 / mux 2 / history 4 / config 2 等——后端功能完整，前端 UI 属未完成规划）。
/// `main.rs` 旧 transport 于本批 S7 移除。
pub const IPC_ALIASES: &[IpcAlias] = &[
    // PTY 会话（6）
    alias!("terminal_create_session"),
    alias!("terminal_write_input"),
    alias!("terminal_resize"),
    alias!("terminal_kill_session"),
    alias!("terminal_list_sessions"),
    alias!("terminal_execute_builtin"),
    // 系统信息 / 目录（2）
    alias!("terminal_get_system_info"),
    alias!("terminal_list_directory"),
    // 历史系（4）
    alias!("terminal_get_history"),
    alias!("terminal_clear_history"),
    alias!("terminal_get_history_count"),
    alias!("terminal_search_history_fts"),
    // 布局系（3）
    alias!("terminal_save_layout"),
    alias!("terminal_load_layout"),
    alias!("terminal_clear_layout"),
    // WSL（2）
    alias!("terminal_detect_wsl"),
    alias!("terminal_create_wsl_session"),
    // Mux（2）
    alias!("terminal_mux_get_active_sessions"),
    alias!("terminal_mux_clear_all_sessions"),
    // SSH（6，零消费整族，裁定 T7 判留）
    alias!("terminal_ssh_list_profiles"),
    alias!("terminal_ssh_save_profile"),
    alias!("terminal_ssh_delete_profile"),
    alias!("terminal_ssh_connect"),
    alias!("terminal_ssh_disconnect"),
    alias!("terminal_ssh_write"),
    alias!("terminal_ssh_resize"),
    // 配置 & 主题（3）
    alias!("terminal_get_themes"),
    alias!("terminal_get_config"),
    alias!("terminal_save_config"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::terminal_commands;
use crate::db::connection::AppState;
use crate::models::terminal::{SshProfileInput, TerminalConfigInput, TerminalTabLayoutInput};

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

fn arg_u16(args: &Json, key: &str) -> Result<u16, String> {
    get(args, key)
        .and_then(Json::as_u64)
        .and_then(|v| u16::try_from(v).ok())
        .ok_or_else(|| format!("参数 {key} 缺失或不是 u16 整数"))
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

fn parse_named<T: serde::de::DeserializeOwned>(args: &Json, key: &str) -> Result<T, String> {
    get(args, key)
        .cloned()
        .ok_or_else(|| format!("参数 {key} 缺失"))
        .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
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
    let state = handle.state::<AppState>();
    match legacy {
        // ===== PTY 会话（create/resize/kill/wsl 依赖 ConPTY，S6 失败路径口径）=====
        "terminal_create_session" => {
            let session_type = arg_str(&args, "session_type")?;
            let tab_id = arg_opt_str(&args, "tab_id")?;
            let pane_id = arg_opt_str(&args, "pane_id")?;
            let cols = match get(&args, "cols") {
                None | Some(Json::Null) => None,
                Some(v) => Some(v.as_u64().and_then(|x| u16::try_from(x).ok()).ok_or_else(|| "参数 cols 不是 u16".to_string())?),
            };
            let rows = match get(&args, "rows") {
                None | Some(Json::Null) => None,
                Some(v) => Some(v.as_u64().and_then(|x| u16::try_from(x).ok()).ok_or_else(|| "参数 rows 不是 u16".to_string())?),
            };
            to_json(
                terminal_commands::terminal_create_session(state, session_type, tab_id, pane_id, cols, rows, handle.clone())
                    .await?,
            )
        }
        "terminal_write_input" => {
            let session_id = arg_str(&args, "session_id")?;
            let input = arg_str(&args, "input")?;
            to_json(terminal_commands::terminal_write_input(state, session_id, input).await?)
        }
        "terminal_resize" => {
            let session_id = arg_str(&args, "session_id")?;
            let cols = arg_u16(&args, "cols")?;
            let rows = arg_u16(&args, "rows")?;
            to_json(terminal_commands::terminal_resize(state, session_id, cols, rows, handle.clone()).await?)
        }
        "terminal_kill_session" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(terminal_commands::terminal_kill_session(state, session_id, handle.clone()).await?)
        }
        "terminal_list_sessions" => to_json(terminal_commands::terminal_list_sessions(state).await?),
        "terminal_execute_builtin" => {
            let command = arg_str(&args, "command")?;
            to_json(terminal_commands::terminal_execute_builtin(state, command).await?)
        }
        // ===== 系统信息 / 目录 =====
        "terminal_get_system_info" => to_json(terminal_commands::terminal_get_system_info(state).await?),
        "terminal_list_directory" => {
            let path = arg_opt_str(&args, "path")?;
            to_json(terminal_commands::terminal_list_directory(state, path).await?)
        }
        // ===== 历史系 =====
        "terminal_get_history" => {
            let limit = arg_opt_i64(&args, "limit")?;
            let session_type = arg_opt_str(&args, "session_type")?;
            to_json(terminal_commands::terminal_get_history(state, limit, session_type).await?)
        }
        "terminal_clear_history" => {
            let session_type = arg_opt_str(&args, "session_type")?;
            to_json(terminal_commands::terminal_clear_history(state, session_type).await?)
        }
        "terminal_get_history_count" => to_json(terminal_commands::terminal_get_history_count(state).await?),
        "terminal_search_history_fts" => {
            let keyword = arg_str(&args, "keyword")?;
            let limit = arg_opt_i64(&args, "limit")?;
            to_json(terminal_commands::terminal_search_history_fts(state, keyword, limit).await?)
        }
        // ===== 布局系 =====
        "terminal_save_layout" => {
            let tabs: Vec<TerminalTabLayoutInput> = parse_named(&args, "tabs")?;
            to_json(terminal_commands::terminal_save_layout(state, tabs).await?)
        }
        "terminal_load_layout" => to_json(terminal_commands::terminal_load_layout(state).await?),
        "terminal_clear_layout" => to_json(terminal_commands::terminal_clear_layout(state).await?),
        // ===== WSL =====
        "terminal_detect_wsl" => to_json(terminal_commands::terminal_detect_wsl(state).await?),
        "terminal_create_wsl_session" => {
            let distro = arg_opt_str(&args, "distro")?;
            let shell = arg_opt_str(&args, "shell")?;
            to_json(terminal_commands::terminal_create_wsl_session(state, handle.clone(), distro, shell).await?)
        }
        // ===== Mux =====
        "terminal_mux_get_active_sessions" => {
            to_json(terminal_commands::terminal_mux_get_active_sessions(state).await?)
        }
        "terminal_mux_clear_all_sessions" => {
            to_json(terminal_commands::terminal_mux_clear_all_sessions(state).await?)
        }
        // ===== SSH =====
        "terminal_ssh_list_profiles" => to_json(terminal_commands::terminal_ssh_list_profiles(state).await?),
        "terminal_ssh_save_profile" => {
            let input: SshProfileInput = parse_named(&args, "input")?;
            to_json(terminal_commands::terminal_ssh_save_profile(state, input).await?)
        }
        "terminal_ssh_delete_profile" => {
            let id = arg_str(&args, "id")?;
            to_json(terminal_commands::terminal_ssh_delete_profile(state, id).await?)
        }
        "terminal_ssh_connect" => {
            let profile_id = arg_str(&args, "profile_id")?;
            to_json(terminal_commands::terminal_ssh_connect(state, profile_id, handle.clone()).await?)
        }
        "terminal_ssh_disconnect" => {
            let session_id = arg_str(&args, "session_id")?;
            to_json(terminal_commands::terminal_ssh_disconnect(state, session_id).await?)
        }
        "terminal_ssh_write" => {
            let session_id = arg_str(&args, "session_id")?;
            let data = arg_str(&args, "data")?;
            to_json(terminal_commands::terminal_ssh_write(state, session_id, data).await?)
        }
        "terminal_ssh_resize" => {
            let session_id = arg_str(&args, "session_id")?;
            let cols = arg_u16(&args, "cols")?;
            let rows = arg_u16(&args, "rows")?;
            to_json(terminal_commands::terminal_ssh_resize(state, session_id, cols, rows).await?)
        }
        // ===== 配置 & 主题 =====
        "terminal_get_themes" => to_json(terminal_commands::terminal_get_themes(state).await?),
        "terminal_get_config" => to_json(terminal_commands::terminal_get_config(state).await?),
        "terminal_save_config" => {
            let input: TerminalConfigInput = parse_named(&args, "input")?;
            to_json(terminal_commands::terminal_save_config(state, input).await?)
        }
        _ => Err(format!("未知 boards.terminal 逻辑命令: {legacy}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "terminal_create_session",
        "terminal_write_input",
        "terminal_resize",
        "terminal_kill_session",
        "terminal_list_sessions",
        "terminal_execute_builtin",
        "terminal_get_system_info",
        "terminal_list_directory",
        "terminal_get_history",
        "terminal_clear_history",
        "terminal_get_history_count",
        "terminal_search_history_fts",
        "terminal_save_layout",
        "terminal_load_layout",
        "terminal_clear_layout",
        "terminal_detect_wsl",
        "terminal_create_wsl_session",
        "terminal_mux_get_active_sessions",
        "terminal_mux_clear_all_sessions",
        "terminal_ssh_list_profiles",
        "terminal_ssh_save_profile",
        "terminal_ssh_delete_profile",
        "terminal_ssh_connect",
        "terminal_ssh_disconnect",
        "terminal_ssh_write",
        "terminal_ssh_resize",
        "terminal_get_themes",
        "terminal_get_config",
        "terminal_save_config",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 29);
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
