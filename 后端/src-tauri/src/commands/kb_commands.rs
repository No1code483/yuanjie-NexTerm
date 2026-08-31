use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::knowledge::{AddKbEntryRequest, KbCategory, KbEntry, KbTag, KbTrackedPath, KbTemplate, KbSnapshotResult, CreateKbTemplateRequest, UpdateKbTemplateRequest, UpdateKbEntryRequest, EntryTagsResult, ScanDirFileInfo, PathCheckResult, TagStats, BacklinkResult};
use serde::{Serialize, Deserialize};
use crate::services::intelligence_v4_service;
use crate::services::kb_embedding_service;
use crate::services::kb_service;
use crate::services::kb_attachment_cache_service;
use crate::commands::common::require_auth;

static NOTES_BASE_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

fn default_notes_dir() -> &'static std::path::Path {
    NOTES_BASE_DIR.get_or_init(|| {
        let mut cwd = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        if cwd.ends_with("src-tauri") {
            cwd.pop();
        }
        cwd.join("内部笔记")
    }).as_path()
}

#[derive(Serialize, Deserialize)]
pub struct ReadExternalFileResult {
    pub content: String,
    pub size_bytes: u64,
    pub extension: String,
}

#[derive(Serialize, Deserialize)]
pub struct ReadFileBase64Result {
    pub base64: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub extension: String,
}

#[derive(Serialize)]
pub struct ImportFolderResult {
    pub categories: Vec<KbCategory>,
    pub entries: Vec<KbEntry>,
}

#[derive(Serialize)]
pub struct MultiImportResult {
    pub categories: Vec<KbCategory>,
    pub entries: Vec<KbEntry>,
    pub success_count: u32,
    pub fail_count: u32,
}

#[derive(Serialize, Deserialize)]
pub struct TableDataResult {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub sheet_name: String,
    pub total_rows: usize,
    pub total_cols: usize,
    pub extension: String,
}

#[derive(Serialize, Deserialize)]
pub struct PptxSlideTextBlock {
    pub text: String,
    pub is_bold: bool,
    pub is_italic: bool,
    pub font_size_pt: Option<f64>,
}

#[derive(Serialize, Deserialize)]
pub struct PptxSlideImage {
    pub base64: String,
    pub mime_type: String,
}

#[derive(Serialize, Deserialize)]
pub struct PptxSlide {
    pub slide_number: u32,
    pub slide_image_base64: Option<String>,
    pub texts: Vec<PptxSlideTextBlock>,
    pub images: Vec<PptxSlideImage>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PptxExtractResult {
    pub slides: Vec<PptxSlide>,
    pub slide_count: u32,
    pub render_mode: String,
}

#[derive(Serialize)]
pub struct CategoryCount {
    pub category_id: i64,
    pub count: i64,
}

#[derive(Serialize)]
pub struct RecursiveDeleteResult {
    pub deleted_subfolders: i64,
    pub deleted_entries: i64,
}

#[tauri::command]
pub async fn get_kb_categories(
    state: State<'_, AppState>,
    library: Option<String>,
) -> Result<ApiResponse<Vec<KbCategory>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_service::get_categories(&state.pool, user_id, library.as_deref()).await {
        Ok(cats) => Ok(ApiResponse::success(cats)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn get_kb_category_counts(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<CategoryCount>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_service::get_category_entry_counts(&state.pool, user_id).await {
        Ok(rows) => {
            let counts: Vec<CategoryCount> = rows
                .into_iter()
                .map(|(category_id, count)| CategoryCount { category_id, count })
                .collect();
            Ok(ApiResponse::success(counts))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn add_kb_category(
    state: State<'_, AppState>,
    name: String,
    parent_id: Option<i64>,
    library: Option<String>,
    sort_order: Option<i32>,
) -> Result<ApiResponse<KbCategory>, String> {
    let user_id = require_auth(&state).await?;
    let lib = library.unwrap_or_else(|| "material".to_string());
    match kb_service::add_category(&state.pool, user_id, &name, parent_id, &lib, sort_order.unwrap_or(0)).await {
        Ok(cat) => {
            intelligence_v4_service::instrument_cmd(&state, "knowledge_base", &format!("添加分类/{}", cat.name), None).await;
            Ok(ApiResponse::success(cat))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn delete_kb_category(
    state: State<'_, AppState>,
    id: i64,
    recursive: Option<bool>,
) -> Result<ApiResponse<RecursiveDeleteResult>, String> {
    let user_id = require_auth(&state).await?;
    if recursive.unwrap_or(false) {
        match kb_service::recursive_delete_category(&state.pool, user_id, id).await {
            Ok((subfolders, entries)) => Ok(ApiResponse::success(RecursiveDeleteResult {
                deleted_subfolders: subfolders,
                deleted_entries: entries,
            })),
            Err(e) => Err(e.into()),
        }
    } else {
        let children = kb_service::get_child_category_ids(&state.pool, user_id, id).await
            .map_err(|e| String::from(e))?;
        if !children.is_empty() {
            return Err("该分类下有子分类，请使用 recursive=true 递归删除".to_string());
        }
        match kb_service::delete_category(&state.pool, user_id, id).await {
            Ok(_) => Ok(ApiResponse::success(RecursiveDeleteResult {
                deleted_subfolders: 0,
                deleted_entries: 0,
            })),
            Err(e) => Err(e.into()),
        }
    }
}

#[tauri::command]
pub async fn update_kb_category(
    state: State<'_, AppState>,
    id: i64,
    name: String,
) -> Result<ApiResponse<KbCategory>, String> {
    let user_id = require_auth(&state).await?;
    match kb_service::update_category(&state.pool, user_id, id, &name).await {
        Ok(cat) => Ok(ApiResponse::success(cat)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn get_kb_entries(
    state: State<'_, AppState>,
    category_id: i64,
) -> Result<ApiResponse<Vec<KbEntry>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_service::get_entries(&state.pool, user_id, category_id).await {
        Ok(entries) => Ok(ApiResponse::success(entries)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn add_kb_entry(
    state: State<'_, AppState>,
    request: AddKbEntryRequest,
) -> Result<ApiResponse<KbEntry>, String> {
    let user_id = require_auth(&state).await?;
    let notes_base_dir = default_notes_dir();
    match kb_service::add_entry(
        &state.pool,
        user_id,
        request.category_id,
        &request.name,
        &request.path_url,
        &request.entry_type,
        notes_base_dir,
    )
    .await
    {
        Ok(entry) => {
            let op = format!("添加条目/{}", entry.name);
            intelligence_v4_service::instrument_cmd(&state, "knowledge_base", &op, None).await;
            Ok(ApiResponse::success(entry))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn delete_kb_entry(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;
    match kb_service::delete_entry(&state.pool, user_id, id).await {
        Ok(_) => {
            intelligence_v4_service::instrument_cmd(&state, "knowledge_base", "删除条目", None).await;
            Ok(ApiResponse::success(()))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn move_kb_entry(
    state: State<'_, AppState>,
    request: MoveEntryRequest,
) -> Result<ApiResponse<KbEntry>, String> {
    let user_id = require_auth(&state).await?;
    match kb_service::move_entry(&state.pool, user_id, request.id, request.target_category_id).await {
        Ok(entry) => Ok(ApiResponse::success(entry)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn move_kb_category(
    state: State<'_, AppState>,
    request: MoveCategoryRequest,
) -> Result<ApiResponse<KbCategory>, String> {
    let user_id = require_auth(&state).await?;
    match kb_service::move_category(&state.pool, user_id, request.id, request.target_parent_id, request.target_library).await {
        Ok(cat) => Ok(ApiResponse::success(cat)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn update_kb_entry(
    state: State<'_, AppState>,
    request: UpdateKbEntryRequest,
) -> Result<ApiResponse<KbEntry>, String> {
    let user_id = require_auth(&state).await?;
    let notes_base_dir = default_notes_dir();
    match kb_service::update_entry(
        &state.pool,
        user_id,
        request.id,
        request.name.as_deref(),
        request.path_url.as_deref(),
        request.entry_type.as_deref(),
        request.category_id,
        request.content.as_deref(),
        notes_base_dir,
    )
    .await
    {
        Ok(entry) => {
            let name = entry.name.clone();
            intelligence_v4_service::instrument_cmd(&state, "knowledge_base", &format!("编辑条目/{}", name), None).await;
            Ok(ApiResponse::success(entry))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn search_kb_entries(
    state: State<'_, AppState>,
    query: String,
) -> Result<ApiResponse<Vec<KbEntry>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_service::search_entries(&state.pool, user_id, &query).await {
        Ok(entries) => {
            let op = format!("搜索/{}", query);
            intelligence_v4_service::instrument_cmd(&state, "knowledge_base", &op, None).await;
            Ok(ApiResponse::success(entries))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn kb_import_folder(
    state: State<'_, AppState>,
    folder_path: String,
    category_id: i64,
    library: Option<String>,
) -> Result<ApiResponse<ImportFolderResult>, String> {
    let user_id = require_auth(&state).await?;
    let lib = library.unwrap_or_else(|| "material".to_string());
    match kb_service::import_folder(&state.pool, user_id, &folder_path, category_id, &lib).await {
        Ok((categories, entries)) => {
            let total = categories.len() + entries.len();
            intelligence_v4_service::instrument_cmd(&state, "knowledge_base", &format!("导入文件夹/{}项", total), None).await;
            Ok(ApiResponse::success(ImportFolderResult { categories, entries }))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn kb_import_multi_folders(
    state: State<'_, AppState>,
    folder_paths: Vec<String>,
    category_id: i64,
    library: Option<String>,
) -> Result<ApiResponse<MultiImportResult>, String> {
    let user_id = require_auth(&state).await?;
    let lib = library.unwrap_or_else(|| "material".to_string());
    match kb_service::import_multiple_folders(&state.pool, user_id, &folder_paths, category_id, &lib).await {
        Ok((categories, entries, success_count, fail_count)) => {
            let total = categories.len() + entries.len();
            intelligence_v4_service::instrument_cmd(&state, "knowledge_base", &format!("批量导入/{}/{}个成功", total, success_count), None).await;
            Ok(ApiResponse::success(MultiImportResult {
                categories,
                entries,
                success_count,
                fail_count,
            }))
        }
        Err(e) => Err(e.into()),
    }
}

/// 批量检测文件路径是否存在（前端调用，用于标记失效文件）
#[tauri::command]
pub async fn kb_check_files_existence(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<ApiResponse<std::collections::HashMap<String, bool>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut result = std::collections::HashMap::new();
    for path in &paths {
        if !path.is_empty() {
            result.insert(path.clone(), std::path::Path::new(path).exists());
        } else {
            result.insert(path.clone(), true);
        }
    }
    Ok(ApiResponse::success(result))
}

#[tauri::command]
pub async fn get_all_kb_entries(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<KbEntry>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_service::get_all_entries(&state.pool, user_id).await {
        Ok(entries) => Ok(ApiResponse::success(entries)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn get_kb_tags(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<KbTag>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_service::get_tags(&state.pool, user_id).await {
        Ok(tags) => Ok(ApiResponse::success(tags)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn add_kb_tag(
    state: State<'_, AppState>,
    name: String,
    color: Option<String>,
) -> Result<ApiResponse<KbTag>, String> {
    let user_id = require_auth(&state).await?;
    let c = color.unwrap_or_else(|| "#00F0FF".to_string());
    match kb_service::add_tag(&state.pool, user_id, &name, &c).await {
        Ok(tag) => Ok(ApiResponse::success(tag)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn update_kb_tag(
    state: State<'_, AppState>,
    id: i64,
    name: String,
    color: String,
) -> Result<ApiResponse<KbTag>, String> {
    let user_id = require_auth(&state).await?;
    match kb_service::update_tag(&state.pool, user_id, id, &name, &color).await {
        Ok(tag) => Ok(ApiResponse::success(tag)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn delete_kb_tag(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;
    match kb_service::delete_tag(&state.pool, user_id, id).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn get_kb_entry_tags(
    state: State<'_, AppState>,
    entry_id: i64,
) -> Result<ApiResponse<Vec<KbTag>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_service::get_entry_tags(&state.pool, user_id, entry_id).await {
        Ok(tags) => Ok(ApiResponse::success(tags)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn get_kb_all_entry_tags(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<EntryTagsResult>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_service::get_all_entry_tags(&state.pool, user_id).await {
        Ok(rows) => Ok(ApiResponse::success(rows)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn set_kb_entry_tags(
    state: State<'_, AppState>,
    entry_id: i64,
    tag_ids: Vec<i64>,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;
    match kb_service::set_entry_tags(&state.pool, user_id, entry_id, &tag_ids).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn get_kb_tag_stats(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<TagStats>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_service::get_tag_stats(&state.pool, user_id).await {
        Ok(stats) => Ok(ApiResponse::success(stats)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn get_kb_entries_by_tag(
    state: State<'_, AppState>,
    tag_id: i64,
) -> Result<ApiResponse<Vec<KbEntry>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_service::get_entries_by_tag(&state.pool, user_id, tag_id).await {
        Ok(entries) => Ok(ApiResponse::success(entries)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn batch_add_kb_tag(
    state: State<'_, AppState>,
    entry_ids: Vec<i64>,
    tag_id: i64,
) -> Result<ApiResponse<u64>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_service::batch_add_tag(&state.pool, user_id, &entry_ids, tag_id).await {
        Ok(count) => Ok(ApiResponse::success(count)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn batch_remove_kb_tag(
    state: State<'_, AppState>,
    entry_ids: Vec<i64>,
    tag_id: i64,
) -> Result<ApiResponse<u64>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_service::batch_remove_tag(&state.pool, user_id, &entry_ids, tag_id).await {
        Ok(count) => Ok(ApiResponse::success(count)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn toggle_kb_favorite(
    state: State<'_, AppState>,
    entry_id: i64,
) -> Result<ApiResponse<bool>, String> {
    let user_id = require_auth(&state).await?;
    match kb_service::toggle_favorite(&state.pool, user_id, entry_id).await {
        Ok(is_fav) => Ok(ApiResponse::success(is_fav)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn get_kb_favorites(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<KbEntry>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_service::get_favorite_entries(&state.pool, user_id).await {
        Ok(entries) => Ok(ApiResponse::success(entries)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn move_kb_category_to_recycle(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<RecursiveDeleteResult>, String> {
    let deleted_by = require_auth(&state).await?;
    // 多用户隔离批次 3：deleted_by 同时充当 user_id 进行隔离过滤
    match kb_service::move_category_tree_to_recycle(&state.pool, deleted_by, id, Some(deleted_by)).await {
        Ok((subfolders, entries)) => Ok(ApiResponse::success(RecursiveDeleteResult {
            deleted_subfolders: subfolders,
            deleted_entries: entries,
        })),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn record_kb_access(
    state: State<'_, AppState>,
    entry_id: i64,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_service::record_access(&state.pool, user_id, entry_id).await {
        Ok(_) => {
            intelligence_v4_service::instrument_cmd(&state, "knowledge_base", "打开条目", None).await;
            Ok(ApiResponse::success(()))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn get_kb_recent(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<ApiResponse<Vec<KbEntry>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_service::get_recent_entries(&state.pool, user_id, limit.unwrap_or(20)).await {
        Ok(entries) => Ok(ApiResponse::success(entries)),
        Err(e) => Err(e.into()),
    }
}

#[derive(Deserialize)]
pub struct BatchDeleteRequest {
    pub ids: Vec<i64>,
}

#[derive(Deserialize)]
pub struct BatchMoveRequest {
    pub ids: Vec<i64>,
    pub category_id: i64,
}

#[derive(Deserialize)]
pub struct MoveEntryRequest {
    pub id: i64,
    pub target_category_id: i64,
}

#[derive(Deserialize)]
pub struct MoveCategoryRequest {
    pub id: i64,
    pub target_parent_id: Option<i64>,
    pub target_library: Option<String>,
}

#[tauri::command]
pub async fn batch_delete_kb_entries(
    state: State<'_, AppState>,
    request: BatchDeleteRequest,
) -> Result<ApiResponse<u64>, String> {
    let user_id = require_auth(&state).await?;
    match kb_service::batch_delete_entries(&state.pool, user_id, &request.ids).await {
        Ok(count) => Ok(ApiResponse::success(count)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn batch_move_kb_entries(
    state: State<'_, AppState>,
    request: BatchMoveRequest,
) -> Result<ApiResponse<u64>, String> {
    let user_id = require_auth(&state).await?;
    match kb_service::batch_move_entries(&state.pool, user_id, &request.ids, request.category_id).await {
        Ok(count) => Ok(ApiResponse::success(count)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn kb_scan_directory(
    state: State<'_, AppState>,
    folder_path: String,
) -> Result<ApiResponse<Vec<ScanDirFileInfo>>, String> {
    crate::commands::common::require_auth(&state).await?;
    match kb_service::scan_directory(&folder_path) {
        Ok(files) => Ok(ApiResponse::success(files)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn kb_extract_table_data(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<TableDataResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    match kb_service::extract_table_data(&path) {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn kb_read_external_file(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<ReadExternalFileResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let file_path = std::path::Path::new(&path);
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match kb_service::read_external_file(&path) {
        Ok(content) => {
            let size = content.len() as u64;
            Ok(ApiResponse::success(ReadExternalFileResult {
                content,
                size_bytes: size,
                extension: ext,
            }))
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn kb_read_file_base64(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<ReadFileBase64Result>, String> {
    crate::commands::common::require_auth(&state).await?;
    match kb_service::read_file_base64(&path) {
        Ok((base64, mime_type, size_bytes)) => {
            let ext = std::path::Path::new(&path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            Ok(ApiResponse::success(ReadFileBase64Result {
                base64,
                mime_type,
                size_bytes,
                extension: ext,
            }))
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn kb_extract_docx_text(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<ReadExternalFileResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("docx")
        .to_lowercase();

    match kb_service::extract_docx_text(&path) {
        Ok(content) => {
            let size = content.len() as u64;
            Ok(ApiResponse::success(ReadExternalFileResult {
                content,
                size_bytes: size,
                extension: ext,
            }))
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn kb_extract_psd_info(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<ReadExternalFileResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    match kb_service::extract_psd_info(&path) {
        Ok(content) => {
            let size_bytes = content.len() as u64;
            Ok(ApiResponse::success(ReadExternalFileResult {
                content,
                size_bytes,
                extension: "psd".to_string(),
            }))
        }
        Err(e) => {
            tracing::warn!("PSD 信息提取失败[{}]: {:?}", path, e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn kb_extract_ai_info(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<ReadExternalFileResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    match kb_service::extract_ai_info(&path) {
        Ok(content) => {
            let size_bytes = content.len() as u64;
            Ok(ApiResponse::success(ReadExternalFileResult {
                content,
                size_bytes,
                extension: "ai".to_string(),
            }))
        }
        Err(e) => {
            tracing::warn!("AI 信息提取失败[{}]: {:?}", path, e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn kb_list_zip_contents(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<ReadExternalFileResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    match kb_service::list_zip_contents(&path) {
        Ok(content) => {
            let size_bytes = content.len() as u64;
            let ext = std::path::Path::new(&path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("zip")
                .to_lowercase();
            Ok(ApiResponse::success(ReadExternalFileResult {
                content,
                size_bytes,
                extension: ext,
            }))
        }
        Err(e) => {
            tracing::warn!("ZIP 内容列表失败[{}]: {:?}", path, e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn kb_extract_epub_text(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<ReadExternalFileResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    match kb_service::extract_epub_text(&path) {
        Ok(content) => {
            let size_bytes = content.len() as u64;
            let ext = std::path::Path::new(&path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("epub")
                .to_lowercase();
            Ok(ApiResponse::success(ReadExternalFileResult {
                content,
                size_bytes,
                extension: ext,
            }))
        }
        Err(e) => {
            tracing::warn!("EPUB 文本提取失败[{}]: {:?}", path, e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn kb_extract_odp_text(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<ReadExternalFileResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    match kb_service::extract_pptx_text(&path) {
        Ok(content) => {
            let size_bytes = content.len() as u64;
            let ext = std::path::Path::new(&path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("odp")
                .to_lowercase();
            Ok(ApiResponse::success(ReadExternalFileResult {
                content,
                size_bytes,
                extension: ext,
            }))
        }
        Err(e) => {
            tracing::warn!("ODP 文本提取失败[{}]: {:?}", path, e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn kb_extract_pptx_text(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<PptxExtractResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    match kb_service::extract_pptx_slides(&path) {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn kb_extract_doc_text(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<ReadExternalFileResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("doc")
        .to_lowercase();

    match kb_service::extract_doc_text(&path) {
        Ok(content) => {
            let size = content.len() as u64;
            Ok(ApiResponse::success(ReadExternalFileResult {
                content,
                size_bytes: size,
                extension: ext,
            }))
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn kb_extract_rtf_text(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<ReadExternalFileResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("rtf")
        .to_lowercase();

    match kb_service::extract_rtf_text(&path) {
        Ok(content) => {
            let size = content.len() as u64;
            Ok(ApiResponse::success(ReadExternalFileResult {
                content,
                size_bytes: size,
                extension: ext,
            }))
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn kb_add_tracked_path(
    state: State<'_, AppState>,
    path: String,
    category_id: i64,
    library: Option<String>,
) -> Result<ApiResponse<KbTrackedPath>, String> {
    let user_id = require_auth(&state).await?;
    let lib = library.unwrap_or_else(|| "material".to_string());
    kb_service::record_tracked_path(&state.pool, user_id, &path, category_id, &lib)
        .await
        .map(|tp| ApiResponse::success(tp))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kb_get_tracked_paths(
    state: State<'_, AppState>,
    library: Option<String>,
) -> Result<ApiResponse<Vec<KbTrackedPath>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let lib = library.as_deref();
    kb_service::get_tracked_paths(&state.pool, user_id, lib)
        .await
        .map(|paths| ApiResponse::success(paths))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kb_remove_tracked_path(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;
    kb_service::remove_tracked_path(&state.pool, user_id, id)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
pub struct ScannedFilesResult {
    pub added: Vec<KbEntry>,
    pub total: usize,
}

#[tauri::command]
pub async fn kb_add_scanned_files(
    state: State<'_, AppState>,
    file_paths: Vec<String>,
    category_id: i64,
    source_path: String,
) -> Result<ApiResponse<ScannedFilesResult>, String> {
    let user_id = require_auth(&state).await?;
    let total = file_paths.len();
    kb_service::add_scanned_files(&state.pool, user_id, &file_paths, category_id, &source_path)
        .await
        .map(|added| ApiResponse::success(ScannedFilesResult { added, total }))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kb_check_paths(
    state: State<'_, AppState>,
) -> Result<ApiResponse<PathCheckResult>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    kb_service::check_paths(&state.pool, user_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kb_get_snapshots(
    state: State<'_, AppState>,
    entry_id: i64,
) -> Result<ApiResponse<Vec<KbSnapshotResult>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    kb_service::get_snapshots(&state.pool, user_id, entry_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kb_restore_snapshot(
    state: State<'_, AppState>,
    snapshot_id: i64,
) -> Result<ApiResponse<KbEntry>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let notes_base_dir = default_notes_dir();
    kb_service::restore_snapshot(&state.pool, user_id, snapshot_id, notes_base_dir)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kb_get_templates(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<KbTemplate>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    kb_service::get_templates(&state.pool, user_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kb_create_template(
    state: State<'_, AppState>,
    request: CreateKbTemplateRequest,
) -> Result<ApiResponse<KbTemplate>, String> {
    let user_id = require_auth(&state).await?;
    kb_service::create_template(
        &state.pool,
        user_id,
        &request.name,
        &request.icon,
        &request.description,
        &request.entry_type,
        &request.content,
    )
    .await
    .map(ApiResponse::success)
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kb_update_template(
    state: State<'_, AppState>,
    request: UpdateKbTemplateRequest,
) -> Result<ApiResponse<KbTemplate>, String> {
    let user_id = require_auth(&state).await?;
    kb_service::update_template(
        &state.pool,
        user_id,
        request.id,
        request.name.as_deref(),
        request.icon.as_deref(),
        request.description.as_deref(),
        request.entry_type.as_deref(),
        request.content.as_deref(),
    )
    .await
    .map(ApiResponse::success)
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kb_delete_template(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<String>, String> {
    let user_id = require_auth(&state).await?;
    kb_service::delete_template(&state.pool, user_id, id)
        .await
        .map(|_| ApiResponse::success("ok".into()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kb_get_backlinks(
    state: State<'_, AppState>,
    entry_id: i64,
) -> Result<ApiResponse<Vec<BacklinkResult>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    kb_service::get_backlinks(&state.pool, user_id, entry_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kb_get_outgoing_links(
    state: State<'_, AppState>,
    entry_id: i64,
) -> Result<ApiResponse<Vec<KbEntry>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    kb_service::get_outgoing_links(&state.pool, user_id, entry_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct SemanticSearchResult {
    pub entry_id: i64,
    pub title: String,
    pub content_preview: String,
    pub score: f32,
    pub category_name: String,
    pub entry_type: String,
}

#[derive(Deserialize)]
pub struct SemanticSearchRequest {
    pub query: String,
    pub category_id: Option<i64>,
    pub limit: Option<u32>,
}

#[tauri::command]
pub async fn kb_semantic_search(
    state: State<'_, AppState>,
    request: SemanticSearchRequest,
) -> Result<ApiResponse<Vec<SemanticSearchResult>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let limit = request.limit.unwrap_or(20).min(100) as usize;
    let all_entries = kb_service::get_all_entries(&state.pool, user_id)
        .await
        .map_err(|e| String::from(e))?;

    // Filter by category if specified
    let entries: Vec<&crate::models::knowledge::KbEntry> = if let Some(cat_id) = request.category_id {
        all_entries.iter().filter(|e| e.category_id == cat_id).collect()
    } else {
        all_entries.iter().collect()
    };

    if entries.is_empty() {
        return Ok(ApiResponse::success(Vec::new()));
    }

    // Build search documents: combine name + content for richer matching
    let documents: Vec<String> = entries
        .iter()
        .map(|e| {
            let content = e.content.as_deref().unwrap_or("");
            format!("{} {}", e.name, content)
        })
        .collect();

    let scored = kb_embedding_service::search_similar(&request.query, &documents, limit);

    let mut results = Vec::new();
    for (idx, score) in scored {
        let entry = entries[idx];
        let content_preview = entry
            .content
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(200)
            .collect::<String>();

        // Get category name
        let category_name = match kb_service::get_category_by_id(&state.pool, user_id, entry.category_id).await {
            Ok(cat) => cat.name,
            Err(_) => String::from("未分类"),
        };

        results.push(SemanticSearchResult {
            entry_id: entry.id,
            title: entry.name.clone(),
            content_preview,
            score,
            category_name,
            entry_type: entry.entry_type.clone(),
        });
    }

    let op = format!("语义搜索/{}", request.query);
    intelligence_v4_service::instrument_cmd(&state, "knowledge_base", &op, None).await;

    Ok(ApiResponse::success(results))
}

// ============================================================================
// A5 离线与同步机制 - Phase 3 Task 4: 知识库附件预加载 IPC 命令
// 规范: 功能展望/平台级增强/04_离线与同步机制.md §Phase 3 Task 4
// ============================================================================

/// 标记附件为「常用」+ 预加载到本地缓存
///
/// - 若已标记，则刷新缓存（重新复制文件）
/// - 复制源文件到 `{app_data_dir}/attachment_cache/{entry_id}.{ext}`
/// - 自动触发 LRU 淘汰（30 天 + 500MB）
#[tauri::command]
pub async fn kb_attachment_pin(
    state: State<'_, AppState>,
    entry_id: i64,
    file_path: String,
) -> Result<ApiResponse<kb_attachment_cache_service::KbAttachmentCache>, String> {
    let user_id = require_auth(&state).await?;
    match kb_attachment_cache_service::pin_attachment(
        &state.pool,
        user_id,
        entry_id,
        &file_path,
        &state.data_dir,
    )
    .await
    {
        Ok(record) => {
            intelligence_v4_service::instrument_cmd(
                &state,
                "knowledge_base",
                "标记常用附件",
                None,
            )
            .await;
            Ok(ApiResponse::success(record))
        }
        Err(e) => Err(e.to_string()),
    }
}

/// 取消「常用」标记 + 删除本地缓存文件
#[tauri::command]
pub async fn kb_attachment_unpin(
    state: State<'_, AppState>,
    entry_id: i64,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;
    match kb_attachment_cache_service::unpin_attachment(&state.pool, user_id, entry_id, &state.data_dir)
        .await
    {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.to_string()),
    }
}

/// 刷新缓存（重新复制源文件，更新 file_size 和 last_accessed_at）
///
/// 用于源文件变更后刷新缓存。仅对已标记的附件有效。
#[tauri::command]
pub async fn kb_attachment_preload(
    state: State<'_, AppState>,
    entry_id: i64,
) -> Result<ApiResponse<kb_attachment_cache_service::KbAttachmentCache>, String> {
    let user_id = require_auth(&state).await?;
    match kb_attachment_cache_service::preload_attachment(
        &state.pool,
        user_id,
        entry_id,
        &state.data_dir,
    )
    .await
    {
        Ok(record) => Ok(ApiResponse::success(record)),
        Err(e) => Err(e.to_string()),
    }
}

/// 列出所有已标记为「常用」的附件（按 last_accessed_at DESC 排序）
#[tauri::command]
pub async fn kb_attachment_list_pinned(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<kb_attachment_cache_service::KbAttachmentCache>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_attachment_cache_service::list_pinned(&state.pool, user_id).await {
        Ok(items) => Ok(ApiResponse::success(items)),
        Err(e) => Err(e.to_string()),
    }
}

/// 获取本地缓存路径（离线打开时使用）
///
/// 同时更新 last_accessed_at（touch access）。
/// 返回 None 表示该附件未被标记为常用，离线时无法访问。
#[tauri::command]
pub async fn kb_attachment_get_cached_path(
    state: State<'_, AppState>,
    entry_id: i64,
) -> Result<ApiResponse<Option<String>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match kb_attachment_cache_service::get_cached_path(&state.pool, user_id, entry_id).await {
        Ok(path) => Ok(ApiResponse::success(path)),
        Err(e) => Err(e.to_string()),
    }
}