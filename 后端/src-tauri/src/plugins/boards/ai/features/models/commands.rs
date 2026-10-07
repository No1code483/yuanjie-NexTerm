//! ai.models 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发（批次2b-1）。
//!
//! 与 boards.home / boards.knowledge 同构：alias 只做旧名 → 逻辑名的映射登记；真实
//! handler 在 `dispatch_legacy` —— 闭包仅捕获 AppHandle，运行时经
//! `handle.state::<AppState>()` 解析主应用真实 AppState（契约 06_Rust代码契约 §8.1，
//! pool / 认证 / 服务全部来源于此，禁止伪造或第二池）；业务实现复用 `ai_commands`
//! 原函数，返回值序列化与旧 IPC 路径一致（V1 输入/输出快照等价）。
//!
//! **参数键口径**：Tauri v2 默认 `ArgumentCase::Camel`，故前端顶层多词参数键统一为
//! camelCase（`modelId`，档案 §5.3 预检）。本 dispatcher 以 snake_case 为先、
//! camelCase 回退，兼容两种口径，不按 transport 静默改写输入
//! （《测试流程》IPC 双轨回放纪律第 1 条）。

pub const PLUGIN_ID: &str = "ai.models";
pub const SHORT_CODE: &str = "am";

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
            new_command: concat!("am:plugin:", $command),
        }
    };
}

/// 9 条 alias（2b-1：ai_models 表 8 条；2b-2 裁定 11：单模型健康检测 `check_model_health`
/// 写 `ai_models` 表 → 归 am）。`main.rs` 旧 transport 于对应批次 S7 移除。
pub const IPC_ALIASES: &[IpcAlias] = &[
    alias!("get_ai_models"),
    alias!("add_ai_model"),
    alias!("update_ai_model"),
    alias!("delete_ai_model"),
    alias!("get_ai_provider_info"),
    alias!("check_all_models_health"),
    alias!("get_model_health_status"),
    alias!("set_health_check_interval"),
    // 2b-2（裁定 11）：单模型健康检测，实现在 chat_commands（写 ai_models 表）。
    alias!("check_model_health"),
];

// ========== dispatcher 业务 handler 分发（S2 门禁：dispatcher 状态注入） ==========
//
// 每条 handler 的形态一致：闭包捕获 AppHandle → 运行时 handle.state::<AppState>()
// 取主应用真实状态 → 从 Json args 提取旧 IPC 参数（键名与前端 invoke 完全一致，
// 禁止按 transport 改写）→ 直接调用 ai_commands 原函数 → serde 序列化返回值，
// 与旧 Tauri command 路径输出一致。

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::{ai_commands, chat_commands};
use crate::db::connection::AppState;
use crate::models::ai_model::{AddModelRequest, UpdateModelRequest};

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

fn arg_u64(args: &Json, key: &str) -> Result<u64, String> {
    get(args, key)
        .and_then(Json::as_u64)
        .ok_or_else(|| format!("参数 {key} 缺失或不是非负整数"))
}

fn arg_str(args: &Json, key: &str) -> Result<String, String> {
    get(args, key)
        .and_then(Json::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("参数 {key} 缺失或不是字符串"))
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
        "get_ai_models" => to_json(ai_commands::get_ai_models(state).await?),
        "add_ai_model" => {
            let request: AddModelRequest = parse_request(&args)?;
            to_json(ai_commands::add_ai_model(state, request).await?)
        }
        "update_ai_model" => {
            let request: UpdateModelRequest = parse_request(&args)?;
            to_json(ai_commands::update_ai_model(state, request).await?)
        }
        "delete_ai_model" => {
            let id = arg_i64(&args, "id")?;
            to_json(ai_commands::delete_ai_model(state, id).await?)
        }
        "get_ai_provider_info" => {
            let provider = arg_str(&args, "provider")?;
            to_json(ai_commands::get_ai_provider_info(state, provider).await?)
        }
        // 全量检测命令需 AppHandle 发射 `ai-model-health-changed`（S5 事件零改动）；
        // dispatcher 的泛型 R 在登记处（TauriPlugin<Wry>）统一为 Wry。
        "check_all_models_health" => {
            to_json(ai_commands::check_all_models_health(handle.clone(), state).await?)
        }
        "get_model_health_status" => {
            let model_id = arg_i64(&args, "model_id")?;
            to_json(ai_commands::get_model_health_status(state, model_id).await?)
        }
        "set_health_check_interval" => {
            let minutes = arg_u64(&args, "minutes")?;
            to_json(ai_commands::set_health_check_interval(state, minutes).await?)
        }
        // 2b-2（裁定 11）：单模型健康检测实现在 chat_commands（写 ai_models 表）。
        "check_model_health" => {
            let model_id = arg_i64(&args, "model_id")?;
            to_json(chat_commands::check_model_health(state, model_id).await?)
        }
        _ => Err(format!("未知 ai.models 逻辑命令: {legacy}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "get_ai_models",
        "add_ai_model",
        "update_ai_model",
        "delete_ai_model",
        "get_ai_provider_info",
        "check_all_models_health",
        "get_model_health_status",
        "set_health_check_interval",
        "check_model_health",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 9);
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