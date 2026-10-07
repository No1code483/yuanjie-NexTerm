//! home.journal 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发。
//!
//! 与 boards.profile / customs.auth / home.todo 同构：alias 只做旧名 → 逻辑名的
//! 映射登记；真实 handler 在 commands::dispatch_legacy —— 闭包仅捕获 AppHandle，
//! 运行时经 handle.state::<AppState>() 解析主应用真实 AppState（契约 06
//! Rust代码契约 §8.1）；业务实现复用 journal_commands 原函数（含
//! intelligence_v4_service 埋点，不得重复调用），返回值序列化与旧 IPC 路径一致
//! （V1 输入/输出快照等价）。

pub const PLUGIN_ID: &str = "home.journal";
pub const SHORT_CODE: &str = "jn";

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
            new_command: concat!("jn:plugin:", $command),
        }
    };
}

/// 3 条 alias：journals 表全部 IPC 命令（`main.rs` L515-517 旧 transport 于本批 S7 移除）。
pub const IPC_ALIASES: &[IpcAlias] = &[
    alias!("get_journal"),
    alias!("save_journal"),
    alias!("delete_journal"),
];

// ========== dispatcher 业务 handler 分发（S2 门禁：dispatcher 状态注入） ==========
//
// 每条 handler 的形态一致：闭包捕获 AppHandle → 运行时 handle.state::<AppState>()
// 取主应用真实状态 → 从 Json args 提取旧 IPC 参数（键名与前端 invoke 完全一致，
// 禁止按 transport 改写）→ 直接调用 journal_commands 原函数
// → serde 序列化返回值，与旧 Tauri command 路径输出一致。

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::journal_commands;
use crate::db::connection::AppState;

/// 从 args 提取必填字符串参数；缺失/类型不符按旧 Tauri 反序列化失败语义返回错误。
fn arg_str(args: &Json, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Json::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("参数 {key} 缺失或不是字符串"))
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
        "get_journal" => {
            let date = arg_str(&args, "date")?;
            to_json(journal_commands::get_journal(state, date).await?)
        }
        "save_journal" => {
            let date = arg_str(&args, "date")?;
            let content = arg_str(&args, "content")?;
            to_json(journal_commands::save_journal(state, date, content).await?)
        }
        "delete_journal" => {
            let date = arg_str(&args, "date")?;
            to_json(journal_commands::delete_journal(state, date).await?)
        }
        _ => Err(format!("未知 home.journal 逻辑命令: {legacy}")),
    }?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &["get_journal", "save_journal", "delete_journal"];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 3);
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
