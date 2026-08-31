use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::io_control::{
    ExecuteWithStreamRequest, IoControlStats, IoKillRequest, IoStdinRequest, OutputConfig,
    StreamedOutput,
};
use crate::services::io_control_service::IoControlService;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn yuan_io_config_get(
    state: State<'_, AppState>,
    service: State<'_, IoControlService>,
) -> Result<ApiResponse<OutputConfig>, String> {
    crate::commands::common::require_auth(&state).await?;
    let config = service.get_config().await;
    Ok(ApiResponse::success(config))
}

#[tauri::command]
pub async fn yuan_io_config_update(
    state: State<'_, AppState>,
    config: OutputConfig,
    service: State<'_, IoControlService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service.update_config(config).await;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn yuan_io_stats(
    state: State<'_, AppState>,
    service: State<'_, IoControlService>,
) -> Result<ApiResponse<IoControlStats>, String> {
    crate::commands::common::require_auth(&state).await?;
    let stats = service.get_stats().await;
    Ok(ApiResponse::success(stats))
}

#[tauri::command]
pub async fn yuan_io_execute(
    state: State<'_, AppState>,
    request: ExecuteWithStreamRequest,
    service: State<'_, IoControlService>,
) -> Result<ApiResponse<StreamedOutput>, String> {
    crate::commands::common::require_auth(&state).await?;
    let (result, mut rx) = service
        .execute_streamed(request)
        .await
        .map_err(|e| e.to_string())?;

    let mut deltas = Vec::new();
    while let Some(delta) = rx.recv().await {
        deltas.push(delta);
    }

    let mut output = result;
    output.deltas = deltas
        .into_iter()
        .map(|d| d.delta)
        .collect();

    Ok(ApiResponse::success(output))
}

#[tauri::command]
pub async fn yuan_io_cancel(
    state: State<'_, AppState>,
    execution_id: String,
    service: State<'_, IoControlService>,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    let cancelled = service.cancel_execution(&execution_id).await;
    Ok(ApiResponse::success(cancelled))
}

#[tauri::command]
pub async fn yuan_io_cancel_all(
    state: State<'_, AppState>,
    service: State<'_, IoControlService>,
) -> Result<ApiResponse<usize>, String> {
    crate::commands::common::require_auth(&state).await?;
    let count = service.cancel_all().await;
    Ok(ApiResponse::success(count))
}

#[tauri::command]
pub async fn yuan_io_kill(
    state: State<'_, AppState>,
    request: IoKillRequest,
    service: State<'_, IoControlService>,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    let killed = service.kill_execution(&request.execution_id).await;
    Ok(ApiResponse::success(killed))
}

#[tauri::command]
pub async fn yuan_io_stdin(
    state: State<'_, AppState>,
    request: IoStdinRequest,
    service: State<'_, IoControlService>,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    let sent = service.send_stdin(&request.execution_id, &request.data).await;
    Ok(ApiResponse::success(sent))
}