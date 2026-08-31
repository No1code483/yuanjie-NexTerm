use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::yuancode::{
    CodeAnalysisRequest, CodeAnalysisResult, CodeCompletionRequest, CodeCompletionResult,
    CodeExecutionRequest, CodeExecutionResult, CodeSnippet,
    CopyMoveRequest, CreateItemRequest, DeleteItemRequest,
    DiffRequest, DiffResult, FileInfoRequest, FileInfoResult, FileTreeNode,
    HighlightRequest, HighlightResult, ListFilesRequest,
    ReadFileRequest, RenameItemRequest, ReplaceFilesRequest, SaveSnippetRequest, SaveWorkspaceRequest,
    SearchFilesRequest, SearchMatch, UpdateSnippetRequest,
    WorkspaceSession, WriteFileRequest, YuanCodeSettings,
};
use crate::services::yuancode_service;

async fn get_user_id(state: &State<'_, AppState>) -> Result<i64, String> {
    let user = state.current_user.read().await;
    user.ok_or_else(|| String::from("未登录"))
}

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn yuan_list_files(
    state: State<'_, AppState>,
    request: ListFilesRequest,
) -> Result<ApiResponse<Vec<FileTreeNode>>, String> {
    crate::commands::common::require_auth(&state).await?;
    yuancode_service::list_files(request)
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_read_file(
    state: State<'_, AppState>,
    request: ReadFileRequest,
) -> Result<ApiResponse<crate::models::yuancode::FileContent>, String> {
    crate::commands::common::require_auth(&state).await?;
    yuancode_service::read_file(request)
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_write_file(
    state: State<'_, AppState>,
    request: WriteFileRequest,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    yuancode_service::write_file(request)
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_create_item(
    state: State<'_, AppState>,
    request: CreateItemRequest,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    yuancode_service::create_item(request)
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_delete_item(
    state: State<'_, AppState>,
    request: DeleteItemRequest,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    yuancode_service::delete_item(request)
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_rename_item(
    state: State<'_, AppState>,
    request: RenameItemRequest,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    yuancode_service::rename_item(request)
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_highlight(
    state: State<'_, AppState>,
    request: HighlightRequest,
) -> Result<ApiResponse<HighlightResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    yuancode_service::highlight_code(request)
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_execute(
    state: State<'_, AppState>,
    request: CodeExecutionRequest,
) -> Result<ApiResponse<CodeExecutionResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    yuancode_service::execute_code(request).await
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_complete(
    state: State<'_, AppState>,
    request: CodeCompletionRequest,
) -> Result<ApiResponse<CodeCompletionResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    yuancode_service::ai_complete(&state.pool, &state.mek_manager, user_id, request).await
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

/// D1.2 流式补全：emit `ai-stream` (conversation_id=-1) + `yuan-code-stream-done`
#[tauri::command]
pub async fn yuan_complete_stream(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    request: CodeCompletionRequest,
) -> Result<ApiResponse<CodeCompletionResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    yuancode_service::ai_complete_stream(&state.pool, &state.mek_manager, user_id, request, &app_handle)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_analyze(
    state: State<'_, AppState>,
    request: CodeAnalysisRequest,
) -> Result<ApiResponse<CodeAnalysisResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    yuancode_service::analyze_code(&state.pool, &state.mek_manager, user_id, request).await
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_save_snippet(
    state: State<'_, AppState>,
    request: SaveSnippetRequest,
) -> Result<ApiResponse<CodeSnippet>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    yuancode_service::save_snippet(&state.pool, user_id, request).await
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_get_snippets(
    state: State<'_, AppState>,
    language: Option<String>,
) -> Result<ApiResponse<Vec<CodeSnippet>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    yuancode_service::get_snippets(&state.pool, user_id, language.as_deref()).await
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_update_snippet(
    state: State<'_, AppState>,
    request: UpdateSnippetRequest,
) -> Result<ApiResponse<Option<CodeSnippet>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    yuancode_service::update_snippet(&state.pool, user_id, request).await
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_delete_snippet(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<bool>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    yuancode_service::delete_snippet(&state.pool, user_id, id).await
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_search_snippets(
    state: State<'_, AppState>,
    query: String,
) -> Result<ApiResponse<Vec<CodeSnippet>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    yuancode_service::search_snippets(&state.pool, user_id, &query).await
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_compute_diff(
    state: State<'_, AppState>,
    request: DiffRequest,
) -> Result<ApiResponse<DiffResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    yuancode_service::compute_diff(request)
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_search_files(
    state: State<'_, AppState>,
    request: SearchFilesRequest,
) -> Result<ApiResponse<Vec<SearchMatch>>, String> {
    crate::commands::common::require_auth(&state).await?;
    yuancode_service::search_files(&request)
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_replace_files(
    state: State<'_, AppState>,
    request: ReplaceFilesRequest,
) -> Result<ApiResponse<usize>, String> {
    crate::commands::common::require_auth(&state).await?;
    yuancode_service::replace_files(&request)
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_copy_move(
    state: State<'_, AppState>,
    request: CopyMoveRequest,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    yuancode_service::copy_move_item(&request)
        .map(|_| ApiResponse::success(true))
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_get_file_info(
    state: State<'_, AppState>,
    request: FileInfoRequest,
) -> Result<ApiResponse<FileInfoResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    yuancode_service::get_file_info(&request)
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_save_workspace(
    state: State<'_, AppState>,
    request: SaveWorkspaceRequest,
) -> Result<ApiResponse<i64>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    yuancode_service::save_workspace(&state.pool, user_id, request).await
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_load_workspace(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<Option<WorkspaceSession>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    yuancode_service::load_workspace(&state.pool, user_id, id).await
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_list_workspaces(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<WorkspaceSession>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    yuancode_service::list_workspaces(&state.pool, user_id).await
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_delete_workspace(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<bool>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    yuancode_service::delete_workspace(&state.pool, user_id, id).await
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_format_code(
    state: State<'_, AppState>,
    request: crate::models::yuancode::FormatRequest,
) -> Result<ApiResponse<crate::models::yuancode::FormatResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    yuancode_service::format_code(request).await
        .map(ApiResponse::success)
        .map_err(|e| e.into())
}

#[tauri::command]
pub async fn yuan_settings_save(
    state: State<'_, AppState>,
    settings: YuanCodeSettings,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    let config_dir = dirs_next::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("nexterm")
        .join("yuancode");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("创建配置目录失败: {}", e))?;
    let config_path = config_dir.join("settings.json");
    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    std::fs::write(&config_path, json)
        .map_err(|e| format!("保存配置失败: {}", e))?;
    tracing::info!("YuanCode settings saved to {:?}", config_path);
    Ok(ApiResponse::success(()))
}