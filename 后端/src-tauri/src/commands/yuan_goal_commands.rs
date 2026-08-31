use tauri::State;

use crate::db::connection::AppState;
use crate::goals::continuation::Checkpoint;
use crate::models::api_response::ApiResponse;
use crate::models::goals::{
    CreateGoalRequest, GoalContinuationData, GoalListResponse, GoalSnapshot, TokenConsumeRequest,
    UpdateGoalRequest,
};
use crate::goals::tracker::BudgetStatus;
use crate::services::goal_service::GoalService;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn yuan_goal_create(
    state: State<'_, AppState>,
    request: CreateGoalRequest,
    service: State<'_, GoalService>,
) -> Result<ApiResponse<GoalSnapshot>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .create_goal(request)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_goal_get(
    state: State<'_, AppState>,
    goal_id: i64,
    service: State<'_, GoalService>,
) -> Result<ApiResponse<GoalSnapshot>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .get_goal(goal_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_goal_list(
    state: State<'_, AppState>,
    session_id: String,
    service: State<'_, GoalService>,
) -> Result<ApiResponse<GoalListResponse>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_goals(&session_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_goal_update(
    state: State<'_, AppState>,
    request: UpdateGoalRequest,
    service: State<'_, GoalService>,
) -> Result<ApiResponse<GoalSnapshot>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .update_goal(request)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_goal_start(
    state: State<'_, AppState>,
    goal_id: i64,
    service: State<'_, GoalService>,
) -> Result<ApiResponse<GoalSnapshot>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .start_goal(goal_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_goal_pause(
    state: State<'_, AppState>,
    goal_id: i64,
    service: State<'_, GoalService>,
) -> Result<ApiResponse<GoalSnapshot>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .pause_goal(goal_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_goal_complete(
    state: State<'_, AppState>,
    goal_id: i64,
    result_json: Option<String>,
    service: State<'_, GoalService>,
) -> Result<ApiResponse<GoalSnapshot>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .complete_goal(goal_id, result_json)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_goal_abort(
    state: State<'_, AppState>,
    goal_id: i64,
    reason: String,
    service: State<'_, GoalService>,
) -> Result<ApiResponse<GoalSnapshot>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .abort_goal(goal_id, &reason)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_goal_delete(
    state: State<'_, AppState>,
    goal_id: i64,
    service: State<'_, GoalService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .delete_goal(goal_id)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_goal_consume_tokens(
    state: State<'_, AppState>,
    request: TokenConsumeRequest,
    service: State<'_, GoalService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let status = service.consume_tokens(request).await.map_err(|e| e.to_string())?;
    let label = match status {
        BudgetStatus::Safe => "safe",
        BudgetStatus::Warning => "warning",
        BudgetStatus::Critical => "critical",
        BudgetStatus::Exhausted => "exhausted",
    };
    Ok(ApiResponse::success(label.to_string()))
}

#[tauri::command]
pub async fn yuan_goal_update_progress(
    state: State<'_, AppState>,
    goal_id: i64,
    progress_pct: i32,
    service: State<'_, GoalService>,
) -> Result<ApiResponse<GoalSnapshot>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .update_progress(goal_id, progress_pct)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_goal_save_checkpoint(
    state: State<'_, AppState>,
    goal_id: i64,
    agent_id: Option<String>,
    last_message: String,
    turn_count: u32,
    tokens_used: i64,
    sandbox_id: Option<String>,
    service: State<'_, GoalService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut cp = Checkpoint::new(agent_id, last_message, turn_count, tokens_used);
    if let Some(sid) = sandbox_id {
        cp = cp.with_sandbox(sid);
    }
    service
        .save_checkpoint(goal_id, cp)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_goal_build_continuation(
    state: State<'_, AppState>,
    goal_id: i64,
    service: State<'_, GoalService>,
) -> Result<ApiResponse<GoalContinuationData>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .build_continuation(goal_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_goal_checkpoints_count(
    state: State<'_, AppState>,
    goal_id: i64,
    service: State<'_, GoalService>,
) -> Result<ApiResponse<usize>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .checkpoints_count(goal_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}