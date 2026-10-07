//! boards.knowledge 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发（批次2a-1/2a-2/2a-3）。
//!
//! 与 boards.home / customs.auth 同构：alias 只做旧名 → 逻辑名的映射登记；真实 handler
//! 在 `dispatch_legacy` —— 闭包仅捕获 AppHandle，运行时经 `handle.state::<AppState>()`
//! 解析主应用真实 AppState（契约 06_Rust代码契约 §8.1，pool / 认证 / 服务全部来源于此，
//! 禁止伪造或第二池）；业务实现复用 `kb_commands` 原函数（含 intelligence_v4_service 埋点，
//! 不得重复调用），返回值序列化与旧 IPC 路径一致（V1 输入/输出快照等价）。
//!
//! **参数键口径**：Tauri v2 默认 `ArgumentCase::Camel`，故前端顶层多词参数键统一为
//! camelCase（`entryId` / `tagIds` / `entryIds` / `tagId` / `parentId` / `sortOrder`，
//! 裁定 37-A）。本 dispatcher 以 snake_case 为先、camelCase 回退，兼容两种口径，
//! 不按 transport 静默改写输入（《测试流程》IPC 双轨回放纪律第 1 条）。

pub const PLUGIN_ID: &str = "boards.knowledge";
pub const SHORT_CODE: &str = "kb";

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
            new_command: concat!("kb:plugin:", $command),
        }
    };
}

/// 知识库 alias 全集：2a-1 段 29 条（核心数据面）+ 2a-2 段 22 条（导入与解析，
/// 4 条模版命令已随批C3 迁出至 knowledge.templates L2）+ 2a-3 段 6 条（附件与语义）
/// = 57 条。
///
/// 知识库共 66 条命令，按裁定 33-A 拆三段：2a-1（29）+ 2a-2（29 − 3 判删 = 26）
/// + 2a-3（附件语义 6）。**5 条判删不入 alias**（2 条 35-A + 3 条 42-A，已随各段 S7 清理）：
/// `delete_kb_category` / `get_kb_entries`（35-A）、`kb_import_folder`（被 multi_folders
/// 覆盖）/ `kb_extract_pptx_text`（零消费）/ `kb_add_tracked_path`（零消费）——42-A；
/// 零消费判「留」有 alias 无前端方法 2 条：`kb_get_outgoing_links`（功能缺口，42-A）、
/// `kb_attachment_preload`（A5 命令面完整保留，43-A）。
/// 批C3：4 条模版命令（kb_get/create/update/delete_template）迁出至 knowledge.templates
/// L2（短码 kt），本文件 61 → 57 条。
pub const IPC_ALIASES: &[IpcAlias] = &[
    alias!("get_kb_categories"),
    alias!("get_kb_category_counts"),
    alias!("add_kb_category"),
    alias!("update_kb_category"),
    alias!("add_kb_entry"),
    alias!("delete_kb_entry"),
    alias!("move_kb_entry"),
    alias!("move_kb_category"),
    alias!("update_kb_entry"),
    alias!("search_kb_entries"),
    alias!("get_all_kb_entries"),
    alias!("get_kb_tags"),
    alias!("add_kb_tag"),
    alias!("update_kb_tag"),
    alias!("delete_kb_tag"),
    alias!("get_kb_entry_tags"),
    alias!("get_kb_all_entry_tags"),
    alias!("set_kb_entry_tags"),
    alias!("get_kb_tag_stats"),
    alias!("get_kb_entries_by_tag"),
    alias!("batch_add_kb_tag"),
    alias!("batch_remove_kb_tag"),
    alias!("toggle_kb_favorite"),
    alias!("get_kb_favorites"),
    alias!("move_kb_category_to_recycle"),
    alias!("record_kb_access"),
    alias!("get_kb_recent"),
    alias!("batch_delete_kb_entries"),
    alias!("batch_move_kb_entries"),
    // ========== 2a-2 段 22 条（导入与解析；裁定 39-A 沿用 2a-1 口径；
    // 4 条模版命令已随批C3 迁出至 knowledge.templates L2） ==========
    alias!("kb_import_multi_folders"),
    alias!("kb_check_files_existence"),
    alias!("kb_scan_directory"),
    alias!("kb_extract_table_data"),
    alias!("kb_read_external_file"),
    alias!("kb_read_file_base64"),
    alias!("kb_extract_docx_text"),
    alias!("kb_extract_psd_info"),
    alias!("kb_extract_ai_info"),
    alias!("kb_list_zip_contents"),
    alias!("kb_extract_epub_text"),
    alias!("kb_extract_odp_text"),
    alias!("kb_extract_doc_text"),
    alias!("kb_extract_rtf_text"),
    alias!("kb_get_tracked_paths"),
    alias!("kb_remove_tracked_path"),
    alias!("kb_add_scanned_files"),
    alias!("kb_check_paths"),
    alias!("kb_get_snapshots"),
    alias!("kb_restore_snapshot"),
    // 批C3：4 条模版命令（kb_get/create/update/delete_template）迁出至 knowledge.templates
    // L2（短码 kt，features/templates/commands.rs）。
    alias!("kb_get_backlinks"),
    alias!("kb_get_outgoing_links"),
    // ========== 2a-3 段 6 条（附件与语义；preload 零消费判「留」入 alias，裁定 43-A） ==========
    alias!("kb_semantic_search"),
    alias!("kb_attachment_pin"),
    alias!("kb_attachment_unpin"),
    alias!("kb_attachment_preload"),
    alias!("kb_attachment_list_pinned"),
    alias!("kb_attachment_get_cached_path"),
];

// ========== dispatcher 业务 handler 分发（S2 门禁：dispatcher 状态注入） ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::kb_commands;
use crate::db::connection::AppState;
use crate::models::knowledge::{AddKbEntryRequest, UpdateKbEntryRequest};

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

fn arg_i64_vec(args: &Json, key: &str) -> Result<Vec<i64>, String> {
    let items = get(args, key)
        .and_then(Json::as_array)
        .ok_or_else(|| format!("参数 {key} 缺失或不是数组"))?;
    items
        .iter()
        .map(|v| {
            v.as_i64()
                .ok_or_else(|| format!("参数 {key} 含非整数元素"))
        })
        .collect()
}

/// 字符串数组参数（`folder_paths` / `paths` / `file_paths`）。
fn arg_str_vec(args: &Json, key: &str) -> Result<Vec<String>, String> {
    let items = get(args, key)
        .and_then(Json::as_array)
        .ok_or_else(|| format!("参数 {key} 缺失或不是数组"))?;
    items
        .iter()
        .map(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| format!("参数 {key} 含非字符串元素"))
        })
        .collect()
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

fn arg_opt_i64(args: &Json, key: &str) -> Result<Option<i64>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| format!("参数 {key} 不是整数或 null")),
    }
}

fn arg_opt_i32(args: &Json, key: &str) -> Result<Option<i32>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .and_then(|v| i32::try_from(v).ok())
            .map(Some)
            .ok_or_else(|| format!("参数 {key} 不是 32 位整数或 null")),
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
    match legacy {
        "get_kb_categories" => {
            let library = arg_opt_str(&args, "library")?;
            to_json(kb_commands::get_kb_categories(state, library).await?)
        }
        "get_kb_category_counts" => to_json(kb_commands::get_kb_category_counts(state).await?),
        "add_kb_category" => {
            let name = arg_str_required(&args, "name")?;
            let parent_id = arg_opt_i64(&args, "parent_id")?;
            let library = arg_opt_str(&args, "library")?;
            let sort_order = arg_opt_i32(&args, "sort_order")?;
            to_json(kb_commands::add_kb_category(state, name, parent_id, library, sort_order).await?)
        }
        "update_kb_category" => {
            let id = arg_i64(&args, "id")?;
            let name = arg_str_required(&args, "name")?;
            to_json(kb_commands::update_kb_category(state, id, name).await?)
        }
        "add_kb_entry" => {
            let request: AddKbEntryRequest = parse_request(&args)?;
            to_json(kb_commands::add_kb_entry(state, request).await?)
        }
        "delete_kb_entry" => {
            let id = arg_i64(&args, "id")?;
            to_json(kb_commands::delete_kb_entry(state, id).await?)
        }
        "move_kb_entry" => {
            let request: kb_commands::MoveEntryRequest = parse_request(&args)?;
            to_json(kb_commands::move_kb_entry(state, request).await?)
        }
        "move_kb_category" => {
            let request: kb_commands::MoveCategoryRequest = parse_request(&args)?;
            to_json(kb_commands::move_kb_category(state, request).await?)
        }
        "update_kb_entry" => {
            let request: UpdateKbEntryRequest = parse_request(&args)?;
            to_json(kb_commands::update_kb_entry(state, request).await?)
        }
        "search_kb_entries" => {
            let query = arg_str_required(&args, "query")?;
            to_json(kb_commands::search_kb_entries(state, query).await?)
        }
        "get_all_kb_entries" => to_json(kb_commands::get_all_kb_entries(state).await?),
        "get_kb_tags" => to_json(kb_commands::get_kb_tags(state).await?),
        "add_kb_tag" => {
            let name = arg_str_required(&args, "name")?;
            let color = arg_opt_str(&args, "color")?;
            to_json(kb_commands::add_kb_tag(state, name, color).await?)
        }
        "update_kb_tag" => {
            let id = arg_i64(&args, "id")?;
            let name = arg_str_required(&args, "name")?;
            let color = arg_str_required(&args, "color")?;
            to_json(kb_commands::update_kb_tag(state, id, name, color).await?)
        }
        "delete_kb_tag" => {
            let id = arg_i64(&args, "id")?;
            to_json(kb_commands::delete_kb_tag(state, id).await?)
        }
        "get_kb_entry_tags" => {
            let entry_id = arg_i64(&args, "entry_id")?;
            to_json(kb_commands::get_kb_entry_tags(state, entry_id).await?)
        }
        "get_kb_all_entry_tags" => to_json(kb_commands::get_kb_all_entry_tags(state).await?),
        "set_kb_entry_tags" => {
            let entry_id = arg_i64(&args, "entry_id")?;
            let tag_ids = arg_i64_vec(&args, "tag_ids")?;
            to_json(kb_commands::set_kb_entry_tags(state, entry_id, tag_ids).await?)
        }
        "get_kb_tag_stats" => to_json(kb_commands::get_kb_tag_stats(state).await?),
        "get_kb_entries_by_tag" => {
            let tag_id = arg_i64(&args, "tag_id")?;
            to_json(kb_commands::get_kb_entries_by_tag(state, tag_id).await?)
        }
        "batch_add_kb_tag" => {
            let entry_ids = arg_i64_vec(&args, "entry_ids")?;
            let tag_id = arg_i64(&args, "tag_id")?;
            to_json(kb_commands::batch_add_kb_tag(state, entry_ids, tag_id).await?)
        }
        "batch_remove_kb_tag" => {
            let entry_ids = arg_i64_vec(&args, "entry_ids")?;
            let tag_id = arg_i64(&args, "tag_id")?;
            to_json(kb_commands::batch_remove_kb_tag(state, entry_ids, tag_id).await?)
        }
        "toggle_kb_favorite" => {
            let entry_id = arg_i64(&args, "entry_id")?;
            to_json(kb_commands::toggle_kb_favorite(state, entry_id).await?)
        }
        "get_kb_favorites" => to_json(kb_commands::get_kb_favorites(state).await?),
        "move_kb_category_to_recycle" => {
            let id = arg_i64(&args, "id")?;
            to_json(kb_commands::move_kb_category_to_recycle(state, id).await?)
        }
        "record_kb_access" => {
            let entry_id = arg_i64(&args, "entry_id")?;
            to_json(kb_commands::record_kb_access(state, entry_id).await?)
        }
        "get_kb_recent" => {
            let limit = arg_opt_i64(&args, "limit")?;
            to_json(kb_commands::get_kb_recent(state, limit).await?)
        }
        "batch_delete_kb_entries" => {
            let request: kb_commands::BatchDeleteRequest = parse_request(&args)?;
            to_json(kb_commands::batch_delete_kb_entries(state, request).await?)
        }
        "batch_move_kb_entries" => {
            let request: kb_commands::BatchMoveRequest = parse_request(&args)?;
            to_json(kb_commands::batch_move_kb_entries(state, request).await?)
        }
        // ========== 2a-2 段（导入与解析；4 条模版命令已随批C3 迁出） ==========
        "kb_import_multi_folders" => {
            let folder_paths = arg_str_vec(&args, "folder_paths")?;
            let category_id = arg_i64(&args, "category_id")?;
            let library = arg_opt_str(&args, "library")?;
            to_json(
                kb_commands::kb_import_multi_folders(state, folder_paths, category_id, library)
                    .await?,
            )
        }
        "kb_check_files_existence" => {
            let paths = arg_str_vec(&args, "paths")?;
            to_json(kb_commands::kb_check_files_existence(state, paths).await?)
        }
        "kb_scan_directory" => {
            let folder_path = arg_str_required(&args, "folder_path")?;
            to_json(kb_commands::kb_scan_directory(state, folder_path).await?)
        }
        "kb_extract_table_data" => {
            let path = arg_str_required(&args, "path")?;
            to_json(kb_commands::kb_extract_table_data(state, path).await?)
        }
        "kb_read_external_file" => {
            let path = arg_str_required(&args, "path")?;
            to_json(kb_commands::kb_read_external_file(state, path).await?)
        }
        "kb_read_file_base64" => {
            let path = arg_str_required(&args, "path")?;
            to_json(kb_commands::kb_read_file_base64(state, path).await?)
        }
        "kb_extract_docx_text" => {
            let path = arg_str_required(&args, "path")?;
            to_json(kb_commands::kb_extract_docx_text(state, path).await?)
        }
        "kb_extract_psd_info" => {
            let path = arg_str_required(&args, "path")?;
            to_json(kb_commands::kb_extract_psd_info(state, path).await?)
        }
        "kb_extract_ai_info" => {
            let path = arg_str_required(&args, "path")?;
            to_json(kb_commands::kb_extract_ai_info(state, path).await?)
        }
        "kb_list_zip_contents" => {
            let path = arg_str_required(&args, "path")?;
            to_json(kb_commands::kb_list_zip_contents(state, path).await?)
        }
        "kb_extract_epub_text" => {
            let path = arg_str_required(&args, "path")?;
            to_json(kb_commands::kb_extract_epub_text(state, path).await?)
        }
        "kb_extract_odp_text" => {
            let path = arg_str_required(&args, "path")?;
            to_json(kb_commands::kb_extract_odp_text(state, path).await?)
        }
        "kb_extract_doc_text" => {
            let path = arg_str_required(&args, "path")?;
            to_json(kb_commands::kb_extract_doc_text(state, path).await?)
        }
        "kb_extract_rtf_text" => {
            let path = arg_str_required(&args, "path")?;
            to_json(kb_commands::kb_extract_rtf_text(state, path).await?)
        }
        "kb_get_tracked_paths" => {
            let library = arg_opt_str(&args, "library")?;
            to_json(kb_commands::kb_get_tracked_paths(state, library).await?)
        }
        "kb_remove_tracked_path" => {
            let id = arg_i64(&args, "id")?;
            to_json(kb_commands::kb_remove_tracked_path(state, id).await?)
        }
        "kb_add_scanned_files" => {
            let file_paths = arg_str_vec(&args, "file_paths")?;
            let category_id = arg_i64(&args, "category_id")?;
            let source_path = arg_str_required(&args, "source_path")?;
            to_json(
                kb_commands::kb_add_scanned_files(state, file_paths, category_id, source_path)
                    .await?,
            )
        }
        "kb_check_paths" => to_json(kb_commands::kb_check_paths(state).await?),
        "kb_get_snapshots" => {
            let entry_id = arg_i64(&args, "entry_id")?;
            to_json(kb_commands::kb_get_snapshots(state, entry_id).await?)
        }
        "kb_restore_snapshot" => {
            let snapshot_id = arg_i64(&args, "snapshot_id")?;
            to_json(kb_commands::kb_restore_snapshot(state, snapshot_id).await?)
        }
        // 批C3：4 条模版命令迁出至 knowledge.templates L2（features/templates/commands.rs）
        "kb_get_backlinks" => {
            let entry_id = arg_i64(&args, "entry_id")?;
            to_json(kb_commands::kb_get_backlinks(state, entry_id).await?)
        }
        "kb_get_outgoing_links" => {
            let entry_id = arg_i64(&args, "entry_id")?;
            to_json(kb_commands::kb_get_outgoing_links(state, entry_id).await?)
        }
        // ========== 2a-3 段 6 条（附件与语义） ==========
        "kb_semantic_search" => {
            let request: kb_commands::SemanticSearchRequest = parse_request(&args)?;
            to_json(kb_commands::kb_semantic_search(state, request).await?)
        }
        "kb_attachment_pin" => {
            let entry_id = arg_i64(&args, "entry_id")?;
            let file_path = arg_str_required(&args, "file_path")?;
            to_json(kb_commands::kb_attachment_pin(state, entry_id, file_path).await?)
        }
        "kb_attachment_unpin" => {
            let entry_id = arg_i64(&args, "entry_id")?;
            to_json(kb_commands::kb_attachment_unpin(state, entry_id).await?)
        }
        "kb_attachment_preload" => {
            let entry_id = arg_i64(&args, "entry_id")?;
            to_json(kb_commands::kb_attachment_preload(state, entry_id).await?)
        }
        "kb_attachment_list_pinned" => {
            to_json(kb_commands::kb_attachment_list_pinned(state).await?)
        }
        "kb_attachment_get_cached_path" => {
            let entry_id = arg_i64(&args, "entry_id")?;
            to_json(kb_commands::kb_attachment_get_cached_path(state, entry_id).await?)
        }
        _ => Err(format!("未知 boards.knowledge 逻辑命令: {legacy}")),
    }
}

/// 必填字符串参数（`arg_str` 与 `arg_opt_str` 的区分仅在于是否允许 null）。
fn arg_str_required(args: &Json, key: &str) -> Result<String, String> {
    get(args, key)
        .and_then(Json::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("参数 {key} 缺失或不是字符串"))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// 2a-1 段 29 条 + 2a-2 段 22 条（4 条模版命令已随批C3 迁出）+ 2a-3 段 6 条
    ///（与 `阶段3` 拆批口径一致；5 条判删不入 alias）
    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "get_kb_categories",
        "get_kb_category_counts",
        "add_kb_category",
        "update_kb_category",
        "add_kb_entry",
        "delete_kb_entry",
        "move_kb_entry",
        "move_kb_category",
        "update_kb_entry",
        "search_kb_entries",
        "get_all_kb_entries",
        "get_kb_tags",
        "add_kb_tag",
        "update_kb_tag",
        "delete_kb_tag",
        "get_kb_entry_tags",
        "get_kb_all_entry_tags",
        "set_kb_entry_tags",
        "get_kb_tag_stats",
        "get_kb_entries_by_tag",
        "batch_add_kb_tag",
        "batch_remove_kb_tag",
        "toggle_kb_favorite",
        "get_kb_favorites",
        "move_kb_category_to_recycle",
        "record_kb_access",
        "get_kb_recent",
        "batch_delete_kb_entries",
        "batch_move_kb_entries",
        // 2a-2 段（导入与解析）
        "kb_import_multi_folders",
        "kb_check_files_existence",
        "kb_scan_directory",
        "kb_extract_table_data",
        "kb_read_external_file",
        "kb_read_file_base64",
        "kb_extract_docx_text",
        "kb_extract_psd_info",
        "kb_extract_ai_info",
        "kb_list_zip_contents",
        "kb_extract_epub_text",
        "kb_extract_odp_text",
        "kb_extract_doc_text",
        "kb_extract_rtf_text",
        "kb_get_tracked_paths",
        "kb_remove_tracked_path",
        "kb_add_scanned_files",
        "kb_check_paths",
        "kb_get_snapshots",
        "kb_restore_snapshot",
        "kb_get_backlinks",
        "kb_get_outgoing_links",
        // 2a-3 段（附件与语义）
        "kb_semantic_search",
        "kb_attachment_pin",
        "kb_attachment_unpin",
        "kb_attachment_preload",
        "kb_attachment_list_pinned",
        "kb_attachment_get_cached_path",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 57);
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

    /// 双形态参数键探测：snake_case 优先、camelCase 回退（前端两条通道口径实证）。
    #[test]
    fn arg_lookup_accepts_both_key_shapes() {
        let snake = serde_json::json!({ "entry_id": 7 });
        let camel = serde_json::json!({ "entryId": 7 });
        let both = serde_json::json!({ "entry_id": 7, "entryId": 9 });

        assert_eq!(arg_i64(&snake, "entry_id").unwrap(), 7);
        assert_eq!(arg_i64(&camel, "entry_id").unwrap(), 7);
        assert_eq!(arg_i64(&both, "entry_id").unwrap(), 7, "snake_case 优先");
        assert!(arg_i64(&serde_json::json!({}), "entry_id").is_err());
    }
}
