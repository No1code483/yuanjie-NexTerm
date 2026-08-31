// 工具系统 Tauri 命令 — 前端调用的工具管理接口

use tauri::State;
use crate::tools::{ToolRegistry, ApplyPatchTool, PlanTool};
use crate::models::tool::{
    ToolCallRequest, ToolInfo, ToolSearchRequest, ToolSearchResponse,
    PatchRequest, PatchResult, Plan, PlanStep, PlanStepStatus,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::connection::AppState;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

/// 工具系统应用状态
pub struct ToolState {
    pub registry: Arc<Mutex<ToolRegistry>>,
    pub plan_tool: Arc<Mutex<PlanTool>>,
    pub patch_tool: Arc<Mutex<ApplyPatchTool>>,
}

// ============================================================
// 工具列表 & 搜索
// ============================================================

/// 列出所有可用工具
#[tauri::command]
pub async fn yuan_tools_list(
    state: State<'_, AppState>,
    tool_state: State<'_, ToolState>,
) -> Result<Vec<ToolInfo>, String> {
    crate::commands::common::require_auth(&state).await?;
    let registry: tokio::sync::MutexGuard<'_, ToolRegistry> = tool_state.registry.lock().await;
    Ok(registry.list_all())
}

/// 搜索工具
#[tauri::command]
pub async fn yuan_tools_search(
    state: State<'_, AppState>,
    tool_state: State<'_, ToolState>,
    request: ToolSearchRequest,
) -> Result<ToolSearchResponse, String> {
    crate::commands::common::require_auth(&state).await?;
    let registry: tokio::sync::MutexGuard<'_, ToolRegistry> = tool_state.registry.lock().await;
    let definitions = registry.list_definitions();
    let search_tool = crate::tools::tool_search::ToolSearchTool::new();
    search_tool.index_tools(definitions).await;
    Ok(search_tool.search(request).await)
}

/// 调用工具
#[tauri::command]
pub async fn yuan_tools_call(
    state: State<'_, AppState>,
    tool_state: State<'_, ToolState>,
    request: ToolCallRequest,
) -> Result<serde_json::Value, String> {
    crate::commands::common::require_auth(&state).await?;
    let registry: tokio::sync::MutexGuard<'_, ToolRegistry> = tool_state.registry.lock().await;
    let result = registry.call(request).await.map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(&result).unwrap_or_default())
}

// ============================================================
// Plan 工具
// ============================================================

/// 创建计划
#[tauri::command]
pub async fn yuan_plan_create(
    state: State<'_, AppState>,
    tool_state: State<'_, ToolState>,
    title: String,
    description: String,
    steps: Vec<PlanStep>,
    session_id: String,
) -> Result<Plan, String> {
    crate::commands::common::require_auth(&state).await?;
    let plan_tool: tokio::sync::MutexGuard<'_, PlanTool> = tool_state.plan_tool.lock().await;
    plan_tool
        .create_plan(&title, &description, steps, &session_id)
        .await
        .map_err(|e: crate::models::tool::ToolError| e.message)
}

/// 更新计划步骤
#[tauri::command]
pub async fn yuan_plan_update_step(
    state: State<'_, AppState>,
    tool_state: State<'_, ToolState>,
    plan_id: String,
    step_id: String,
    status: String,
    result: Option<String>,
) -> Result<Plan, String> {
    crate::commands::common::require_auth(&state).await?;
    let plan_tool: tokio::sync::MutexGuard<'_, PlanTool> = tool_state.plan_tool.lock().await;
    let plan_status = match status.as_str() {
        "pending" => PlanStepStatus::Pending,
        "in_progress" => PlanStepStatus::InProgress,
        "completed" => PlanStepStatus::Completed,
        "failed" => PlanStepStatus::Failed,
        "skipped" => PlanStepStatus::Skipped,
        _ => return Err(format!("无效的状态值: {}", status)),
    };
    plan_tool
        .update_step(&plan_id, &step_id, plan_status, result)
        .await
        .map_err(|e: crate::models::tool::ToolError| e.message)
}

/// 获取计划详情
#[tauri::command]
pub async fn yuan_plan_get(
    state: State<'_, AppState>,
    tool_state: State<'_, ToolState>,
    plan_id: String,
) -> Result<Plan, String> {
    crate::commands::common::require_auth(&state).await?;
    let plan_tool: tokio::sync::MutexGuard<'_, PlanTool> = tool_state.plan_tool.lock().await;
    plan_tool
        .get_plan(&plan_id)
        .await
        .ok_or_else(|| format!("计划不存在: {}", plan_id))
}

/// 列出所有计划
#[tauri::command]
pub async fn yuan_plan_list_all(
    state: State<'_, AppState>,
    tool_state: State<'_, ToolState>,
) -> Result<Vec<Plan>, String> {
    crate::commands::common::require_auth(&state).await?;
    let plan_tool: tokio::sync::MutexGuard<'_, PlanTool> = tool_state.plan_tool.lock().await;
    Ok(plan_tool.list_plans(None).await)
}

// ============================================================
// Apply Patch 工具
// ============================================================

/// 应用补丁
#[tauri::command]
pub async fn yuan_apply_patch(
    state: State<'_, AppState>,
    tool_state: State<'_, ToolState>,
    request: PatchRequest,
) -> Result<PatchResult, String> {
    crate::commands::common::require_auth(&state).await?;
    let patch_tool: tokio::sync::MutexGuard<'_, ApplyPatchTool> = tool_state.patch_tool.lock().await;
    patch_tool.apply_patch(request).map_err(|e: crate::models::tool::ToolError| e.message)
}