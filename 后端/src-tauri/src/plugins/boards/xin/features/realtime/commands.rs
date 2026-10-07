//! xin.realtime 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发（阶段3 批次4a-1）。
//!
//! 与 boards.xin L1 同构：alias 只做旧名 → 逻辑名的映射登记；真实 handler 在
//! `dispatch_legacy`，运行时经 `handle.state::<AppState>()` 解析主应用真实 AppState。
//! 业务实现复用 `xin_realtime_commands` 原函数（含 `xin_realtime_start` 的 `app_handle`
//! 透传 —— Tauri Channel 事件推送依赖它）。
//!
//! **参数键口径**：Tauri v2 默认 `ArgumentCase::Camel`；本 dispatcher 以 snake_case 为先、
//! camelCase 回退，兼容两种口径。

pub const PLUGIN_ID: &str = "xin.realtime";
pub const SHORT_CODE: &str = "xr";

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
            new_command: concat!("xr:plugin:", $command),
        }
    };
}

/// 4 条 alias：`xin_realtime_commands` 全量（全部已消费）。
/// `main.rs` 旧 transport 于本批 S7 移除。
pub const IPC_ALIASES: &[IpcAlias] = &[
    alias!("xin_realtime_start"),
    alias!("xin_realtime_stop"),
    alias!("xin_realtime_push_chunk"),
    alias!("xin_realtime_get_state"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::xin_realtime_commands;
use crate::db::connection::AppState;
use crate::plugins::_legacy::services::xin_realtime_service::RealtimeConfig;

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
        "xin_realtime_start" => {
            let config: RealtimeConfig = parse_named(&args, "config")?;
            to_json(xin_realtime_commands::xin_realtime_start(state, handle.clone(), config).await?)
        }
        "xin_realtime_stop" => {
            to_json(xin_realtime_commands::xin_realtime_stop(state).await?)
        }
        "xin_realtime_push_chunk" => {
            let samples: Vec<i16> = parse_named(&args, "samples")?;
            let timestamp_ms = get(&args, "timestamp_ms")
                .and_then(Json::as_u64)
                .ok_or_else(|| "参数 timestamp_ms 缺失或不是 u64 整数".to_string())?;
            to_json(xin_realtime_commands::xin_realtime_push_chunk(state, samples, timestamp_ms).await?)
        }
        "xin_realtime_get_state" => {
            to_json(xin_realtime_commands::xin_realtime_get_state(state).await?)
        }
        other => Err(format!("xin.realtime dispatcher 未登记命令: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "xin_realtime_start",
        "xin_realtime_stop",
        "xin_realtime_push_chunk",
        "xin_realtime_get_state",
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