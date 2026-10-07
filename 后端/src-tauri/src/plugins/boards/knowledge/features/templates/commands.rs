//! knowledge.templates 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发（批C3）。
//!
//! 4 条模版命令自 boards.knowledge L1 迁入（旧命令名不变，transport 名改按短码 `kt`
//! 构造 `kt:plugin:<旧命令名>`）；业务实现复用 `kb_commands` 原函数，返回值序列化与
//! 旧 IPC 路径一致。参数键口径与 L1 同：snake_case 为先、camelCase 回退。

pub const PLUGIN_ID: &str = "knowledge.templates";
pub const SHORT_CODE: &str = "kt";

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
            new_command: concat!("kt:plugin:", $command),
        }
    };
}

/// 4 条 alias：模版命令全集（自 L1 2a-2 段迁入，旧命令名不变）。
pub const IPC_ALIASES: &[IpcAlias] = &[
    alias!("kb_get_templates"),
    alias!("kb_create_template"),
    alias!("kb_update_template"),
    alias!("kb_delete_template"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::kb_commands;
use crate::db::connection::AppState;
use crate::models::knowledge::{CreateKbTemplateRequest, UpdateKbTemplateRequest};

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
pub(crate) async fn dispatch_legacy<R: Runtime>(
    handle: AppHandle<R>,
    legacy: &'static str,
    args: Json,
) -> Result<Json, String> {
    let state = handle.state::<AppState>();
    match legacy {
        "kb_get_templates" => to_json(kb_commands::kb_get_templates(state).await?),
        "kb_create_template" => {
            let request: CreateKbTemplateRequest = parse_request(&args)?;
            to_json(kb_commands::kb_create_template(state, request).await?)
        }
        "kb_update_template" => {
            let request: UpdateKbTemplateRequest = parse_request(&args)?;
            to_json(kb_commands::kb_update_template(state, request).await?)
        }
        "kb_delete_template" => {
            let id = arg_i64(&args, "id")?;
            to_json(kb_commands::kb_delete_template(state, id).await?)
        }
        _ => Err(format!("未知 knowledge.templates 逻辑命令: {legacy}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "kb_get_templates",
        "kb_create_template",
        "kb_update_template",
        "kb_delete_template",
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
