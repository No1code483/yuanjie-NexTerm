use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn extension_get_entry(
    state: State<'_, AppState>,
    module: String,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::error(
        2002,
        &format!("模块 [{}] 功能待完善，期待后续迭代", module),
    ))
}

#[tauri::command]
pub async fn extension_list_modules(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<String>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(vec![
        "xinxin".to_string(),
        "game".to_string(),
        "xincode".to_string(),
        "linux".to_string(),
    ]))
}
