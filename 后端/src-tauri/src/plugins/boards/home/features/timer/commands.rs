//! home.timer 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发。
//!
//! 与 boards.profile / customs.auth / home.todo / home.journal 同构：alias 只做旧名 →
//! 逻辑名的映射登记；真实 handler 在 commands::dispatch_legacy —— 闭包仅捕获
//! AppHandle，运行时经 handle.state::<AppState>() 解析主应用真实 AppState（契约 06
//! Rust代码契约 §8.1）；业务实现复用 timer_commands 原函数（含
//! intelligence_v4_service 埋点，不得重复调用），返回值序列化与旧 IPC 路径一致
//! （V1 输入/输出快照等价）。

pub const PLUGIN_ID: &str = "home.timer";
pub const SHORT_CODE: &str = "ti";

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
            new_command: concat!("ti:plugin:", $command),
        }
    };
}

/// 5 条 alias：timers 表全部 IPC 命令（`main.rs` L514-518 旧 transport 于本批 S7 移除）。
pub const IPC_ALIASES: &[IpcAlias] = &[
    alias!("get_timers"),
    alias!("create_timer"),
    alias!("update_timer_state"),
    alias!("delete_timer"),
    alias!("timer_action"),
];

// ========== dispatcher 业务 handler 分发（S2 门禁：dispatcher 状态注入） ==========
//
// 每条 handler 的形态一致：闭包捕获 AppHandle → 运行时 handle.state::<AppState>()
// 取主应用真实状态 → 从 Json args 提取旧 IPC 参数（键名与前端 invoke 完全一致，
// 禁止按 transport 改写）→ 直接调用 timer_commands 原函数
// → serde 序列化返回值，与旧 Tauri command 路径输出一致。

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::timer_commands;
use crate::db::connection::AppState;
use crate::models::timer::CreateTimerRequest;

/// 从 args 提取必填字符串参数；缺失/类型不符按旧 Tauri 反序列化失败语义返回错误。
fn arg_str(args: &Json, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Json::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("参数 {key} 缺失或不是字符串"))
}

fn arg_i64(args: &Json, key: &str) -> Result<i64, String> {
    args.get(key)
        .and_then(Json::as_i64)
        .ok_or_else(|| format!("参数 {key} 缺失或不是整数"))
}

fn arg_bool(args: &Json, key: &str) -> Result<bool, String> {
    args.get(key)
        .and_then(Json::as_bool)
        .ok_or_else(|| format!("参数 {key} 缺失或不是布尔值"))
}

fn parse_request<T: serde::de::DeserializeOwned>(args: &Json) -> Result<T, String> {
    args.get("request")
        .cloned()
        .ok_or_else(|| "参数 request 缺失".to_string())
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
    let response = match legacy {
        "get_timers" => to_json(timer_commands::get_timers(state).await?),
        "create_timer" => {
            let request: CreateTimerRequest = parse_request(&args)?;
            to_json(timer_commands::create_timer(state, request).await?)
        }
        // 旧命令签名为 `is_running: bool`，Tauri 参数绑定按 camelCase 取键（本批
        // 裁定 13：前端须发 `isRunning`，参数名不做 transport 侧改写）。
        "update_timer_state" => {
            let id = arg_i64(&args, "id")?;
            let is_running = arg_bool(&args, "isRunning")?;
            let elapsed = arg_i64(&args, "elapsed")?;
            to_json(timer_commands::update_timer_state(state, id, is_running, elapsed).await?)
        }
        "delete_timer" => {
            let id = arg_i64(&args, "id")?;
            to_json(timer_commands::delete_timer(state, id).await?)
        }
        "timer_action" => {
            let id = arg_i64(&args, "id")?;
            let action = arg_str(&args, "action")?;
            to_json(timer_commands::timer_action(state, id, action).await?)
        }
        _ => Err(format!("未知 home.timer 逻辑命令: {legacy}")),
    }?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "get_timers",
        "create_timer",
        "update_timer_state",
        "delete_timer",
        "timer_action",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 5);
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
