//! customs.recycle 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发。
//!
//! 与 customs.auth 同构：alias 只做旧名 → 逻辑名的映射登记；真实 handler 在
//! commands::dispatch_legacy —— 闭包仅捕获 AppHandle，运行时经
//! handle.state::<AppState>() 解析主应用真实 AppState（契约 06_Rust代码契约 §8.1），
//! 业务实现复用 commands::recycle_commands 原函数，返回值序列化与旧 IPC 路径一致。
//!
//! 批次6a S1：7 recycle 命令登记为 alias。

pub const PLUGIN_ID: &str = "customs.recycle";
pub const SHORT_CODE: &str = "rc";

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
            new_command: concat!("rc:plugin:", $command),
        }
    };
}

/// 7 条 alias：recycle 7 条命令。
pub const IPC_ALIASES: &[IpcAlias] = &[
    alias!("recycle_list"),
    alias!("recycle_move_to"),
    alias!("recycle_restore"),
    alias!("recycle_delete_permanently"),
    alias!("recycle_empty_all"),
    alias!("cleanup_expired_recycle"),
    alias!("recycle_stats"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::recycle_commands;
use crate::db::connection::AppState;

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

fn arg_opt_i64(args: &Json, key: &str) -> Result<Option<i64>, String> {
    match args.get(key) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => {
            let v = v.as_i64().ok_or_else(|| format!("参数 {key} 不是整数"))?;
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

fn arg_vec_i64(args: &Json, key: &str) -> Result<Vec<i64>, String> {
    args.get(key)
        .and_then(Json::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_i64())
                .collect::<Vec<_>>()
        })
        .ok_or_else(|| format!("参数 {key} 缺失或不是数组"))
}

fn arg_bool(args: &Json, key: &str) -> Result<bool, String> {
    args.get(key)
        .and_then(Json::as_bool)
        .ok_or_else(|| format!("参数 {key} 缺失或不是布尔值"))
}

fn to_json<T: serde::Serialize>(value: T) -> Result<Json, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

/// 逐条分发到旧命令函数。legacy 为 IPC_ALIASES.legacy_command 值。
/// recycle_commands 使用 #[tauri::command] + State<'_, AppState> + require_auth 模式。
/// 直接调用原函数（参数与前端 invoke 一致），复用 require_auth。
pub(crate) async fn dispatch_legacy<R: Runtime>(
    handle: AppHandle<R>,
    legacy: &'static str,
    args: Json,
) -> Result<Json, String> {
    let state = handle.state::<AppState>();

    match legacy {
        "recycle_list" => {
            to_json(recycle_commands::recycle_list(state).await?)
        }
        "recycle_move_to" => {
            let item_type = arg_str(&args, "itemType")?;
            let item_ids = arg_vec_i64(&args, "itemIds")?;
            to_json(recycle_commands::recycle_move_to(state, item_type, item_ids).await?)
        }
        "recycle_restore" => {
            let ids = arg_vec_i64(&args, "ids")?;
            to_json(recycle_commands::recycle_restore(state, ids).await?)
        }
        "recycle_delete_permanently" => {
            let ids = arg_vec_i64(&args, "ids")?;
            to_json(
                recycle_commands::recycle_delete_permanently(state, ids).await?,
            )
        }
        "recycle_empty_all" => {
            to_json(recycle_commands::recycle_empty_all(state).await?)
        }
        "cleanup_expired_recycle" => {
            to_json(recycle_commands::cleanup_expired_recycle(state).await?)
        }
        "recycle_stats" => {
            to_json(recycle_commands::recycle_stats(state).await?)
        }
        _ => Err(format!("未知 recycle 逻辑命令: {legacy}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "recycle_list",
        "recycle_move_to",
        "recycle_restore",
        "recycle_delete_permanently",
        "recycle_empty_all",
        "cleanup_expired_recycle",
        "recycle_stats",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 7);
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
