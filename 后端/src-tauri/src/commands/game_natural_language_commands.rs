//! D4.4 游戏自然语言交互 - IPC 命令层
//!
//! 命令清单：
//! - game_nl_parse: 解析玩家自然语言命令为结构化动作（调用云端 API，AI 失败降级到规则解析）
//!
//! 风格约定：与 game_story_commands.rs 一致
//! - 走云端 API（AiModelService），不使用底层智能模型
//! - 解析后由前端根据 action 分发到既有 game 命令执行（非侵入式）

use tauri::State;

use crate::commands::common::require_auth;
use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::services::game_natural_language_service::{
    GameNaturalLanguageService, ParseCommandRequest, ParsedCommand,
};

/// 解析玩家自然语言命令
///
/// 调用云端 API 将自然语言（如「建造图书馆」）解析为结构化动作。
/// 响应中 `used_ai` 字段标识是否走了 AI（false 表示降级到规则解析）。
/// 前端拿到 `action` 后分发到对应既有 game 命令执行建造/突破等。
#[tauri::command]
pub async fn game_nl_parse(
    state: State<'_, AppState>,
    mut request: ParseCommandRequest,
) -> Result<ApiResponse<ParsedCommand>, String> {
    let user_id = require_auth(&state).await?;
    request.user_id = user_id;

    // A5 Phase 3 Task 2: 取网络状态传入 service（离线时 service 内部直接走规则解析降级）
    let is_online = state.connectivity_checker.get_status().await.is_online;

    match GameNaturalLanguageService::parse_command(&state.pool, &state.mek_manager, request, is_online).await
    {
        Ok(cmd) => {
            if !cmd.used_ai {
                tracing::info!("[D4.4-NL] 命令解析降级到规则（confidence={}）", cmd.confidence);
            }
            Ok(ApiResponse::success(cmd))
        }
        Err(e) => Err(e.into()),
    }
}
