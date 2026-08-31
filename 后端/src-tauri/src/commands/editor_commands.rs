/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::db::connection::AppState;
use crate::crypto::mek_manager::MekManager;
use crate::db::repositories::editor_repo;
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;
use crate::models::editor::{
    AutoSaveRequest, CanvasData, CanvasSaveRequest, ConvertRequest, ConvertResult,
    CsvInfo, CsvPreviewRequest, DecryptedPreview, EditorContent,
    EditorDocument, EditorVersion, HighlightResult, MediaMetadata, OpenDocumentRequest,
    RecoveredSession, SaveDocumentRequest, SearchRequest, SearchResponse,
    ThumbnailRequest, ThumbnailResult, TokenRequest, VersionCleanupRequest,
    VersionCleanupResult, VersionDiffRequest, VersionDiffResult,
};
use crate::services::editor_service;
use crate::services::editor_storage_service::EditorStorageService;
use crate::services::editor_metadata_service;
use crate::services::editor_thumbnail_service;
use crate::services::editor_highlight_service;
use crate::services::editor_csv_service;
use crate::services::editor_preview_service;
use crate::services::editor_canvas_service;
use crate::services::editor_convert_service;
use crate::services::editor_search_service;
use crate::services::editor_version_service;

async fn get_mek(mek_manager: &Arc<RwLock<MekManager>>, user_id: i64) -> Result<[u8; 32], String> {
    let mgr = mek_manager.read().await;
    mgr.get_mek(user_id)
        .copied()
        .ok_or_else(|| AppError::MekDecryption("MEK 未在内存中".into()).to_string())
}

async fn get_user_id(state: &State<'_, AppState>) -> Result<i64, String> {
    let user = state.current_user.read().await;
    user.ok_or_else(|| AppError::Auth("未登录".into()).to_string())
}

fn get_storage(state: &State<'_, AppState>) -> EditorStorageService {
    EditorStorageService::new(state.data_dir.clone())
}

#[tauri::command]
pub async fn editor_open(
    state: State<'_, AppState>,
    request: OpenDocumentRequest,
) -> Result<ApiResponse<EditorContent>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);
    storage.init_dirs().map_err(|e| String::from(e))?;

    match editor_service::open_document(&state.pool, &storage, &mek, request).await {
        Ok(content) => Ok(ApiResponse::success(content)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_save(
    state: State<'_, AppState>,
    request: SaveDocumentRequest,
) -> Result<ApiResponse<EditorContent>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);

    match editor_service::save_document(&state.pool, &storage, &mek, request).await {
        Ok(content) => Ok(ApiResponse::success(content)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_auto_save(
    state: State<'_, AppState>,
    request: AutoSaveRequest,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);

    match editor_service::auto_save(&state.pool, &storage, &mek, request).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_close(
    state: State<'_, AppState>,
    doc_uuid: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    let _user_id = get_user_id(&state).await?;
    let storage = get_storage(&state);

    match editor_service::close_document(&state.pool, &storage, &doc_uuid).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_get_versions(
    state: State<'_, AppState>,
    doc_uuid: String,
) -> Result<ApiResponse<Vec<EditorVersion>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);

    match editor_service::get_versions(&state.pool, &storage, &mek, &doc_uuid).await {
        Ok(versions) => Ok(ApiResponse::success(versions)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_get_version(
    state: State<'_, AppState>,
    doc_uuid: String,
    version_num: i64,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);

    match editor_service::get_version_content(&state.pool, &storage, &mek, &doc_uuid, version_num).await {
        Ok(content) => Ok(ApiResponse::success(content)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_restore_version(
    state: State<'_, AppState>,
    doc_uuid: String,
    version_num: i64,
) -> Result<ApiResponse<EditorContent>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);

    match editor_service::restore_version(&state.pool, &storage, &mek, &doc_uuid, version_num).await {
        Ok(content) => Ok(ApiResponse::success(content)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_recover_session(
    state: State<'_, AppState>,
    doc_uuid: String,
) -> Result<ApiResponse<Option<RecoveredSession>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);

    match editor_service::recover_session(&state.pool, &storage, &mek, &doc_uuid).await {
        Ok(session) => Ok(ApiResponse::success(session)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_list_documents(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<EditorDocument>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let _user_id = get_user_id(&state).await?;

    match editor_service::list_documents(&state.pool).await {
        Ok(docs) => Ok(ApiResponse::success(docs)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_delete_document(
    state: State<'_, AppState>,
    doc_uuid: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    let _user_id = get_user_id(&state).await?;
    let storage = get_storage(&state);

    match editor_service::delete_document(&state.pool, &storage, &doc_uuid).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_extract_metadata(
    state: State<'_, AppState>,
    doc_uuid: String,
    content_type: String,
) -> Result<ApiResponse<MediaMetadata>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);

    match editor_metadata_service::extract_metadata(&storage, &mek, &doc_uuid, &content_type) {
        Ok(meta) => Ok(ApiResponse::success(meta)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_generate_thumbnail(
    state: State<'_, AppState>,
    request: ThumbnailRequest,
) -> Result<ApiResponse<ThumbnailResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);

    match editor_thumbnail_service::generate_thumbnail(
        &storage, &mek, &request.doc_uuid, request.max_width, request.max_height,
    ) {
        Ok(thumb) => Ok(ApiResponse::success(thumb)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_highlight(
    state: State<'_, AppState>,
    request: TokenRequest,
) -> Result<ApiResponse<HighlightResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    match editor_highlight_service::highlight_content(&request.content, &request.language) {
        Ok(highlight) => Ok(ApiResponse::success(highlight)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_csv_preview(
    state: State<'_, AppState>,
    request: CsvPreviewRequest,
) -> Result<ApiResponse<CsvInfo>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);

    match editor_csv_service::read_csv_preview(
        &storage, &mek, &request.doc_uuid, request.max_rows, request.delimiter,
    ) {
        Ok(csv) => Ok(ApiResponse::success(csv)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_decrypted_preview(
    state: State<'_, AppState>,
    doc_uuid: String,
    content_type: String,
) -> Result<ApiResponse<DecryptedPreview>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);

    match editor_preview_service::get_decrypted_preview(&storage, &mek, &doc_uuid, &content_type) {
        Ok(preview) => Ok(ApiResponse::success(preview)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_search(
    state: State<'_, AppState>,
    request: SearchRequest,
) -> Result<ApiResponse<SearchResponse>, String> {
    crate::commands::common::require_auth(&state).await?;
    match editor_search_service::search(&state.pool, request).await {
        Ok(response) => Ok(ApiResponse::success(response)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_convert(
    state: State<'_, AppState>,
    request: ConvertRequest,
) -> Result<ApiResponse<ConvertResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);

    let doc = editor_repo::get_document(&state.pool, &request.doc_uuid)
        .await
        .map_err(|e| String::from(e))?;
    let doc = doc.ok_or_else(|| AppError::NotFound.to_string())?;

    match editor_convert_service::convert_document(
        &storage, &mek, &request.doc_uuid, &doc.content_type, &request.target_format,
    ) {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_convert_content(
    state: State<'_, AppState>,
    content: String,
    source_format: String,
    target_format: String,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    match editor_convert_service::convert_content(&content, &source_format, &target_format) {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_canvas_save(
    state: State<'_, AppState>,
    request: CanvasSaveRequest,
) -> Result<ApiResponse<CanvasData>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);

    match editor_canvas_service::save_canvas(&storage, &mek, request) {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_canvas_load(
    state: State<'_, AppState>,
    doc_uuid: String,
) -> Result<ApiResponse<Option<CanvasData>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);

    match editor_canvas_service::load_canvas(&storage, &mek, &doc_uuid) {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_canvas_delete(
    state: State<'_, AppState>,
    doc_uuid: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    let storage = get_storage(&state);

    match editor_canvas_service::delete_canvas(&storage, &doc_uuid) {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_diff_versions(
    state: State<'_, AppState>,
    request: VersionDiffRequest,
) -> Result<ApiResponse<VersionDiffResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);

    match editor_version_service::diff_versions(
        &state.pool, &storage, &mek, &request.doc_uuid, request.version_a, request.version_b,
    ).await {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_cleanup_versions(
    state: State<'_, AppState>,
    request: VersionCleanupRequest,
) -> Result<ApiResponse<VersionCleanupResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let storage = get_storage(&state);

    match editor_version_service::cleanup_versions(&state.pool, &storage, request).await {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_index_content(
    state: State<'_, AppState>,
    doc_uuid: String,
    title: String,
    content_type: String,
    content: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match editor_search_service::index_document(
        &state.pool, &doc_uuid, &title, &content_type, &content,
    ).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_recover_all_sessions(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<RecoveredSession>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let mek = get_mek(&state.mek_manager, user_id).await?;
    let storage = get_storage(&state);

    match editor_service::recover_all_sessions(&state.pool, &storage, &mek).await {
        Ok(sessions) => Ok(ApiResponse::success(sessions)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn editor_cleanup_sessions(
    state: State<'_, AppState>,
    max_age_hours: Option<i64>,
) -> Result<ApiResponse<u64>, String> {
    crate::commands::common::require_auth(&state).await?;
    let storage = get_storage(&state);
    let max_age = max_age_hours.unwrap_or(24);

    match editor_service::cleanup_expired_sessions(&state.pool, &storage, max_age).await {
        Ok(count) => Ok(ApiResponse::success(count)),
        Err(e) => Err(e.into()),
    }
}