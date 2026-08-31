//! D1 v3.1+v3.2 自主执行 + 多文件原子编辑 IPC 命令
//!
//! 设计依据：
//!   - 功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md
//!   - .trae/rules/项目核心设计意图.md §三（Yuan Code 走云端 API）
//!
//! 命令清单：
//!   - yuan_agent_execute_autonomous  ReAct 自主执行循环
//!   - yuan_multi_file_edit            多文件原子编辑（事务 + 回滚）
//!   - yuan_multi_file_preview_diffs  跨文件 diff 预览（D1.7，不写入磁盘）

use std::sync::Arc;

use tauri::State;
use tokio::sync::RwLock;

use crate::crypto::mek_manager::MekManager;
use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::services::agent_executor_service::{
    execute_autonomous, AutonomousExecuteRequest, AutonomousExecuteResponse,
};
use crate::services::multi_file_edit_service::{
    apply_atomic_edits, preview_diffs, MultiFileDiffPreview, MultiFileEditRequest,
    MultiFileEditResult,
};

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

/// 自主执行 Agent 任务（ReAct 模式）
///
/// 调用云端 API 模型进行多轮推理 + 工具调用循环。
/// 前端通过监听 `agent-step` 事件实时获取每轮思考/动作。
#[tauri::command]
pub async fn yuan_agent_execute_autonomous(
    request: AutonomousExecuteRequest,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<ApiResponse<AutonomousExecuteResponse>, String> {
    crate::commands::common::require_auth(&state).await?;
    // A5 Phase 3 Task 2: 云端 AI 离线守卫
    //
    // 设计依据：.trae/rules/项目核心设计意图.md §三（Yuan Code 走云端 API，不切换本地 ollama）
    // 离线时返回 Offline 错误（前端提示「AI 离线，本地编辑可用」），不进入 ReAct 循环。
    // 本地 Monaco 编辑、文件读写等非 AI 功能不受影响。
    crate::services::ai_model_service::AiModelService::check_online_before_call(&state)
        .await
        .map_err(|e| e.to_string())?;

    let pool = state.pool.clone();
    let mek_manager: Arc<RwLock<MekManager>> = state.mek_manager.clone();
    execute_autonomous(&pool, &mek_manager, &app_handle, request)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 多文件原子编辑
///
/// 接收一组编辑操作（write/create/delete/rename），按顺序执行；
/// 任一失败时若 auto_rollback=true，自动回滚已应用的编辑。
#[tauri::command]
pub async fn yuan_multi_file_edit(
    state: State<'_, AppState>,
    request: MultiFileEditRequest,
) -> Result<ApiResponse<MultiFileEditResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    apply_atomic_edits(request)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 跨文件 diff 预览（D1.7）
///
/// 接收一组编辑操作，生成每个文件的 unified diff 预览，不实际写入磁盘。
/// 供 Agent / 前端在提交多文件编辑前审查变更范围（新增/删除行数 + 差异文本）。
#[tauri::command]
pub async fn yuan_multi_file_preview_diffs(
    state: State<'_, AppState>,
    request: MultiFileEditRequest,
) -> Result<ApiResponse<MultiFileDiffPreview>, String> {
    crate::commands::common::require_auth(&state).await?;
    preview_diffs(&request)
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}
