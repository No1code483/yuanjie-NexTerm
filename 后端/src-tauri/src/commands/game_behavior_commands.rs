//! D4.6 游戏数据分析 AI 洞察 - IPC 命令层
//!
//! 命令清单：
//! - game_analyze_behavior: 聚合玩家行为 → 调用云端 API 输出洞察（AI 失败降级到规则分析）
//!
//! 风格约定：与 game_story_commands.rs 一致
//! - 走云端 API（AiModelService），不使用底层智能模型
//! - 纯只读分析，不修改游戏数据

use tauri::State;

use crate::commands::common::require_auth;
use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::services::game_behavior_analyzer_service::{
    AnalyzeBehaviorRequest, BehaviorAnalysis, GameBehaviorAnalyzerService,
};

/// 分析玩家行为，输出 AI 洞察
///
/// 调用云端 API 基于玩家行为快照生成 3-5 条个性化洞察。
/// 响应中 `used_ai` 字段标识是否走了 AI（false 表示降级到规则分析）。
#[tauri::command]
pub async fn game_analyze_behavior(
    state: State<'_, AppState>,
    mut request: AnalyzeBehaviorRequest,
) -> Result<ApiResponse<BehaviorAnalysis>, String> {
    let user_id = require_auth(&state).await?;
    request.user_id = user_id;

    // A5 Phase 3 Task 2: 取网络状态传入 service（离线时 service 内部直接走规则化洞察降级）
    let is_online = state.connectivity_checker.get_status().await.is_online;

    match GameBehaviorAnalyzerService::analyze_behavior(&state.pool, &state.mek_manager, request, is_online)
        .await
    {
        Ok(analysis) => {
            if !analysis.used_ai {
                tracing::info!("[D4.6] 行为分析降级到规则（{} 条洞察）", analysis.insights.len());
            }
            Ok(ApiResponse::success(analysis))
        }
        Err(e) => Err(e.into()),
    }
}
