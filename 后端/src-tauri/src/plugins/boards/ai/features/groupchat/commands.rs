//! ai.groupchat 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发（批次2b-1）。
//!
//! 与 ai.models / ai.agent 同构：alias 只做旧名 → 逻辑名的映射登记；真实 handler
//! 在 `dispatch_legacy` —— 闭包仅捕获 AppHandle，运行时经 `handle.state::<AppState>()`
//! 解析主应用真实 AppState（契约 06_Rust代码契约 §8.1）；业务实现复用 `ai_commands`
//! 原函数（内部调 `chat_service`，属批次 2b-2 的暂跨引用，登记收口）。
//!
//! **参数键口径**：`conversation_id` 前端为 camelCase（`conversationId`，档案 §5.3 预检）；
//! 本 dispatcher 以 snake_case 为先、camelCase 回退，不按 transport 静默改写输入。

pub const PLUGIN_ID: &str = "ai.groupchat";
pub const SHORT_CODE: &str = "gc";

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
            new_command: concat!("gc:plugin:", $command),
        }
    };
}

/// 4 条 alias（2b-1：编排状态/强制结束 2 条；2b-2 裁定 11：会话编排 2 条归入本域）。
/// `main.rs` 旧 transport 于对应批次 S7 移除。
pub const IPC_ALIASES: &[IpcAlias] = &[
    alias!("ai_get_orchestration_status"),
    alias!("ai_end_group_chat"),
    // 2b-2（裁定 11）：会话编排两条与既有编排同域同实现（chat_service::run_orchestrator /
    // stop_generation），归 gc；`run_orchestrator` 已按裁定 13 补会话归属校验。
    alias!("run_orchestrator"),
    alias!("stop_generation"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::{ai_commands, chat_commands};
use crate::db::connection::AppState;
use crate::models::chat::OrchestratorConfig;

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

/// 取参数：snake_case 优先，camelCase 回退（前端两条调用通道的键名口径均已实证）。
fn get<'a>(args: &'a Json, key: &str) -> Option<&'a Json> {
    args.get(key).or_else(|| {
        let camel = to_camel(key);
        args.get(camel.as_str())
    })
}

fn arg_i64(args: &Json, key: &str) -> Result<i64, String> {
    get(args, key)
        .and_then(Json::as_i64)
        .ok_or_else(|| format!("参数 {key} 缺失或不是整数"))
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
        "ai_get_orchestration_status" => {
            let conversation_id = arg_i64(&args, "conversation_id")?;
            to_json(ai_commands::ai_get_orchestration_status(state, conversation_id).await?)
        }
        "ai_end_group_chat" => {
            let conversation_id = arg_i64(&args, "conversation_id")?;
            to_json(ai_commands::ai_end_group_chat(state, conversation_id).await?)
        }
        // 2b-2（裁定 11/13）：会话编排命令复用 chat_commands 原函数；run_orchestrator 需
        // AppHandle 发射 ai-orchestrator/ai-stream，dispatcher 的泛型 R 在登记处统一为 Wry。
        "run_orchestrator" => {
            let conversation_id = arg_i64(&args, "conversation_id")?;
            let user_message = args
                .get("user_message")
                .or_else(|| args.get("userMessage"))
                .and_then(Json::as_str)
                .ok_or_else(|| "参数 user_message 缺失或不是字符串".to_string())?
                .to_string();
            let config: Option<OrchestratorConfig> = match args.get("config") {
                None | Some(Json::Null) => None,
                Some(v) => Some(serde_json::from_value(v.clone()).map_err(|e| e.to_string())?),
            };
            to_json(
                chat_commands::run_orchestrator(
                    handle.clone(),
                    state,
                    conversation_id,
                    user_message,
                    config,
                )
                .await?,
            )
        }
        "stop_generation" => {
            let conversation_id = arg_i64(&args, "conversation_id")?;
            to_json(chat_commands::stop_generation(state, conversation_id).await?)
        }
        _ => Err(format!("未知 ai.groupchat 逻辑命令: {legacy}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "ai_get_orchestration_status",
        "ai_end_group_chat",
        "run_orchestrator",
        "stop_generation",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 4);
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