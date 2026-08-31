use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::journal::Journal;
use crate::services::intelligence_v4_service;
use crate::services::journal_service;
use crate::commands::common::require_auth;

#[tauri::command]
pub async fn get_journal(
    state: State<'_, AppState>,
    date: String,
) -> Result<ApiResponse<Option<Journal>>, String> {
    let user_id = require_auth(&state).await?;
    match journal_service::get_journal(&state.pool, user_id, &date).await {
        Ok(journal) => Ok(ApiResponse::success(journal)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn save_journal(
    state: State<'_, AppState>,
    date: String,
    content: String,
) -> Result<ApiResponse<Journal>, String> {
    let user_id = require_auth(&state).await?;
    match journal_service::save_journal(&state.pool, user_id, &date, &content).await {
        Ok(journal) => {
            intelligence_v4_service::instrument_cmd(&state, "journal", &format!("编辑日志/{}", date), None).await;
            Ok(ApiResponse::success(journal))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn delete_journal(
    state: State<'_, AppState>,
    date: String,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;
    match journal_service::delete_journal(&state.pool, user_id, &date).await {
        Ok(_) => {
            intelligence_v4_service::instrument_cmd(&state, "journal", &format!("删除日志/{}", date), None).await;
            Ok(ApiResponse::success(()))
        }
        Err(e) => Err(e.into()),
    }
}
