//! D4.6 游戏数据底层智能监测接入 - IPC 命令层
//!
//! 命令清单：
//! - game_intelligence_record_event: 非侵入式记录游戏事件到底层智能 activity_logs（关闭时 no-op）
//! - game_intelligence_status: 查询底层智能监测钩子状态（启用与否 + 近期事件数）
//!
//! 边界（项目核心设计意图 §二、§8.2）：
//! - 此钩子仅"喂数据给底层智能监测"，不替代游戏的云端 API 调用
//! - 底层智能关闭时，record_event 立即 no-op，但游戏核心功能（D4.4/D4.5/D4.6）仍可用
//! - 非侵入式：游戏流程不依赖此钩子；前端可选地调用 record_event 上报事件

use tauri::State;

use crate::commands::common::require_auth;
use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::services::game_intelligence_hook::{self, GameEventType, IntelligenceHookStatus};

/// 记录一条游戏事件供底层智能监测消费（非侵入式、可关闭）
///
/// 前端在发生建造/突破/NPC 对话/剧情推进等事件后可选调用本命令上报。
/// 底层智能关闭时立即 no-op；写入失败不影响游戏流程。
#[tauri::command]
pub async fn game_intelligence_record_event(
    state: State<'_, AppState>,
    event_type: String,
    detail: Option<String>,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;

    let parsed = match event_type.as_str() {
        "build" => GameEventType::Build,
        "upgrade" => GameEventType::Upgrade,
        "remove" => GameEventType::Remove,
        "breakthrough" => GameEventType::Breakthrough,
        "npc_chat" => GameEventType::NpcChat,
        "story_advance" => GameEventType::StoryAdvance,
        other => GameEventType::Custom(other.to_string()),
    };

    match game_intelligence_hook::record_event(
        &state.pool,
        &state.intelligence_service,
        user_id,
        parsed,
        detail.as_deref(),
    )
    .await
    {
        Ok(()) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

/// 查询底层智能监测钩子状态
#[tauri::command]
pub async fn game_intelligence_status(
    state: State<'_, AppState>,
) -> Result<ApiResponse<IntelligenceHookStatus>, String> {
    let user_id = require_auth(&state).await?;
    match game_intelligence_hook::get_status(&state.pool, &state.intelligence_service, user_id)
        .await
    {
        Ok(status) => Ok(ApiResponse::success(status)),
        Err(e) => {
            // 状态查询失败不应阻塞前端展示，返回一个关闭态默认值
            tracing::warn!("[D4.6-hook] 钩子状态查询失败: {}", e);
            Ok(ApiResponse::success(IntelligenceHookStatus {
                enabled: false,
                recent_event_count: 0,
            }))
        }
    }
}
