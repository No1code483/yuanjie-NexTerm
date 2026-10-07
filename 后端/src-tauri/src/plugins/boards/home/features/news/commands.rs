//! home.news 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发。
//!
//! 与 boards.profile / customs.auth / home.todo / home.journal / home.timer 同构：
//! alias 只做旧名 → 逻辑名的映射登记；真实 handler 在 commands::dispatch_legacy ——
//! 闭包仅捕获 AppHandle，运行时经 handle.state::<AppState>() 解析主应用真实 AppState
//! （契约 06 Rust代码契约 §8.1）；业务实现复用 news_commands /
//! news_source_commands 原函数（含 intelligence_v4_service 埋点，不得重复调用），
//! 返回值序列化与旧 IPC 路径一致（V1 输入/输出快照等价）。

pub const PLUGIN_ID: &str = "home.news";
pub const SHORT_CODE: &str = "nw";

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
            new_command: concat!("nw:plugin:", $command),
        }
    };
}

/// 14 条 alias：news 域全部 IPC 命令（`main.rs` L540-554 旧 transport 于本批移除）。
pub const IPC_ALIASES: &[IpcAlias] = &[
    alias!("get_news"),
    alias!("get_news_by_category"),
    alias!("fetch_news"),
    alias!("add_news"),
    alias!("mark_news_read"),
    alias!("clear_old_news"),
    alias!("toggle_news_favorite"),
    alias!("get_pending_delete_news"),
    alias!("generate_news_ai_summary"),
    alias!("news_get_cached"),
    alias!("news_cache_status"),
    alias!("get_news_sources"),
    alias!("add_news_source"),
    alias!("delete_news_source"),
];

// ========== dispatcher 业务 handler 分发（S2 门禁：dispatcher 状态注入） ==========
//
// 每条 handler 的形态一致：闭包捕获 AppHandle → 运行时 handle.state::<AppState>()
// 取主应用真实状态 → 从 Json args 提取旧 IPC 参数（键名与前端 invoke 完全一致，
// 禁止按 transport 改写）→ 直接调用旧命令函数 → serde 序列化返回值，与旧 Tauri
// command 路径输出一致。

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::{news_commands, news_source_commands};
use crate::db::connection::AppState;

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

fn arg_bool(args: &Json, key: &str) -> Result<bool, String> {
    args.get(key)
        .and_then(Json::as_bool)
        .ok_or_else(|| format!("参数 {key} 缺失或不是布尔值"))
}

/// 可选参数：缺键或值为 null 均按 None（与旧 Tauri `Option<T>` 绑定同义）。
fn arg_opt_str(args: &Json, key: &str) -> Option<String> {
    args.get(key).and_then(Json::as_str).map(|s| s.to_string())
}

fn arg_opt_i64(args: &Json, key: &str) -> Option<i64> {
    args.get(key).and_then(Json::as_i64)
}

fn arg_opt_bool(args: &Json, key: &str) -> Option<bool> {
    args.get(key).and_then(Json::as_bool)
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
        "get_news" => to_json(news_commands::get_news(state).await?),
        "get_news_by_category" => {
            let category = arg_str(&args, "category")?;
            to_json(news_commands::get_news_by_category(state, category).await?)
        }
        "fetch_news" => to_json(news_commands::fetch_news(state).await?),
        "add_news" => {
            let title = arg_str(&args, "title")?;
            let url = arg_opt_str(&args, "url");
            let source = arg_opt_str(&args, "source");
            let summary = arg_opt_str(&args, "summary");
            to_json(news_commands::add_news(state, title, url, source, summary).await?)
        }
        "mark_news_read" => {
            let id = arg_i64(&args, "id")?;
            to_json(news_commands::mark_news_read(state, id).await?)
        }
        "clear_old_news" => {
            let days = arg_i64(&args, "days")?;
            to_json(news_commands::clear_old_news(state, days).await?)
        }
        // 旧命令签名为 `is_favorite: bool`，Tauri 参数绑定按 camelCase 取键
        // （前端 `home.ts:39-42` 发 `isFavorite`，参数名不做 transport 侧改写）。
        "toggle_news_favorite" => {
            let id = arg_i64(&args, "id")?;
            let is_favorite = arg_bool(&args, "isFavorite")?;
            to_json(news_commands::toggle_news_favorite(state, id, is_favorite).await?)
        }
        "get_pending_delete_news" => to_json(news_commands::get_pending_delete_news(state).await?),
        "generate_news_ai_summary" => {
            let id = arg_i64(&args, "id")?;
            let provider = arg_opt_str(&args, "provider");
            let endpoint = arg_opt_str(&args, "endpoint");
            let model = arg_opt_str(&args, "model");
            // 旧形参 `is_github: Option<bool>` → camelCase 取键 `isGithub`。
            let is_github = arg_opt_bool(&args, "isGithub");
            to_json(
                news_commands::generate_news_ai_summary(
                    state, id, provider, endpoint, model, is_github,
                )
                .await?,
            )
        }
        "news_get_cached" => {
            // 前端固定发 `{source, limit}`（可为 null/数字），缺省语义与旧命令一致。
            let source = arg_opt_str(&args, "source");
            let limit = arg_opt_i64(&args, "limit");
            to_json(news_commands::news_get_cached(state, source, limit).await?)
        }
        "news_cache_status" => to_json(news_commands::news_cache_status(state).await?),
        "get_news_sources" => to_json(news_source_commands::get_news_sources(state).await?),
        // 旧形参 `feed_type: String` → camelCase 取键 `feedType`（本批裁定 17-A：
        // 修复前端发送键名，参数名不做 transport 侧改写）。
        "add_news_source" => {
            let name = arg_str(&args, "name")?;
            let url = arg_str(&args, "url")?;
            let category = arg_str(&args, "category")?;
            let feed_type = arg_str(&args, "feedType")?;
            to_json(
                news_source_commands::add_news_source(state, name, url, category, feed_type).await?,
            )
        }
        "delete_news_source" => {
            let id = arg_i64(&args, "id")?;
            to_json(news_source_commands::delete_news_source(state, id).await?)
        }
        _ => Err(format!("未知 home.news 逻辑命令: {legacy}")),
    }?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "get_news",
        "get_news_by_category",
        "fetch_news",
        "add_news",
        "mark_news_read",
        "clear_old_news",
        "toggle_news_favorite",
        "get_pending_delete_news",
        "generate_news_ai_summary",
        "news_get_cached",
        "news_cache_status",
        "get_news_sources",
        "add_news_source",
        "delete_news_source",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 14);
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
