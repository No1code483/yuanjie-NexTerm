//! customs.search 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发。
//!
//! alias 只做旧名 → 逻辑名的映射登记；真实 handler 在
//! commands::dispatch_legacy —— 闭包仅捕获 AppHandle，运行时经
//! handle.state::<AppState>() 解析主应用真实 AppState（契约 06_Rust代码契约 §8.1），
//! 业务实现复用 commands::search_commands 原函数，返回值序列化与旧 IPC 路径一致。
//!
//! 批次6b S1：15 search 命令登记为 alias。

pub const PLUGIN_ID: &str = "customs.search";
pub const SHORT_CODE: &str = "se";

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
            new_command: concat!("se:plugin:", $command),
        }
    };
}

/// 15 条 alias：search 15 条命令。
pub const IPC_ALIASES: &[IpcAlias] = &[
    // ===== 搜索内核命令（5）=====
    alias!("search_global"),
    alias!("search_index_document"),
    alias!("search_delete_document"),
    alias!("search_clear_index"),
    alias!("search_rebuild_index"),
    // ===== 索引管理（1）=====
    alias!("search_index_status"),
    // ===== 搜索建议（1）=====
    alias!("search_suggest"),
    // ===== 高级搜索（1）=====
    alias!("search_advanced"),
    // ===== 分面统计（1）=====
    alias!("search_facets"),
    // ===== 搜索历史（2）=====
    alias!("search_history"),
    alias!("search_clear_history"),
    // ===== 热门查询（1）=====
    alias!("search_hot_queries"),
    // ===== 批量索引（1）=====
    alias!("search_batch_index"),
    // ===== 全站搜索（1）=====
    alias!("global_search"),
    // ===== AI 摘要（1）=====
    alias!("search_ai_summary"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::search_commands;
use crate::db::connection::AppState;
use crate::models::search::{
    AdvancedSearchQuery, BatchIndexRequest, GlobalSearchRequest, IndexableDocument,
    SearchSuggestRequest,
};

fn arg_str(args: &Json, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Json::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("参数 {key} 缺失或不是字符串"))
}

fn arg_opt_str(args: &Json, key: &str) -> Result<Option<String>, String> {
    match args.get(key) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => {
            let v = v.as_str().ok_or_else(|| format!("参数 {key} 不是字符串"))?;
            Ok(Some(v.to_string()))
        }
        None => Ok(None),
    }
}

fn arg_usize(args: &Json, key: &str) -> Result<usize, String> {
    args.get(key)
        .and_then(Json::as_u64)
        .map(|v| v as usize)
        .ok_or_else(|| format!("参数 {key} 缺失或不是整数"))
}

fn arg_opt_usize(args: &Json, key: &str) -> Result<Option<usize>, String> {
    match args.get(key) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => {
            let v = v.as_u64().map(|v| v as usize).ok_or_else(|| format!("参数 {key} 不是整数"))?;
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

fn arg_json<T: serde::de::DeserializeOwned>(args: &Json, key: &str) -> Result<T, String> {
    args.get(key)
        .ok_or_else(|| format!("参数 {key} 缺失"))
        .and_then(|v| serde_json::from_value(v.clone()).map_err(|e| format!("参数 {key} 反序列化失败: {}", e)))
}

fn arg_json_opt<T: serde::de::DeserializeOwned>(args: &Json, key: &str) -> Result<Option<T>, String> {
    match args.get(key) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => {
            let v = serde_json::from_value(v.clone()).map_err(|e| format!("参数 {key} 反序列化失败: {}", e))?;
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

fn arg_str_vec(args: &Json, key: &str) -> Result<Vec<String>, String> {
    args.get(key)
        .and_then(Json::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .ok_or_else(|| format!("参数 {key} 缺失或不是字符串数组"))
}

fn arg_str_vec_opt(args: &Json, key: &str) -> Result<Option<Vec<String>>, String> {
    match args.get(key) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => {
            let arr = v.as_array().ok_or_else(|| format!("参数 {key} 不是字符串数组"))?;
            let result: Vec<String> = arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            Ok(Some(result))
        }
        None => Ok(None),
    }
}

fn to_json<T: serde::Serialize>(value: T) -> Result<Json, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

/// 逐条分发到旧命令函数。legacy 为 IPC_ALIASES.legacy_command 值。
/// search_commands 使用 State<'_, AppState> + require_auth 模式。
/// 直接调用原函数复用 require_auth。
pub(crate) async fn dispatch_legacy<R: Runtime>(
    handle: tauri::AppHandle<R>,
    legacy: &'static str,
    args: Json,
) -> Result<Json, String> {
    let state = handle.state::<AppState>();

    match legacy {
        "search_global" => {
            let query = arg_str(&args, "query")?;
            let source_filter = arg_json_opt(&args, "sourceFilter")?;
            let limit = arg_opt_usize(&args, "limit")?;
            let offset = arg_opt_usize(&args, "offset")?;
            to_json(search_commands::search_global(state, query, source_filter, limit, offset).await?)
        }
        "search_index_document" => {
            let document: IndexableDocument = arg_json(&args, "document")?;
            to_json(search_commands::search_index_document(state, document).await?)
        }
        "search_delete_document" => {
            let doc_id = arg_str(&args, "docId")?;
            to_json(search_commands::search_delete_document(state, doc_id).await?)
        }
        "search_clear_index" => {
            to_json(search_commands::search_clear_index(state).await?)
        }
        "search_rebuild_index" => {
            to_json(search_commands::search_rebuild_index(state).await?)
        }
        "search_index_status" => {
            to_json(search_commands::search_index_status(state).await?)
        }
        "search_suggest" => {
            let request: SearchSuggestRequest = arg_json(&args, "request")?;
            to_json(search_commands::search_suggest(state, request).await?)
        }
        "search_advanced" => {
            let query: AdvancedSearchQuery = arg_json(&args, "query")?;
            to_json(search_commands::search_advanced(state, query).await?)
        }
        "search_facets" => {
            let query = arg_str(&args, "query")?;
            to_json(search_commands::search_facets(state, query).await?)
        }
        "search_history" => {
            to_json(search_commands::search_history(state).await?)
        }
        "search_clear_history" => {
            to_json(search_commands::search_clear_history(state).await?)
        }
        "search_hot_queries" => {
            let limit = arg_opt_usize(&args, "limit")?;
            to_json(search_commands::search_hot_queries(state, limit).await?)
        }
        "search_batch_index" => {
            let request: BatchIndexRequest = arg_json(&args, "request")?;
            to_json(search_commands::search_batch_index(state, request).await?)
        }
        "global_search" => {
            let request: GlobalSearchRequest = arg_json(&args, "request")?;
            to_json(search_commands::global_search(state, request).await?)
        }
        "search_ai_summary" => {
            let request: crate::models::search::SearchAiSummaryRequest = arg_json(&args, "request")?;
            to_json(search_commands::search_ai_summary(state, request).await?)
        }
        _ => Err(format!("未知 search 逻辑命令: {legacy}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "search_global",
        "search_index_document",
        "search_delete_document",
        "search_clear_index",
        "search_rebuild_index",
        "search_index_status",
        "search_suggest",
        "search_advanced",
        "search_facets",
        "search_history",
        "search_clear_history",
        "search_hot_queries",
        "search_batch_index",
        "global_search",
        "search_ai_summary",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 15);
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
