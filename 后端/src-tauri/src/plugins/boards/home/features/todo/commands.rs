//! home.todo 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发。
//!
//! 与 boards.profile / customs.auth 同构：alias 只做旧名 → 逻辑名的映射登记；
//! 真实 handler 在 commands::dispatch_legacy —— 闭包仅捕获 AppHandle，运行时经
//! handle.state::<AppState>() 解析主应用真实 AppState（契约 06_Rust代码契约 §8.1，
//! pool / 认证 / 服务全部来源于此，禁止伪造或第二池）；业务实现复用
//! todo_commands 原函数（含 intelligence_v4_service 埋点，不得重复调用），
//! 返回值序列化与旧 IPC 路径一致（V1 输入/输出快照等价）。

pub const PLUGIN_ID: &str = "home.todo";
pub const SHORT_CODE: &str = "td";

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
            new_command: concat!("td:plugin:", $command),
        }
    };
}

/// 6 条 alias：todos 表全部 IPC 命令（`main.rs` L509-514 旧 transport 于本批 S7 移除）。
pub const IPC_ALIASES: &[IpcAlias] = &[
    alias!("get_todos"),
    alias!("get_todos_paginated"),
    alias!("add_todo"),
    alias!("toggle_todo"),
    alias!("update_todo"),
    alias!("delete_todo"),
];

// ========== dispatcher 业务 handler 分发（S2 门禁：dispatcher 状态注入） ==========
//
// 每条 handler 的形态一致：闭包捕获 AppHandle → 运行时 handle.state::<AppState>()
// 取主应用真实状态 → 从 Json args 提取旧 IPC 参数（键名与前端 invoke 完全一致，
// 禁止按 transport 改写）→ 直接调用 todo_commands 原函数
// → serde 序列化返回值，与旧 Tauri command 路径输出一致。

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::todo_commands;
use crate::db::connection::AppState;
use crate::models::todo::{CreateTodoRequest, UpdateTodoRequest};

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

/// 旧 Tauri 命令 `get_todos_paginated` 的 `Option<u32>` 参数（前端零消费，
/// 输入形状取自旧命令签名；Tauri 参数绑定为 camelCase）。
fn arg_opt_u32(args: &Json, key: &str) -> Result<Option<u32>, String> {
    match args.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .map(Some)
            .ok_or_else(|| format!("参数 {key} 不是无符号整数或 null")),
    }
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
        "get_todos" => {
            let date = arg_str(&args, "date")?;
            to_json(todo_commands::get_todos(state, date).await?)
        }
        "get_todos_paginated" => {
            let page = arg_opt_u32(&args, "page")?;
            let page_size = arg_opt_u32(&args, "pageSize")?;
            to_json(todo_commands::get_todos_paginated(state, page, page_size).await?)
        }
        "add_todo" => {
            let request: CreateTodoRequest = parse_request(&args)?;
            to_json(todo_commands::add_todo(state, request).await?)
        }
        "toggle_todo" => {
            let id = arg_i64(&args, "id")?;
            to_json(todo_commands::toggle_todo(state, id).await?)
        }
        "update_todo" => {
            let request: UpdateTodoRequest = parse_request(&args)?;
            to_json(todo_commands::update_todo(state, request).await?)
        }
        "delete_todo" => {
            let id = arg_i64(&args, "id")?;
            to_json(todo_commands::delete_todo(state, id).await?)
        }
        _ => Err(format!("未知 home.todo 逻辑命令: {legacy}")),
    }?;

    publish_domain_events(&handle, domain_event_specs(legacy, &args, &response));
    Ok(response)
}

// ========== 领域事件（批次1b-2c · 裁定 24-A / 25-A） ==========
//
// 域 = 插件 id（内核 `security.rs` 与 `event_bus.rs` 三处强制「发布域 == 插件 id」），
// 故事件名为 `home.todo:todo.{created,updated,deleted}`，而非设计初稿的 `home:todo.*`。
// 载荷只带摘要（id / 类型 / 字段名 / 时间戳），**禁止含 title 等用户内容**：
// `kernel_domain_events` 无 user_id 列且前端桥接为全量转发，含正文将构成跨用户泄露。
// 形态与 `plugins/boards/profile/commands.rs` 的既有范式一致。

#[derive(Debug, PartialEq)]
struct DomainEventSpec {
    name: &'static str,
    payload: Json,
}

/// 由旧逻辑命令名 + 入参 + 响应推导领域事件；仅成功响应（`code == 0`）发布。
/// 删除路径无需存在性预检：`todo_service::delete_todo` 对不存在/非本人记录返回
/// `NotFound`，dispatcher 经 `?` 提前返回，事件不会下发。
fn domain_event_specs(legacy: &str, args: &Json, response: &Json) -> Vec<DomainEventSpec> {
    if response.get("code").and_then(Json::as_i64) != Some(0) {
        return Vec::new();
    }

    let data = response.get("data").unwrap_or(&Json::Null);
    let arg_id = || args.get("id").cloned().unwrap_or(Json::Null);
    let request_id = || args.pointer("/request/id").cloned().unwrap_or(Json::Null);

    match legacy {
        "add_todo" => vec![DomainEventSpec {
            name: "home.todo:todo.created",
            payload: serde_json::json!({
                "itemId": data.get("id").cloned().unwrap_or(Json::Null),
                "itemType": "todo",
            }),
        }],
        "toggle_todo" => vec![DomainEventSpec {
            name: "home.todo:todo.updated",
            payload: serde_json::json!({
                "itemId": arg_id(),
                "itemType": "todo",
                "changedFields": ["completed"],
            }),
        }],
        "update_todo" => vec![DomainEventSpec {
            name: "home.todo:todo.updated",
            payload: serde_json::json!({
                "itemId": request_id(),
                "itemType": "todo",
                "changedFields": ["title", "description", "priority", "due_date"],
            }),
        }],
        // 删除事件统一契约（回收站依赖，见 04_事件总线 §六）。
        "delete_todo" => vec![DomainEventSpec {
            name: "home.todo:todo.deleted",
            payload: serde_json::json!({
                "itemId": arg_id(),
                "itemType": "todo",
                "deletedAt": chrono::Utc::now().timestamp_millis(),
                "softDelete": true,
                "originPlugin": PLUGIN_ID,
            }),
        }],
        _ => Vec::new(),
    }
}

fn publish_domain_events<R: Runtime>(handle: &AppHandle<R>, specs: Vec<DomainEventSpec>) {
    if specs.is_empty() {
        return;
    }
    let kernel = handle.state::<kernel::tauri_glue::KernelState>();
    let origin = PLUGIN_ID.to_string();
    for spec in specs {
        if let Err(error) = kernel.security.check_publish(
            super::manifest::manifest(),
            spec.name,
            kernel_api::EventScope::Domain,
        ) {
            eprintln!("[home.todo] 领域事件权限校验失败: {error}");
            continue;
        }
        let event = kernel_api::Event {
            name: spec.name.to_string(),
            origin: origin.clone(),
            domain: origin.clone(),
            scope: kernel_api::EventScope::Domain,
            payload: spec.payload,
            at: chrono::Utc::now().timestamp_millis(),
        };
        if let Err(error) = kernel.bus.publish(&origin, event) {
            eprintln!("[home.todo] 领域事件发布失败: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "get_todos",
        "get_todos_paginated",
        "add_todo",
        "toggle_todo",
        "update_todo",
        "delete_todo",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 6);
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
