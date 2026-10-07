//! boards.ai（L1）的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发（批次2b-2）。
//!
//! 裁定 11（命令归属 Y）+ 裁定 12（L1 混合模式）：L1 首次「既持插槽又持自有 IPC + 表」，
//! 承载会话面 18 条命令（会话 10 + 消息 3 + 参与者 1 + 模板 4）。与 ai.models / ai.agent /
//! boards.knowledge 同构：alias 只做旧名 → 逻辑名映射；真实 handler 在 `dispatch_legacy`
//! —— 闭包仅捕获 AppHandle，运行时经 `handle.state::<AppState>()` 解析主应用真实 AppState
//! （契约 06_Rust代码契约 §8.1，pool / 认证 / 服务全部来源于此，禁止伪造或第二池）；
//! 业务实现复用 `chat_commands` 原函数（含 intelligence_v4 埋点，不得重复调用），
//! 返回值序列化与旧 IPC 路径一致（V1 输入/输出快照等价）。
//!
//! **参数键口径**：Tauri v2 默认 `ArgumentCase::Camel`，前端顶层多词参数键统一为
//! camelCase（`conversationId` 等）。本 dispatcher 以 snake_case 为先、camelCase 回退，
//! 兼容两种口径，不按 transport 静默改写输入（《测试流程》IPC 双轨回放纪律第 1 条）。
//!
//! **实现层去留（裁定 15）**：沿用 2b-1 模式——`chat_service` / `chat_repo` /
//! `ai_orchestrator` / models 仍留主应用，实现层迁移登记阶段4；本段仅做命令面收编
//! （alias + dispatcher）+ 4 张会话表归属登记。

pub const PLUGIN_ID: &str = "boards.ai";
pub const SHORT_CODE: &str = "ai";

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
            new_command: concat!("ai:plugin:", $command),
        }
    };
}

/// batchC1 收编后 L1 剩余 5 条 alias：sendMessage 1 + Prompt 模板 4。
/// 会话面 13 条命令（create_conversation / get_conversations / update_conversation /
/// delete_conversation / mark_conversation_read / reorder_conversations /
/// search_conversations / toggle_star_conversation / branch_conversation /
/// export_conversation / get_messages / delete_message / get_participants）
/// 归 L2 ai.sessions（ss）承载，见 features/sessions/commands.rs。
/// 编排 2 条（`run_orchestrator` / `stop_generation`）归 `ai.groupchat`（gc）；
/// `check_model_health` 归 `ai.models`（am）——见裁定 11。
pub const IPC_ALIASES: &[IpcAlias] = &[
    // 消息面（1）— sendMessage 归 L1 承载（编排入口）
    alias!("send_message"),
    // Prompt 模板（4）
    alias!("prompt_template_list"),
    alias!("prompt_template_create"),
    alias!("prompt_template_update"),
    alias!("prompt_template_delete"),
];

// ========== dispatcher 业务 handler 分发（S2：dispatcher 状态注入） ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::chat_commands;
use crate::db::connection::AppState;
use crate::models::chat::{
    CreatePromptTemplateRequest, SendMessageRequest, UpdatePromptTemplateRequest,
};

/// 取参数：snake_case 优先，camelCase 回退（前端两条调用通道的键名口径均已实证）。
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
///
/// 复用 `chat_commands::*` 原 Tauri command 函数：它们签名为
/// `State<'_, AppState>`（部分含 `AppHandle<R>`），本 dispatcher 通过
/// `handle.state::<AppState>()` 取主应用真实 State 直接传入，输出与旧路径逐字一致。
pub(crate) async fn dispatch_legacy<R: Runtime>(
    handle: AppHandle<R>,
    legacy: &'static str,
    args: Json,
) -> Result<Json, String> {
    let state = handle.state::<AppState>();
    match legacy {
        // ===== 消息面 =====
        "send_message" => {
            let request: SendMessageRequest = parse_request(&args)?;
            to_json(chat_commands::send_message(handle.clone(), state, request).await?)
        }
        // ===== Prompt 模板 =====
        "prompt_template_list" => to_json(chat_commands::prompt_template_list(state).await?),
        "prompt_template_create" => {
            let request: CreatePromptTemplateRequest = parse_request(&args)?;
            to_json(chat_commands::prompt_template_create(state, request).await?)
        }
        "prompt_template_update" => {
            let request: UpdatePromptTemplateRequest = parse_request(&args)?;
            to_json(chat_commands::prompt_template_update(state, request).await?)
        }
        "prompt_template_delete" => {
            let id = arg_i64(&args, "id")?;
            to_json(chat_commands::prompt_template_delete(state, id).await?)
        }
        _ => Err(format!("未知 boards.ai 逻辑命令: {legacy}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "send_message",
        "prompt_template_list",
        "prompt_template_create",
        "prompt_template_update",
        "prompt_template_delete",
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
