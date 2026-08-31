use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::services::git_service::{
    GitService, GitStatusResult, GitCommit, GitBranchesResult,
};

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[derive(Debug, serde::Deserialize)]
pub struct WorkspacePathRequest {
    pub workspace_path: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct GitFileRequest {
    pub workspace_path: String,
    pub file_path: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct GitCommitRequest {
    pub workspace_path: String,
    pub message: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct GitPushRequest {
    pub workspace_path: String,
    pub remote: String,
    pub branch: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct GitPullRequest {
    pub workspace_path: String,
    pub remote: String,
    pub branch: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct GitCheckoutRequest {
    pub workspace_path: String,
    pub branch: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct GitLogRequest {
    pub workspace_path: String,
    pub count: u32,
}

#[tauri::command]
pub async fn git_status(
    state: State<'_, AppState>,
    request: WorkspacePathRequest,
    git_service: State<'_, GitService>,
) -> Result<ApiResponse<GitStatusResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    git_service.get_status(&request.workspace_path)
        .map(ApiResponse::success)
        .map_err(|e| format!("{}", e))
}

#[tauri::command]
pub async fn git_diff_file(
    state: State<'_, AppState>,
    request: GitFileRequest,
    git_service: State<'_, GitService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    git_service.get_diff_file(&request.workspace_path, &request.file_path)
        .map(ApiResponse::success)
        .map_err(|e| format!("{}", e))
}

#[tauri::command]
pub async fn git_diff_unstaged(
    state: State<'_, AppState>,
    request: WorkspacePathRequest,
    git_service: State<'_, GitService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    git_service.get_diff_unstaged(&request.workspace_path)
        .map(ApiResponse::success)
        .map_err(|e| format!("{}", e))
}

#[tauri::command]
pub async fn git_stage_file(
    state: State<'_, AppState>,
    request: GitFileRequest,
    git_service: State<'_, GitService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    git_service.stage_file(&request.workspace_path, &request.file_path)
        .map(|_| ApiResponse::success("ok".to_string()))
        .map_err(|e| format!("{}", e))
}

#[tauri::command]
pub async fn git_stage_all(
    state: State<'_, AppState>,
    request: WorkspacePathRequest,
    git_service: State<'_, GitService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    git_service.stage_all(&request.workspace_path)
        .map(|_| ApiResponse::success("ok".to_string()))
        .map_err(|e| format!("{}", e))
}

#[tauri::command]
pub async fn git_unstage_file(
    state: State<'_, AppState>,
    request: GitFileRequest,
    git_service: State<'_, GitService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    git_service.unstage_file(&request.workspace_path, &request.file_path)
        .map(|_| ApiResponse::success("ok".to_string()))
        .map_err(|e| format!("{}", e))
}

#[tauri::command]
pub async fn git_commit(
    state: State<'_, AppState>,
    request: GitCommitRequest,
    git_service: State<'_, GitService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    git_service.commit(&request.workspace_path, &request.message)
        .map(ApiResponse::success)
        .map_err(|e| format!("{}", e))
}

#[tauri::command]
pub async fn git_push(
    state: State<'_, AppState>,
    request: GitPushRequest,
    git_service: State<'_, GitService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    git_service.push(&request.workspace_path, &request.remote, &request.branch)
        .map(|_| ApiResponse::success("ok".to_string()))
        .map_err(|e| format!("{}", e))
}

#[tauri::command]
pub async fn git_pull(
    state: State<'_, AppState>,
    request: GitPullRequest,
    git_service: State<'_, GitService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    git_service.pull(&request.workspace_path, &request.remote, &request.branch)
        .map(|_| ApiResponse::success("ok".to_string()))
        .map_err(|e| format!("{}", e))
}

#[tauri::command]
pub async fn git_branches(
    state: State<'_, AppState>,
    request: WorkspacePathRequest,
    git_service: State<'_, GitService>,
) -> Result<ApiResponse<GitBranchesResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    git_service.get_branches(&request.workspace_path)
        .map(ApiResponse::success)
        .map_err(|e| format!("{}", e))
}

#[tauri::command]
pub async fn git_checkout(
    state: State<'_, AppState>,
    request: GitCheckoutRequest,
    git_service: State<'_, GitService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    git_service.checkout_branch(&request.workspace_path, &request.branch)
        .map(|_| ApiResponse::success("ok".to_string()))
        .map_err(|e| format!("{}", e))
}

#[tauri::command]
pub async fn git_log(
    state: State<'_, AppState>,
    request: GitLogRequest,
    git_service: State<'_, GitService>,
) -> Result<ApiResponse<Vec<GitCommit>>, String> {
    crate::commands::common::require_auth(&state).await?;
    git_service.get_log(&request.workspace_path, request.count)
        .map(ApiResponse::success)
        .map_err(|e| format!("{}", e))
}

#[tauri::command]
pub async fn git_init(
    state: State<'_, AppState>,
    request: WorkspacePathRequest,
    git_service: State<'_, GitService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    git_service.init_repo(&request.workspace_path)
        .map(|_| ApiResponse::success("ok".to_string()))
        .map_err(|e| format!("{}", e))
}