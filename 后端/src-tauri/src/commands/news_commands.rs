use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::news::{NewsCache, NewsPendingDelete};
use crate::services::news_service;
use crate::services::news_cache_service::{self, NewsCacheItem, CacheStatus};
use crate::services::intelligence_v4_service;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn get_news(state: State<'_, AppState>) -> Result<ApiResponse<Vec<NewsCache>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match news_service::get_news(&state.pool, user_id).await {
        Ok(news) => Ok(ApiResponse::success(news)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn get_news_by_category(
    state: State<'_, AppState>,
    category: String,
) -> Result<ApiResponse<Vec<NewsCache>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match news_service::get_news_by_category(&state.pool, user_id, &category).await {
        Ok(news) => Ok(ApiResponse::success(news)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn fetch_news(state: State<'_, AppState>) -> Result<ApiResponse<news_service::FetchNewsResult>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match news_service::fetch_and_cache_news(&state.pool, user_id).await {
        Ok(result) => {
            intelligence_v4_service::instrument_cmd(&state, "news", "刷新新闻", None).await;
            Ok(ApiResponse::success(result))
        }
        Err(e) => Ok(ApiResponse::error(2001, &e.to_string())),
    }
}

#[tauri::command]
pub async fn add_news(
    state: State<'_, AppState>,
    title: String,
    url: Option<String>,
    source: Option<String>,
    summary: Option<String>,
) -> Result<ApiResponse<NewsCache>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match news_service::add_news(
        &state.pool,
        user_id,
        &title,
        url.as_deref(),
        source.as_deref(),
        summary.as_deref(),
    )
    .await
    {
        Ok(news) => Ok(ApiResponse::success(news)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn mark_news_read(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match news_service::mark_read(&state.pool, user_id, id).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn clear_old_news(
    state: State<'_, AppState>,
    days: i64,
) -> Result<ApiResponse<u64>, String> {
    crate::commands::common::require_auth(&state).await?;
    match news_service::clear_old(&state.pool, days).await {
        Ok(count) => Ok(ApiResponse::success(count)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn toggle_news_favorite(
    state: State<'_, AppState>,
    id: i64,
    is_favorite: bool,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match news_service::toggle_news_favorite(&state.pool, user_id, id, is_favorite).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn get_pending_delete_news(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<NewsPendingDelete>>, String> {
    crate::commands::common::require_auth(&state).await?;
    match news_service::get_pending_delete_news(&state.pool).await {
        Ok(news) => Ok(ApiResponse::success(news)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn generate_news_ai_summary(
    state: State<'_, AppState>,
    id: i64,
    provider: Option<String>,
    endpoint: Option<String>,
    model: Option<String>,
    is_github: Option<bool>,
) -> Result<ApiResponse<NewsCache>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match news_service::generate_news_ai_summary(
        &state.pool,
        user_id,
        id,
        provider,
        endpoint,
        model,
        is_github,
    )
    .await
    {
        Ok(news) => {
            intelligence_v4_service::instrument_cmd(&state, "news", "AI生成短报", None).await;
            Ok(ApiResponse::success(news))
        }
        Err(e) => Ok(ApiResponse::error(2002, &e.to_string())),
    }
}

// ============================================================================
// A5 离线与同步机制 - Phase 3 Task 3: 新闻源离线缓存 IPC 命令
// 规范: 功能展望/平台级增强/04_离线与同步机制.md §Phase 3 Task 3
// ============================================================================

/// 读取离线缓存的新闻（离线场景使用）
///
/// - source: 可选，按 RSS 源过滤；None 返回所有缓存
/// - limit: 可选，默认 50，上限 200
#[tauri::command]
pub async fn news_get_cached(
    state: State<'_, AppState>,
    source: Option<String>,
    limit: Option<i64>,
) -> Result<ApiResponse<Vec<NewsCacheItem>>, String> {
    crate::commands::common::require_auth(&state).await?;
    match news_cache_service::get_cached_news(
        &state.pool,
        source.as_deref(),
        limit.unwrap_or(50),
    )
    .await
    {
        Ok(items) => Ok(ApiResponse::success(items)),
        Err(e) => Err(e.to_string()),
    }
}

/// 查询新闻离线缓存状态（最新缓存时间 + 条数 + 年龄）
///
/// 前端用于展示「上次更新 N 分钟前」+ 📴 离线标识。
#[tauri::command]
pub async fn news_cache_status(
    state: State<'_, AppState>,
) -> Result<ApiResponse<CacheStatus>, String> {
    crate::commands::common::require_auth(&state).await?;
    match news_cache_service::get_cache_status(&state.pool).await {
        Ok(status) => Ok(ApiResponse::success(status)),
        Err(e) => Err(e.to_string()),
    }
}