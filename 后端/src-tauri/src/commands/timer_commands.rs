use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::timer::{CreateTimerRequest, Timer};
use crate::services::intelligence_v4_service;
use crate::services::timer_service;
use crate::commands::common::require_auth;

#[tauri::command]
pub async fn get_timers(state: State<'_, AppState>) -> Result<ApiResponse<Vec<Timer>>, String> {
    let user_id = require_auth(&state).await?;
    match timer_service::get_timers(&state.pool, user_id).await {
        Ok(timers) => Ok(ApiResponse::success(timers)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn create_timer(
    state: State<'_, AppState>,
    request: CreateTimerRequest,
) -> Result<ApiResponse<Timer>, String> {
    let user_id = require_auth(&state).await?;
    match timer_service::create_timer(
        &state.pool,
        user_id,
        request.name.as_deref(),
        &request.r#type,
        request.target_time,
    )
    .await
    {
        Ok(timer) => {
            intelligence_v4_service::instrument_cmd(&state, "timer", "创建计时", None).await;
            Ok(ApiResponse::success(timer))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn update_timer_state(
    state: State<'_, AppState>,
    id: i64,
    is_running: bool,
    elapsed: i64,
) -> Result<ApiResponse<Timer>, String> {
    let user_id = require_auth(&state).await?;
    match timer_service::update_timer_state(&state.pool, user_id, id, is_running, elapsed).await {
        Ok(timer) => {
            let action = if is_running { "开始计时" } else { "暂停计时" };
            intelligence_v4_service::instrument_cmd(&state, "timer", action, None).await;
            Ok(ApiResponse::success(timer))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn delete_timer(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;
    match timer_service::delete_timer(&state.pool, user_id, id).await {
        Ok(_) => {
            intelligence_v4_service::instrument_cmd(&state, "timer", "删除计时", None).await;
            Ok(ApiResponse::success(()))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn timer_action(
    state: State<'_, AppState>,
    id: i64,
    action: String,
) -> Result<ApiResponse<Timer>, String> {
    let user_id = require_auth(&state).await?;
    match timer_service::timer_action(&state.pool, user_id, id, &action).await {
        Ok(timer) => {
            let op = format!("计时操作/{}", action);
            intelligence_v4_service::instrument_cmd(&state, "timer", &op, None).await;
            Ok(ApiResponse::success(timer))
        }
        Err(e) => Err(e.into()),
    }
}
