use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::news::NewsSourceRow;
use crate::services::news_source_service;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn get_news_sources(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<NewsSourceRow>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match news_source_service::get_news_sources(&state.pool, user_id).await {
        Ok(sources) => Ok(ApiResponse::success(sources)),
        Err(e) => Ok(ApiResponse::error(3101, &e.to_string())),
    }
}

#[tauri::command]
pub async fn add_news_source(
    state: State<'_, AppState>,
    name: String,
    url: String,
    category: String,
    feed_type: String,
) -> Result<ApiResponse<NewsSourceRow>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match news_source_service::add_news_source(&state.pool, user_id, &name, &url, &category, &feed_type)
        .await
    {
        Ok(source) => Ok(ApiResponse::success(source)),
        Err(e) => Ok(ApiResponse::error(3102, &e.to_string())),
    }
}

#[tauri::command]
pub async fn delete_news_source(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<bool>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match news_source_service::delete_news_source(&state.pool, user_id, id).await {
        Ok(deleted) => Ok(ApiResponse::success(deleted)),
        Err(e) => Ok(ApiResponse::error(3103, &e.to_string())),
    }
}