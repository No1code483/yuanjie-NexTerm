use tauri::State;

use crate::db::connection::AppState;
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;
use crate::models::project_index::{
    FindReferencesRequest, GetRelatedFilesRequest, IndexProjectRequest, IndexProjectResult,
    ProjectIndexStatus, ReferenceHit, RelatedFile, SearchSymbolsRequest, SymbolSearchHit,
};
use crate::services::project_indexer_service::ProjectIndexer;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

/// 索引项目（全量或增量）
#[tauri::command]
pub async fn project_index(
    _state: State<'_, AppState>,
    request: IndexProjectRequest,
) -> Result<ApiResponse<IndexProjectResult>, String> {
    crate::commands::common::require_auth(&_state).await?;
    ProjectIndexer::index_project(&_state.pool, request)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 获取项目索引状态
#[tauri::command]
pub async fn project_index_status(
    state: State<'_, AppState>,
    root_path: String,
) -> Result<ApiResponse<Option<ProjectIndexStatus>>, String> {
    crate::commands::common::require_auth(&state).await?;
    ProjectIndexer::get_status(&state.pool, &root_path)
        .await
        .map(ApiResponse::success)
        .map_err(|e: AppError| e.to_string())
}

/// 搜索项目符号
#[tauri::command]
pub async fn project_search_symbols(
    state: State<'_, AppState>,
    request: SearchSymbolsRequest,
) -> Result<ApiResponse<Vec<SymbolSearchHit>>, String> {
    crate::commands::common::require_auth(&state).await?;
    ProjectIndexer::search_symbols(&state.pool, request)
        .await
        .map(ApiResponse::success)
        .map_err(|e: AppError| e.to_string())
}

/// 查找符号引用
#[tauri::command]
pub async fn project_find_references(
    state: State<'_, AppState>,
    request: FindReferencesRequest,
) -> Result<ApiResponse<Vec<ReferenceHit>>, String> {
    crate::commands::common::require_auth(&state).await?;
    ProjectIndexer::find_references(&state.pool, request)
        .await
        .map(ApiResponse::success)
        .map_err(|e: AppError| e.to_string())
}

/// 获取相关文件（基于依赖图）
#[tauri::command]
pub async fn project_get_related_files(
    state: State<'_, AppState>,
    request: GetRelatedFilesRequest,
) -> Result<ApiResponse<Vec<RelatedFile>>, String> {
    crate::commands::common::require_auth(&state).await?;
    ProjectIndexer::get_related_files(&state.pool, request)
        .await
        .map(ApiResponse::success)
        .map_err(|e: AppError| e.to_string())
}
