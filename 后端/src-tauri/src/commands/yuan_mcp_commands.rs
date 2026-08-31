use tauri::State;

use crate::db::connection::AppState;
use crate::db::repositories::mcp_repo;
use crate::models::api_response::ApiResponse;
use crate::models::mcp::{
    McpHealthCheckResult, McpLifecycleConfig, McpRegisteredTool,
    McpServerRegistration, McpServerStatus,
};
use crate::services::mcp_service::McpService;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn yuan_mcp_register_server(
    state: State<'_, AppState>,
    server: McpServerRegistration,
    auto_connect: bool,
    service: State<'_, McpService>,
) -> Result<ApiResponse<McpServerStatus>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .register_server(server, auto_connect)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

// ===== 3.1 生命周期 =====

#[tauri::command]
pub async fn yuan_mcp_spawn(
    state: State<'_, AppState>,
    server_id: String,
    service: State<'_, McpService>,
) -> Result<ApiResponse<McpServerStatus>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .spawn_server(&server_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_connect_server(
    state: State<'_, AppState>,
    server_id: String,
    service: State<'_, McpService>,
) -> Result<ApiResponse<McpServerStatus>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .connect_server(&server_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_disconnect_server(
    state: State<'_, AppState>,
    server_id: String,
    service: State<'_, McpService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .disconnect_server(&server_id)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_health_check(
    state: State<'_, AppState>,
    server_id: String,
    service: State<'_, McpService>,
) -> Result<ApiResponse<McpHealthCheckResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .health_check(&server_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_health_check_all(
    state: State<'_, AppState>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<Vec<McpHealthCheckResult>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .health_check_all()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_set_lifecycle_config(
    state: State<'_, AppState>,
    server_id: String,
    config: McpLifecycleConfig,
    service: State<'_, McpService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .update_lifecycle_config(&server_id, config)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_get_lifecycle_state(
    state: State<'_, AppState>,
    server_id: String,
    service: State<'_, McpService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .get_lifecycle_state(&server_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

// ===== 工具管理 =====

#[tauri::command]
pub async fn yuan_mcp_list_servers(
    state: State<'_, AppState>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<Vec<McpServerStatus>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_servers()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_list_all_tools(
    state: State<'_, AppState>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<Vec<McpRegisteredTool>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_all_tools()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_server_status(
    state: State<'_, AppState>,
    server_id: String,
    service: State<'_, McpService>,
) -> Result<ApiResponse<McpServerStatus>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .server_status(&server_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_call_tool(
    state: State<'_, AppState>,
    server_id: String,
    tool_name: String,
    arguments: Option<serde_json::Value>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<crate::models::mcp::ToolCallResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .call_tool(&server_id, &tool_name, arguments)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

// ===== 3.2 工具 Schema 塑形 =====

#[tauri::command]
pub async fn yuan_mcp_list_shaped_tools(
    state: State<'_, AppState>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<Vec<McpRegisteredTool>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_shaped_tools()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_group_tools_by_namespace(
    state: State<'_, AppState>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<std::collections::HashMap<String, Vec<McpRegisteredTool>>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .group_tools_by_namespace()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_list_tools_by_namespace(
    state: State<'_, AppState>,
    namespace: String,
    service: State<'_, McpService>,
) -> Result<ApiResponse<Vec<McpRegisteredTool>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_tools_by_namespace(&namespace)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_get_tool_namespaces(
    state: State<'_, AppState>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<Vec<String>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .get_tool_namespaces()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_set_shaper_config(
    state: State<'_, AppState>,
    preferred_servers: Option<Vec<String>>,
    enable_namespace: Option<bool>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .update_shaper_config(preferred_servers, enable_namespace)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

// ===== 3.3 工具过滤与搜索 =====

#[tauri::command]
pub async fn yuan_mcp_search_tools(
    state: State<'_, AppState>,
    query: String,
    service: State<'_, McpService>,
) -> Result<ApiResponse<Vec<McpRegisteredTool>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .search_tools(&query)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_set_tool_enabled(
    state: State<'_, AppState>,
    tool_name: String,
    enabled: bool,
    service: State<'_, McpService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .set_tool_enabled(&tool_name, enabled)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_set_enabled_tools(
    state: State<'_, AppState>,
    tools: Vec<String>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .set_enabled_tools(tools)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_set_disabled_tools(
    state: State<'_, AppState>,
    tools: Vec<String>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .set_disabled_tools(tools)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_list_filtered_tools(
    state: State<'_, AppState>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<Vec<McpRegisteredTool>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_filtered_tools()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

// ===== 3.4 MCP 资源支持 =====

#[tauri::command]
pub async fn yuan_mcp_list_resources(
    state: State<'_, AppState>,
    server_id: String,
    service: State<'_, McpService>,
) -> Result<ApiResponse<Vec<crate::models::mcp::Resource>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_resources(&server_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_read_resource(
    state: State<'_, AppState>,
    server_id: String,
    uri: String,
    service: State<'_, McpService>,
) -> Result<ApiResponse<crate::models::mcp::ReadResourceResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .read_resource(&server_id, &uri)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_list_prompts(
    state: State<'_, AppState>,
    server_id: String,
    service: State<'_, McpService>,
) -> Result<ApiResponse<Vec<crate::models::mcp::Prompt>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_prompts(&server_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_get_prompt(
    state: State<'_, AppState>,
    server_id: String,
    name: String,
    arguments: Option<serde_json::Value>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<crate::models::mcp::GetPromptResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .get_prompt(&server_id, &name, arguments)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

// ===== 3.5 延迟工具加载 =====

#[tauri::command]
pub async fn yuan_mcp_set_deferred_namespaces(
    state: State<'_, AppState>,
    namespaces: Vec<String>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .set_deferred_namespaces(namespaces)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_get_deferred_namespaces(
    state: State<'_, AppState>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<Vec<String>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .get_deferred_namespaces()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_load_namespace(
    state: State<'_, AppState>,
    namespace: String,
    service: State<'_, McpService>,
) -> Result<ApiResponse<McpServerStatus>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .load_namespace(&namespace)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_mcp_unload_namespace(
    state: State<'_, AppState>,
    namespace: String,
    service: State<'_, McpService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .unload_namespace(&namespace)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

// ===== D1.6 持久化：MCP 服务器配置存储 + 启动恢复 =====

/// 列出所有持久化的 MCP 服务器配置（含禁用的，用于前端管理界面展示）
#[tauri::command]
pub async fn yuan_mcp_list_persisted_servers(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<mcp_repo::McpServerRecord>>, String> {
    // 多用户隔离（批次 6）：按当前登录用户 user_id 过滤
    let user_id = crate::commands::common::require_auth(&state).await?;
    mcp_repo::load_all_servers(&state.pool, user_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 保存/更新 MCP 服务器配置（持久化 + 注册到运行时内存）
#[tauri::command]
pub async fn yuan_mcp_save_server_config(
    server: McpServerRegistration,
    auto_connect: bool,
    state: State<'_, AppState>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<()>, String> {
    // 多用户隔离（批次 6）：配置归属当前登录用户
    let user_id = crate::commands::common::require_auth(&state).await?;
    // 1. 持久化到数据库
    mcp_repo::upsert_server(&state.pool, user_id, &server, auto_connect)
        .await
        .map_err(|e| e.to_string())?;
    // 2. 注册到运行时内存（仅当启用时）
    if server.enabled {
        let _ = service.register_server(server, auto_connect).await;
    }
    Ok(ApiResponse::success(()))
}

/// 删除 MCP 服务器配置（预置模板不可删除）
#[tauri::command]
pub async fn yuan_mcp_delete_server_config(
    server_id: String,
    state: State<'_, AppState>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<()>, String> {
    // 多用户隔离（批次 6）：仅允许删除当前用户自己的配置
    let user_id = crate::commands::common::require_auth(&state).await?;
    // 先从运行时断开（忽略错误，可能未连接）
    let _ = service.disconnect_server(&server_id).await;
    // 再从数据库删除（预置模板不可删）
    mcp_repo::delete_server(&state.pool, user_id, &server_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(()))
}

/// 启用/禁用 MCP 服务器
#[tauri::command]
pub async fn yuan_mcp_set_server_enabled(
    server_id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<ApiResponse<()>, String> {
    // 多用户隔离（批次 6）：仅允许修改当前用户自己的配置
    let user_id = crate::commands::common::require_auth(&state).await?;
    mcp_repo::set_enabled(&state.pool, user_id, &server_id, enabled)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(()))
}

/// 从数据库加载已启用的 MCP 服务器到运行时内存（应用启动时自动调用，也可手动触发）
#[tauri::command]
pub async fn yuan_mcp_load_persisted_servers(
    state: State<'_, AppState>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<usize>, String> {
    // 多用户隔离（批次 6）：仅加载当前用户自己的已启用配置
    let user_id = crate::commands::common::require_auth(&state).await?;
    let servers = mcp_repo::load_enabled_servers(&state.pool, user_id)
        .await
        .map_err(|e| e.to_string())?;
    let mut loaded = 0;
    for server in servers {
        if service.register_server(server, true).await.is_ok() {
            loaded += 1;
        }
    }
    tracing::info!("🔌 [D1.6] 从数据库恢复 {} 个 MCP 服务器", loaded);
    Ok(ApiResponse::success(loaded))
}

// ===== D1 v3.1 Task 3.2.1-3.2.9: 内置 MCP 服务器管理 =====

/// 列出内置 MCP 服务器名称（8 个：github/gitlab/sqlite/postgres/slack/jira/memory/websearch）
#[tauri::command]
pub async fn yuan_mcp_list_builtin_servers(
    state: State<'_, AppState>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<Vec<String>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_builtin_servers()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 列出内置 MCP 服务器的工具定义
#[tauri::command]
pub async fn yuan_mcp_list_builtin_tools(
    state: State<'_, AppState>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<Vec<McpRegisteredTool>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_builtin_tools()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 调用内置 MCP 服务器工具
#[tauri::command]
pub async fn yuan_mcp_call_builtin_tool(
    state: State<'_, AppState>,
    server_name: String,
    tool_name: String,
    arguments: Option<serde_json::Value>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<crate::models::mcp::ToolCallResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .call_builtin_tool(&server_name, &tool_name, arguments)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 配置内置 MCP 服务器（注入 API Key / 连接字符串等，仅内存驻留）
#[tauri::command]
pub async fn yuan_mcp_configure_builtin(
    state: State<'_, AppState>,
    server_name: String,
    config: serde_json::Value,
    service: State<'_, McpService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .configure_builtin(&server_name, config)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

// ===== D1 v3.1 Task 3.2.11: 工具调用历史 =====

/// 获取工具调用历史（最近 100 条，最新在前）
#[tauri::command]
pub async fn yuan_mcp_list_call_history(
    state: State<'_, AppState>,
    service: State<'_, McpService>,
) -> Result<ApiResponse<Vec<crate::mcp::McpToolCallHistoryEntry>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_call_history()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}