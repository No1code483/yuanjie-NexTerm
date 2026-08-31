use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::sandbox::{
    ExecuteRequest, FileOperationRequest, SandboxConfig, SandboxHistoryRecord, SandboxInfo, SandboxSaveRequest,
    SandboxSnapshot,
};
use crate::services::sandbox_service::SandboxService;

// 安全审计修复（发现 4，HIGH）：
// 1. 原 yuan_sandbox_* 命令均无认证，任何能注入 IPC 调用的代码均可执行沙箱命令、
//    读写任意文件。现每个命令入口强制调用 `require_auth(&state).await?`。
// 2. 原实现信任前端传入的 `permission_profile`，攻击者可将 profile 设为
//    `full_access`（`allowed_commands: ["*"]`、`read_paths: ["/"]`、
//    `write_paths: ["/"]`、`network_policy: AllowAll`）绕过沙箱限制。
//    现服务端强制将 `full_access` 降级为 `read_write`，永不信任前端传入的
//    `full_access`/`permissive`。

/// 判断并降级危险的 permission_profile。
///
/// 若 profile.name == "full_access" 或 allowed_commands 含 "*"，强制降级为 read_write。
/// 这是最小权限原则的硬性约束，防止前端/AI 注入 full_access 配置。
fn enforce_safe_permission(mut config: SandboxConfig) -> SandboxConfig {
    let needs_downgrade = config.permission_profile.name == "full_access"
        || config
            .permission_profile
            .allowed_commands
            .iter()
            .any(|c| c == "*");

    if needs_downgrade {
        tracing::warn!(
            "[sandbox] 检测到 full_access 权限请求，已强制降级为 read_write（不允许前端/AI 请求 full_access）"
        );
        config.permission_profile = crate::models::sandbox::PermissionProfile::read_write();
    }

    config
}

#[tauri::command]
pub async fn yuan_sandbox_create(
    state: State<'_, AppState>,
    config: SandboxConfig,
    service: State<'_, SandboxService>,
) -> Result<ApiResponse<SandboxInfo>, String> {
    crate::commands::common::require_auth(&state).await?;
    // 服务端强制覆写：拒绝 full_access，降级为 read_write
    let safe_config = enforce_safe_permission(config);
    service
        .create_sandbox(safe_config)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_sandbox_list(
    state: State<'_, AppState>,
    service: State<'_, SandboxService>,
) -> Result<ApiResponse<Vec<SandboxInfo>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_sandboxes()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_sandbox_list_by_agent(
    state: State<'_, AppState>,
    agent_id: String,
    service: State<'_, SandboxService>,
) -> Result<ApiResponse<Vec<SandboxInfo>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_sandboxes_by_agent(&agent_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_sandbox_get(
    state: State<'_, AppState>,
    sandbox_id: String,
    service: State<'_, SandboxService>,
) -> Result<ApiResponse<SandboxSnapshot>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .get_sandbox(&sandbox_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_sandbox_execute(
    state: State<'_, AppState>,
    sandbox_id: String,
    request: ExecuteRequest,
    service: State<'_, SandboxService>,
) -> Result<ApiResponse<crate::models::sandbox::ExecuteResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut req = request;
    req.sandbox_id = sandbox_id;
    service
        .execute(&req.sandbox_id.clone(), req)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_sandbox_read_file(
    state: State<'_, AppState>,
    sandbox_id: String,
    request: FileOperationRequest,
    service: State<'_, SandboxService>,
) -> Result<ApiResponse<crate::models::sandbox::FileOperationResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut req = request;
    req.sandbox_id = sandbox_id;
    service
        .read_file(&req.sandbox_id.clone(), req)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_sandbox_write_file(
    state: State<'_, AppState>,
    sandbox_id: String,
    request: FileOperationRequest,
    service: State<'_, SandboxService>,
) -> Result<ApiResponse<crate::models::sandbox::FileOperationResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut req = request;
    req.sandbox_id = sandbox_id;
    service
        .write_file(&req.sandbox_id.clone(), req)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_sandbox_delete_file(
    state: State<'_, AppState>,
    sandbox_id: String,
    path: String,
    service: State<'_, SandboxService>,
) -> Result<ApiResponse<crate::models::sandbox::FileOperationResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .delete_file(&sandbox_id, &path)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_sandbox_list_files(
    state: State<'_, AppState>,
    sandbox_id: String,
    path: String,
    service: State<'_, SandboxService>,
) -> Result<ApiResponse<Vec<String>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_files(&sandbox_id, &path)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_sandbox_terminate(
    state: State<'_, AppState>,
    sandbox_id: String,
    service: State<'_, SandboxService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .terminate_sandbox(&sandbox_id)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_sandbox_cleanup(
    state: State<'_, AppState>,
    service: State<'_, SandboxService>,
) -> Result<ApiResponse<usize>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .cleanup_terminated()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_sandbox_count(
    state: State<'_, AppState>,
    service: State<'_, SandboxService>,
) -> Result<ApiResponse<usize>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .get_count()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_sandbox_save(
    state: State<'_, AppState>,
    request: SandboxSaveRequest,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    // 服务端强制覆写：拒绝 "permissive"（对应 full_access），降级为 "normal"（read_write）
    let safe_level = match request.safety_level.as_str() {
        "strict" => "strict",
        "permissive" => {
            tracing::warn!(
                "[sandbox] yuan_sandbox_save: 检测到 permissive 请求，已强制降级为 normal（read_write）"
            );
            "normal"
        }
        _ => "normal",
    };
    let _config = SandboxConfig {
        sandbox_type: crate::models::sandbox::SandboxType::FileSystem,
        permission_profile: match safe_level {
            "strict" => crate::models::sandbox::PermissionProfile::read_only(),
            "normal" => crate::models::sandbox::PermissionProfile::read_write(),
            // safe_level 已被 match 限定为 strict/normal，此处不可达
            _ => crate::models::sandbox::PermissionProfile::read_write(),
        },
        timeout_ms: 30000,
        agent_id: None,
        working_dir: ".".into(),
    };
    tracing::info!("Sandbox config saved: safety_level={} (original={}, downgraded={}), perms={:?}", safe_level, request.safety_level, safe_level != request.safety_level, request.permissions);
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn yuan_sandbox_history(
    state: State<'_, AppState>,
    service: State<'_, SandboxService>,
) -> Result<ApiResponse<Vec<SandboxHistoryRecord>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .get_history()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}
