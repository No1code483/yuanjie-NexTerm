//! Yuan Code v3.2 Task 3.4.4 — 协作会话 IPC 命令层
//!
//! 命令清单：
//! - collab_session_create : 创建协作会话（创建者自动加入）
//! - collab_session_list   : 列出所有活跃会话
//! - collab_session_get    : 获取会话详情（含参与者）
//! - collab_session_join   : 加入会话
//! - collab_session_leave  : 退出会话（最后一人退出时自动关闭）
//! - collab_session_close  : 关闭会话（仅创建者）
//! - collab_session_update_cursor : 更新光标位置（实时光标）
//!
//! 设计文档：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.4.4（Phase 6）
//!
//! 与底层智能的边界：协作会话是非 AI 功能，不调用云端 API，不依赖底层智能。
//! 底层智能可监测协作行为（通过 yuan_code_monitor），但不替代协作逻辑。

use std::sync::Arc;

use tauri::State;

use crate::commands::common::require_auth;
use crate::db::connection::AppState;
use crate::db::repositories::user_repo;
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;
use crate::services::collab::{
    CollabSession, CollabSessionManager, CollabSessionSummary, CursorPosition,
};

/// 获取用户显示名（从 users 表读取；找不到则回退到 user_id 字符串）
async fn resolve_display_name(pool: &sqlx::SqlitePool, user_id: i64) -> String {
    match user_repo::find_by_id(pool, user_id).await {
        Ok(Some(user)) => user.display_name.unwrap_or(user.username),
        _ => user_id.to_string(),
    }
}
/// 获取共享会话管理器（懒初始化，存入 AppState）
async fn get_manager(state: &State<'_, AppState>) -> Arc<CollabSessionManager> {
    let mgr = state.collab_session_manager.read().await;
    if let Some(m) = mgr.as_ref() {
        return m.clone();
    }
    drop(mgr);

    // 双检锁：再持写锁初始化
    let mut mgr_write = state.collab_session_manager.write().await;
    if let Some(m) = mgr_write.as_ref() {
        return m.clone();
    }
    let new_mgr = Arc::new(CollabSessionManager::new());
    *mgr_write = Some(new_mgr.clone());
    new_mgr
}

/// 创建协作会话
#[tauri::command]
pub async fn collab_session_create(
    state: State<'_, AppState>,
    name: String,
    workspace_root: String,
) -> Result<ApiResponse<CollabSession>, String> {
    let user_id = require_auth(&state).await?;
    let display_name = resolve_display_name(&state.pool, user_id).await;
    let mgr = get_manager(&state).await;

    match mgr.create_session(name, workspace_root, user_id, display_name).await {
        Ok(session) => Ok(ApiResponse::success(session)),
        Err(e) => Err(e.to_string()),
    }
}

/// 列出所有活跃会话
#[tauri::command]
pub async fn collab_session_list(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<CollabSessionSummary>>, String> {
    let _ = require_auth(&state).await?;
    let mgr = get_manager(&state).await;
    Ok(ApiResponse::success(mgr.list_sessions().await))
}

/// 获取会话详情
#[tauri::command]
pub async fn collab_session_get(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<ApiResponse<Option<CollabSession>>, String> {
    let _ = require_auth(&state).await?;
    let mgr = get_manager(&state).await;
    match mgr.get_session(&session_id).await {
        Ok(session) => Ok(ApiResponse::success(Some(session))),
        Err(AppError::NotFound) => Ok(ApiResponse::success(None)),
        Err(e) => Err(e.to_string()),
    }
}

/// 加入会话
#[tauri::command]
pub async fn collab_session_join(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<ApiResponse<CollabSession>, String> {
    let user_id = require_auth(&state).await?;
    let display_name = resolve_display_name(&state.pool, user_id).await;
    let mgr = get_manager(&state).await;

    match mgr.join_session(&session_id, user_id, display_name).await {
        Ok(session) => Ok(ApiResponse::success(session)),
        Err(AppError::NotFound) => {
            Ok(ApiResponse::error(2002, &format!("会话不存在: {}", session_id)))
        }
        Err(AppError::Validation(msg)) => Ok(ApiResponse::error(1003, &msg)),
        Err(e) => Err(e.to_string()),
    }
}

/// 退出会话（最后一人退出时自动关闭）
#[tauri::command]
pub async fn collab_session_leave(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;
    let mgr = get_manager(&state).await;
    match mgr.leave_session(&session_id, user_id).await {
        Ok(()) => Ok(ApiResponse::success(())),
        Err(AppError::NotFound) => {
            Ok(ApiResponse::error(2002, &format!("会话不存在: {}", session_id)))
        }
        Err(e) => Err(e.to_string()),
    }
}

/// 关闭会话（仅创建者可关闭）
#[tauri::command]
pub async fn collab_session_close(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;
    let mgr = get_manager(&state).await;
    match mgr.close_session(&session_id, user_id).await {
        Ok(()) => Ok(ApiResponse::success(())),
        Err(AppError::NotFound) => {
            Ok(ApiResponse::error(2002, &format!("会话不存在: {}", session_id)))
        }
        Err(AppError::Permission { .. }) => Ok(ApiResponse::error(
            1002,
            "仅创建者可关闭会话",
        )),
        Err(e) => Err(e.to_string()),
    }
}

/// 更新光标位置（实时光标）
#[tauri::command]
pub async fn collab_session_update_cursor(
    state: State<'_, AppState>,
    session_id: String,
    cursor: Option<CursorPosition>,
) -> Result<ApiResponse<CollabSession>, String> {
    let user_id = require_auth(&state).await?;
    let mgr = get_manager(&state).await;
    match mgr.update_cursor(&session_id, user_id, cursor).await {
        Ok(session) => Ok(ApiResponse::success(session)),
        Err(AppError::NotFound) => {
            Ok(ApiResponse::error(2002, &format!("会话不存在: {}", session_id)))
        }
        Err(AppError::Validation(msg)) => Ok(ApiResponse::error(1003, &msg)),
        Err(e) => Err(e.to_string()),
    }
}
