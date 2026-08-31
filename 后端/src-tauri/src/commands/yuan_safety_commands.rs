use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::safety::{SafetyCheckRequest, SafetyCheckResult, SafetyProfile};
use crate::services::safety_service::SafetyService;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn yuan_safety_check(
    state: State<'_, AppState>,
    request: SafetyCheckRequest,
    service: State<'_, SafetyService>,
) -> Result<ApiResponse<SafetyCheckResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .check(request)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_safety_quick_check(
    state: State<'_, AppState>,
    content: String,
    service: State<'_, SafetyService>,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .quick_check(&content)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_safety_set_profile(
    state: State<'_, AppState>,
    profile: SafetyProfile,
    service: State<'_, SafetyService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .set_profile(profile)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_safety_get_profile(
    state: State<'_, AppState>,
    service: State<'_, SafetyService>,
) -> Result<ApiResponse<SafetyProfile>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .get_profile()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_safety_list_profiles(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<SafetyProfile>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let profiles = vec![
        SafetyProfile::default(),
        SafetyProfile::strict(),
        SafetyProfile::permissive(),
        SafetyProfile::disabled(),
    ];
    Ok(ApiResponse::success(profiles))
}