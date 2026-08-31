use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::recycle::RecycleBinItem;
use crate::models::recycle::RecycleStats;
use crate::services::recycle_service;
use crate::commands::common::require_auth;

#[tauri::command]
pub async fn recycle_list(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<RecycleBinItem>>, String> {
    // 多用户隔离（批次 6）：仅返回当前用户的回收站项目
    let user_id = require_auth(&state).await?;
    match recycle_service::get_items(&state.pool, user_id).await {
        Ok(items) => Ok(ApiResponse::success(items)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn recycle_move_to(
    state: State<'_, AppState>,
    item_type: String,
    item_ids: Vec<i64>,
) -> Result<ApiResponse<()>, String> {
    let deleted_by = require_auth(&state).await?;
    match recycle_service::move_to_recycle(&state.pool, deleted_by, &item_type, &item_ids, Some(deleted_by)).await {
        Ok(()) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn recycle_restore(
    state: State<'_, AppState>,
    ids: Vec<i64>,
) -> Result<ApiResponse<u64>, String> {
    let user_id = require_auth(&state).await?;
    match recycle_service::restore_items(&state.pool, user_id, &ids).await {
        Ok(count) => Ok(ApiResponse::success(count)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn recycle_delete_permanently(
    state: State<'_, AppState>,
    ids: Vec<i64>,
) -> Result<ApiResponse<u64>, String> {
    // 多用户隔离（批次 6）：仅允许彻底删除当前用户自己的回收站项目
    let user_id = require_auth(&state).await?;
    match recycle_service::delete_permanently_batch(&state.pool, user_id, &ids).await {
        Ok(count) => Ok(ApiResponse::success(count)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn recycle_empty_all(
    state: State<'_, AppState>,
) -> Result<ApiResponse<()>, String> {
    // 多用户隔离（批次 6）：仅清空当前用户的回收站
    let user_id = require_auth(&state).await?;
    match recycle_service::clear_all(&state.pool, user_id).await {
        Ok(()) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn cleanup_expired_recycle(
    state: State<'_, AppState>,
) -> Result<ApiResponse<u64>, String> {
    let _user_id = require_auth(&state).await?;
    match recycle_service::cleanup_expired(&state.pool).await {
        Ok(count) => Ok(ApiResponse::success(count)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn recycle_stats(
    state: State<'_, AppState>,
) -> Result<ApiResponse<RecycleStats>, String> {
    // 多用户隔离（批次 6）：仅统计当前用户的回收站数据
    let user_id = require_auth(&state).await?;
    match recycle_service::get_stats(&state.pool, user_id).await {
        Ok(stats) => Ok(ApiResponse::success(stats)),
        Err(e) => Err(e.into()),
    }
}