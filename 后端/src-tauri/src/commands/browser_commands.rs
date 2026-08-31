use crate::models::api_response::ApiResponse;
use tauri::{LogicalPosition, LogicalSize, State, WebviewBuilder, WebviewUrl};

use crate::db::connection::AppState;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

fn find_child_webview(
    window: &tauri::Window,
    label: &str,
) -> Result<tauri::Webview, String> {
    window
        .webviews()
        .into_iter()
        .find(|wv| wv.label() == label)
        .ok_or_else(|| format!("WebView「{}」未找到", label))
}

#[tauri::command]
pub async fn browser_open_window(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    url: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    let label = format!("browser-wv-{}", uuid::Uuid::new_v4());

    tauri::WebviewWindowBuilder::new(
        &app_handle,
        &label,
        WebviewUrl::External(
            url.parse()
                .map_err(|e| format!("URL 格式无效: {}", e))?,
        ),
    )
    .title("NexTerm 浏览器")
    .inner_size(1200.0, 800.0)
    .center()
    .resizable(true)
    .minimizable(true)
    .maximizable(true)
    .build()
    .map_err(|e| format!("无法创建浏览器窗口: {}", e))?;

    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn browser_create_view(
    state: State<'_, AppState>,
    window: tauri::Window,
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let label = format!("browser-child-{}", uuid::Uuid::new_v4());

    window
        .add_child(
            WebviewBuilder::new(
                &label,
                WebviewUrl::External(
                    url.parse()
                        .map_err(|e| format!("URL 格式无效: {}", e))?,
                ),
            )
            .focused(true)
            .on_navigation(|_url| true)
            .initialization_script("Object.defineProperty(window,'open',{value:function(url){if(url&&typeof url==='string'){window.location.href=url}return null},writable:false,configurable:false});document.addEventListener('click',function(e){var a=e.target.closest('a');if(a&&a.href&&a.href.startsWith('http')){var t=a.getAttribute('target');if(t==='_blank'||t==='_new'){e.preventDefault();e.stopPropagation();window.location.href=a.href}}},true);"),
            LogicalPosition::new(x, y),
            LogicalSize::new(width, height),
        )
        .map_err(|e| format!("创建子 WebView 失败: {}", e))?;

    Ok(ApiResponse::success(label))
}

#[tauri::command]
pub async fn browser_navigate_view(
    state: State<'_, AppState>,
    window: tauri::Window,
    label: String,
    url: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    let webview = find_child_webview(&window, &label)?;

    webview
        .navigate(
            url.parse()
                .map_err(|e| format!("URL 格式无效: {}", e))?,
        )
        .map_err(|e| format!("导航失败: {}", e))?;

    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn browser_resize_view(
    state: State<'_, AppState>,
    window: tauri::Window,
    label: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    let webview = find_child_webview(&window, &label)?;

    webview
        .set_position(LogicalPosition::new(x, y))
        .map_err(|e| format!("设置位置失败: {}", e))?;
    webview
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| format!("设置大小失败: {}", e))?;

    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn browser_close_view(
    state: State<'_, AppState>,
    window: tauri::Window,
    label: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    let webview = find_child_webview(&window, &label)?;

    webview
        .close()
        .map_err(|e| format!("关闭失败: {}", e))?;

    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn browser_cleanup(
    state: State<'_, AppState>,
    window: tauri::Window,
) -> Result<ApiResponse<u32>, String> {
    crate::commands::common::require_auth(&state).await?;
    let child_labels: Vec<String> = window
        .webviews()
        .into_iter()
        .filter(|wv| wv.label().starts_with("browser-child-"))
        .map(|wv| wv.label().to_string())
        .collect();

    let mut closed = 0u32;
    for label in &child_labels {
        if let Some(wv) = window.webviews().into_iter().find(|w| w.label() == label) {
            if wv.close().is_ok() {
                closed += 1;
            }
        }
    }

    Ok(ApiResponse::success(closed))
}