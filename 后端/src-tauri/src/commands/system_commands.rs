use tauri::State;
use tauri::Manager;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::system::SystemConfig;
use crate::services::system_service;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn get_system_config(
    state: State<'_, AppState>,
    key: String,
) -> Result<ApiResponse<Option<SystemConfig>>, String> {
    crate::commands::common::require_auth(&state).await?;
    match system_service::get_config(&state.pool, &key).await {
        Ok(config) => Ok(ApiResponse::success(config)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn set_system_config(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<ApiResponse<SystemConfig>, String> {
    crate::commands::common::require_auth(&state).await?;
    match system_service::set_config(&state.pool, &key, &value).await {
        Ok(config) => Ok(ApiResponse::success(config)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn get_all_system_configs(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<SystemConfig>>, String> {
    crate::commands::common::require_auth(&state).await?;
    match system_service::get_all_configs(&state.pool).await {
        Ok(configs) => Ok(ApiResponse::success(configs)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn system_open_file(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match system_service::open_file(&path) {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn system_open_url(
    state: State<'_, AppState>,
    url: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match system_service::open_url(&url) {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn system_get_app_info(state: State<'_, AppState>) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let info = serde_json::json!({
        "name": "NexTerm·元界",
        "version": "1.0.0",
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    });
    Ok(ApiResponse::success(info))
}

#[tauri::command]
pub async fn clipboard_write_text(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    text: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let clipboard = app_handle.clipboard();
    clipboard.write_text(text).map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn clipboard_read_text(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let clipboard = app_handle.clipboard();
    let text = clipboard.read_text().map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(text))
}

/// C3.2：设置窗口置顶（always-on-top）
/// 前端调用：invoke('window_set_always_on_top', { alwaysOnTop: true })
#[tauri::command]
pub async fn window_set_always_on_top(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    always_on_top: bool,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    if let Some(window) = app_handle.get_webview_window("main") {
        window
            .set_always_on_top(always_on_top)
            .map_err(|e| e.to_string())?;
    }
    Ok(ApiResponse::success(()))
}

/// C3.2：截取当前窗口截图
/// 前端调用：invoke('window_screenshot')
/// 返回截图保存的文件路径
///
/// 实现说明：Tauri 2.11.1 的 WebviewWindow::capture() 在不稳定 API 中，
/// 当前版本编译不可用。此命令通过 webview eval 触发前端 canvas 截图，
/// 由前端事件回调接收 Base64 图片数据后保存。
/// 前端 handleScreenshot 调用此命令时会收到提示，实际截图走前端 Canvas API。
#[tauri::command]
pub async fn window_screenshot(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let window = app_handle
        .get_webview_window("main")
        .ok_or("主窗口未找到")?;

    // 通过 webview eval 触发前端截图（前端监听 'nexterm:screenshot' 事件）
    let _ = window.eval(
        "window.dispatchEvent(new CustomEvent('nexterm:screenshot'));"
    );

    Ok(ApiResponse::success("screenshot_triggered".to_string()))
}