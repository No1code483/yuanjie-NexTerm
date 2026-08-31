//! D4.4 游戏动态剧情 - IPC 命令层
//!
//! 命令清单：
//! - game_story_generate:  生成动态剧情（调用云端 API，AI 失败降级到预设模板）
//! - game_story_advance:   推进剧情到下一节点（根据玩家选择）
//! - game_story_get:        获取当前剧情状态（D4.4b 从 DB 读取完整剧情）
//! - game_story_list:       列出玩家历史剧情摘要（D4.4b 新增，跨会话恢复）
//!
//! 风格约定：与 game_opponent_commands.rs 一致
//! - 函数签名：`pub async fn xxx(state: State<'_, AppState>, ...) -> Result<ApiResponse<T>, String>`
//! - 走云端 API（AiModelService），不使用底层智能模型

use tauri::State;

use crate::commands::common::require_auth;
use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::services::game_story_service::{
    AdvanceStoryRequest, GenerateStoryRequest, GameStoryService, Story, StorySummary,
};

/// 生成动态剧情
///
/// 调用云端 API 生成初始剧情节点。AI 失败时降级到预设剧情模板，保证游戏可用性。
/// 响应中 `used_ai` 字段标识是否走了 AI（false 表示降级模板）。
/// 持久化：剧情会话 + 首节点写入 DB（game_stories + game_story_nodes）。
#[tauri::command]
pub async fn game_story_generate(
    state: State<'_, AppState>,
    mut request: GenerateStoryRequest,
) -> Result<ApiResponse<Story>, String> {
    let user_id = require_auth(&state).await?;
    request.user_id = user_id;

    // A5 Phase 3 Task 2: 取网络状态传入 service（离线时 service 内部直接走预设模板降级）
    let is_online = state.connectivity_checker.get_status().await.is_online;

    match GameStoryService::generate_story(&state.pool, &state.mek_manager, request, is_online).await {
        Ok(story) => {
            if !story.used_ai {
                tracing::info!("剧情生成降级到预设模板（theme={}）", story.theme);
            }
            Ok(ApiResponse::success(story))
        }
        Err(e) => Err(e.into()),
    }
}

/// 推进剧情到下一节点
///
/// 根据玩家选择调用云端 API 生成后续剧情节点。
/// AI 失败时返回错误（前端可重试）。
/// 持久化：新节点写入 DB，剧情主表更新 current_node_id / node_count / is_finished。
#[tauri::command]
pub async fn game_story_advance(
    state: State<'_, AppState>,
    mut request: AdvanceStoryRequest,
) -> Result<ApiResponse<Story>, String> {
    let user_id = require_auth(&state).await?;
    request.user_id = user_id;

    // A5 Phase 3 Task 2: 取网络状态传入 service（离线时 service 返回 Offline 错误）
    let is_online = state.connectivity_checker.get_status().await.is_online;

    match GameStoryService::advance_story(&state.pool, &state.mek_manager, request, is_online).await {
        Ok(story) => Ok(ApiResponse::success(story)),
        Err(e) => Err(e.into()),
    }
}

/// 获取当前剧情状态（D4.4b 从 DB 读取完整剧情 + 所有节点）
#[tauri::command]
pub async fn game_story_get(
    state: State<'_, AppState>,
    story_id: String,
) -> Result<ApiResponse<Option<Story>>, String> {
    let _ = require_auth(&state).await?;
    match GameStoryService::get_story(&state.pool, &story_id).await {
        Ok(story) => Ok(ApiResponse::success(story)),
        Err(e) => Err(e.into()),
    }
}

/// 列出玩家历史剧情摘要（D4.4b 新增）
///
/// 按最后推进时间倒序返回，前端用于"历史剧情"时间线展示与跨会话恢复。
#[tauri::command]
pub async fn game_story_list(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<ApiResponse<Vec<StorySummary>>, String> {
    let user_id = require_auth(&state).await?;
    let limit = limit.unwrap_or(50);
    match GameStoryService::list_stories(&state.pool, user_id, limit).await {
        Ok(list) => Ok(ApiResponse::success(list)),
        Err(e) => Err(e.into()),
    }
}
