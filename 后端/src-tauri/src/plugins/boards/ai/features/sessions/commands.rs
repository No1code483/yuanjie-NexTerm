//! ai.sessions 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发（batchC1）。
//!
//! 与 boards.ai L1 同构：alias 只做旧名 → 逻辑名映射；真实 handler 在 `dispatch_legacy`
//! —— 闭包仅捕获 AppHandle，运行时经 `handle.state::<AppState>()` 解析主应用真实 AppState
//! （契约 06_Rust代码契约 §8.1），认证/pool/服务均来自真实状态；
//! 业务实现复用 `chat_commands` 原函数，返回值序列化与旧 IPC 路径一致（V1 快照等价）。
//!
//! **参数键口径**：Tauri v2 默认 `ArgumentCase::Camel`，前端顶层多词参数键统一为
//! camelCase（`conversationId` 等）。本 dispatcher 以 snake_case 为先、camelCase 回退，
//! 兼容两种口径，不按 transport 静默改写输入（《测试流程》IPC 双轨回放纪律第 1 条）。

pub const PLUGIN_ID: &str = "ai.sessions";
pub const SHORT_CODE: &str = "ss";

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
            new_command: concat!("ss:plugin:", $command),
        }
    };
}

/// batchC1：会话面 10 条命令（会话 6 + 消息 2 + 参与者 1 + 搜索/导出/标记已读/排序/星标/分支 6）。
/// sendMessage 归 L1 boards.ai 承载；run_orchestrator / stop_generation 归 ai.groupchat（gc）。
/// check_model_health 归 ai.models（am）——见裁定 11。
pub const IPC_ALIASES: &[IpcAlias] = &[
    // 会话面（6）
    alias!("create_conversation"),
    alias!("get_conversations"),
    alias!("update_conversation"),
    alias!("delete_conversation"),
    alias!("mark_conversation_read"),
    alias!("reorder_conversations"),
    alias!("search_conversations"),
    alias!("toggle_star_conversation"),
    alias!("branch_conversation"),
    alias!("export_conversation"),
    // 消息面（2）— sendMessage 归 L1 boards.ai 承载
    alias!("get_messages"),
    alias!("delete_message"),
    // 参与者（1）
    alias!("get_participants"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::chat_commands;
use crate::db::connection::AppState;
use crate::models::ai_model::ReorderItem as SortItem;
use crate::models::chat::{
    BranchConversationRequest, CreateConversationRequest, ExportConversationRequest,
    SearchConversationsRequest, UpdateConversationRequest,
};

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

fn arg_opt_str(args: &Json, key: &str) -> Result<Option<String>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_str()
            .map(|s| Some(s.to_string()))
            .ok_or_else(|| format!("参数 {key} 不是字符串或 null")),
    }
}

fn parse_request<T: serde::de::DeserializeOwned>(args: &Json) -> Result<T, String> {
    args.get("request")
        .cloned()
        .ok_or_else(|| "参数 request 缺失".to_string())
        .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
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
        // ===== 会话面 =====
        "create_conversation" => {
            let request: CreateConversationRequest = parse_request(&args)?;
            to_json(chat_commands::create_conversation(state, request).await?)
        }
        "get_conversations" => {
            let r#type = arg_opt_str(&args, "type")?;
            to_json(chat_commands::get_conversations(state, r#type).await?)
        }
        "update_conversation" => {
            let request: UpdateConversationRequest = parse_request(&args)?;
            to_json(chat_commands::update_conversation(state, request).await?)
        }
        "delete_conversation" => {
            let id = arg_i64(&args, "id")?;
            to_json(chat_commands::delete_conversation(state, id).await?)
        }
        "mark_conversation_read" => {
            let conversation_id = arg_i64(&args, "conversation_id")?;
            to_json(chat_commands::mark_conversation_read(state, conversation_id).await?)
        }
        "reorder_conversations" => {
            let items: Vec<SortItem> = parse_named(&args, "items")?;
            to_json(chat_commands::reorder_conversations(state, items).await?)
        }
        "search_conversations" => {
            let request: SearchConversationsRequest = parse_request(&args)?;
            to_json(chat_commands::search_conversations(state, request).await?)
        }
        "toggle_star_conversation" => {
            let id = arg_i64(&args, "id")?;
            to_json(chat_commands::toggle_star_conversation(state, id).await?)
        }
        "branch_conversation" => {
            let request: BranchConversationRequest = parse_request(&args)?;
            to_json(chat_commands::branch_conversation(state, request).await?)
        }
        "export_conversation" => {
            let request: ExportConversationRequest = parse_request(&args)?;
            to_json(chat_commands::export_conversation(state, request).await?)
        }
        // ===== 消息面（2） =====
        "get_messages" => {
            let conversation_id = arg_i64(&args, "conversation_id")?;
            to_json(chat_commands::get_messages(state, conversation_id).await?)
        }
        "delete_message" => {
            let id = arg_i64(&args, "id")?;
            to_json(chat_commands::delete_message(state, id).await?)
        }
        // ===== 参与者 =====
        "get_participants" => {
            let conversation_id = arg_i64(&args, "conversation_id")?;
            to_json(chat_commands::get_participants(state, conversation_id).await?)
        }
        _ => Err(format!("未知 ai.sessions 逻辑命令: {legacy}")),
    }
}
