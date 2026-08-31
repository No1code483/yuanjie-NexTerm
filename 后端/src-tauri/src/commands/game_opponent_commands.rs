//! D4.3 游戏对手 AI - IPC 命令层
//!
//! 命令清单：
//! - game_opponent_decide: 调用对手 AI 生成本回合决策（AI 失败自动降级到启发式 mock）
//!
//! 风格约定：与 game_commands.rs 一致
//! - 函数签名：`pub async fn xxx(state: State<'_, AppState>, ...) -> Result<ApiResponse<T>, String>`

use tauri::State;

use crate::commands::common::require_auth;
use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::services::game_opponent_service::{
    GameOpponentService, OpponentDecision, OpponentDecisionRequest,
};

/// 生成对手本回合决策。
///
/// - 调用云端 API（AiModelService）生成决策
/// - AI 失败或返回内容无法解析时，自动降级到启发式 mock 决策
/// - 始终返回成功（保证游戏可用性），`used_ai` 字段标识是否走了 AI
#[tauri::command]
pub async fn game_opponent_decide(
    state: State<'_, AppState>,
    request: OpponentDecisionRequest,
) -> Result<ApiResponse<OpponentDecision>, String> {
    let _user_id = require_auth(&state).await?;

    // 请求体中的 user_id 可能与当前认证用户不一致，以认证用户为准
    let mut req = request;
    req.user_id = _user_id;

    match GameOpponentService::make_decision(
        &state.pool,
        &state.mek_manager,
        req,
    )
    .await
    {
        Ok(decision) => {
            if !decision.used_ai {
                tracing::info!(
                    "对手 AI 决策降级到启发式 mock（action={}）",
                    decision.action
                );
            }
            Ok(ApiResponse::success(decision))
        }
        Err(e) => Err(e.into()),
    }
}
