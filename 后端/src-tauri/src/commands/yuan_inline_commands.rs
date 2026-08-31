use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::editor::{
    InlineCompletionRequest, InlineCompletionResult, InlineEditRequest, InlineEditResponse,
};
use crate::services::inline_service::InlineService;
use crate::services::yuancode_service;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

async fn get_user_id(state: &State<'_, AppState>) -> Result<i64, String> {
    let user = state.current_user.read().await;
    user.ok_or_else(|| String::from("未登录"))
}

/// 请求 AI 行内补全
#[tauri::command]
pub async fn yuan_inline_complete(
    state: State<'_, AppState>,
    request: InlineCompletionRequest,
    service: State<'_, InlineService>,
) -> Result<ApiResponse<InlineCompletionResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    // D1.1：从 AppState 取 user_id 传给 InlineService（用于解密 API Key）
    let user_id = get_user_id(&state).await.unwrap_or(0);
    service
        .complete(request, user_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 检查当前位置是否有可用的补全
#[tauri::command]
pub async fn yuan_inline_available(
    state: State<'_, AppState>,
    file_path: String,
    language: String,
    service: State<'_, InlineService>,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    let available = service.is_available(&file_path, &language).await;
    Ok(ApiResponse::success(available))
}

/// AI 内联编辑：根据用户指令修改选中的代码
#[tauri::command]
pub async fn yuan_inline_edit(
    state: State<'_, AppState>,
    request: InlineEditRequest,
) -> Result<ApiResponse<InlineEditResponse>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;

    let modified_code = yuancode_service::ai_inline_edit(
        &state.pool,
        &state.mek_manager,
        user_id,
        &request.selected_code,
        &request.instruction,
        &request.language,
        &request.file_path,
        request.model_id,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(ApiResponse::success(InlineEditResponse { modified_code }))
}