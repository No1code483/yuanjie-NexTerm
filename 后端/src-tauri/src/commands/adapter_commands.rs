use tauri::State;

use crate::db::connection::AppState;
use crate::models::adapter::AdapterInfo;
use crate::models::api_response::ApiResponse;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn adapter_list(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<AdapterInfo>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(vec![]))
}

#[tauri::command]
pub async fn adapter_install(
    state: State<'_, AppState>,
    _path: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success_msg("扩展安装功能待后续迭代开放"))
}

#[tauri::command]
pub async fn adapter_uninstall(
    state: State<'_, AppState>,
    _id: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success_msg("扩展卸载功能待后续迭代开放"))
}
