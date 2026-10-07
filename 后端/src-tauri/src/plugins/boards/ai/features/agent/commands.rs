//! ai.agent 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发（批次2b-1）。
//!
//! 与 ai.models / boards.knowledge 同构：alias 只做旧名 → 逻辑名的映射登记；真实
//! handler 在 `dispatch_legacy` —— 闭包仅捕获 AppHandle，运行时经
//! `handle.state::<AppState>()` 解析主应用真实 AppState（契约 06_Rust代码契约 §8.1）；
//! 业务实现复用 `ai_commands` 原函数，返回值序列化与旧 IPC 路径一致（V1 快照等价）。

pub const PLUGIN_ID: &str = "ai.agent";
pub const SHORT_CODE: &str = "ag";

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
            new_command: concat!("ag:plugin:", $command),
        }
    };
}

/// 4 条 alias：ai_agents 表全部 IPC 命令（`main.rs` L375-378 旧 transport 于本批 S7 移除）。
pub const IPC_ALIASES: &[IpcAlias] = &[
    alias!("get_ai_agents"),
    alias!("add_ai_agent"),
    alias!("update_ai_agent"),
    alias!("delete_ai_agent"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::ai_commands;
use crate::db::connection::AppState;
use crate::models::ai_model::{AddAgentRequest, UpdateAgentRequest};

fn arg_i64(args: &Json, key: &str) -> Result<i64, String> {
    args.get(key)
        .and_then(Json::as_i64)
        .ok_or_else(|| format!("参数 {key} 缺失或不是整数"))
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
    match legacy {
        "get_ai_agents" => to_json(ai_commands::get_ai_agents(state).await?),
        "add_ai_agent" => {
            let request: AddAgentRequest = parse_request(&args)?;
            to_json(ai_commands::add_ai_agent(state, request).await?)
        }
        "update_ai_agent" => {
            let request: UpdateAgentRequest = parse_request(&args)?;
            to_json(ai_commands::update_ai_agent(state, request).await?)
        }
        "delete_ai_agent" => {
            let id = arg_i64(&args, "id")?;
            to_json(ai_commands::delete_ai_agent(state, id).await?)
        }
        _ => Err(format!("未知 ai.agent 逻辑命令: {legacy}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] =
        &["get_ai_agents", "add_ai_agent", "update_ai_agent", "delete_ai_agent"];

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