//! D3.6 小欣实时对话 IPC 命令层
//!
//! 设计依据：
//!   - 功能展望/模块深化/03_小欣_多模态融合_深度.md §2.5 实时对话
//!   - v2_下一阶段开发计划：D3.6 验收「语音实时对话延迟 < 500ms」
//!
//! 命令清单：
//!   - xin_realtime_start(config) → 启动实时会话（状态 → Listening）
//!   - xin_realtime_stop() → 停止会话（状态 → Idle）
//!   - xin_realtime_push_chunk(samples, timestamp_ms) → 前端推送 PCM 块
//!   - xin_realtime_get_state() → 查询当前状态
//!
//! 架构说明：
//!   - XinRealtimeService 用 once_cell 全局单例（无状态依赖，仅会话上下文）
//!   - STT/LLM/TTS 调用所需的 (pool, mek_manager, user_id) 在 D3.6.3-5 填充时
//!     通过 start_session 的 config 传入 model_id，命令层从 AppState 获取 pool
//!   - 前端通过 Tauri Channel 接收 RealtimeEvent（D3.6.2 建立 Channel，D3.6.3-5 填充事件）

use once_cell::sync::Lazy;
use tauri::{AppHandle, State};

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::services::xin_realtime_service::{
    PcmChunk, RealtimeConfig, RealtimeState, XinRealtimeService,
};

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

/// 全局实时会话服务单例
///
/// 设计：XinRealtimeService 仅持有会话上下文（无 DB/网络资源），用全局单例避免
/// 修改 AppState。实际的 (pool, mek_manager) 在调用 STT/LLM/TTS 时从命令层注入。
static REALTIME_SERVICE: Lazy<XinRealtimeService> = Lazy::new(XinRealtimeService::new);

/// 启动实时会话
///
/// 前端调用：await ipc.invoke('xin_realtime_start', { config })
/// 成功后状态变为 Listening，前端可开始通过 push_chunk 推送 PCM
#[tauri::command]
pub async fn xin_realtime_start(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    config: RealtimeConfig,
) -> Result<ApiResponse<RealtimeState>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = state.current_user.read().await.unwrap_or(1);
    REALTIME_SERVICE
        .start_session(
            config,
            state.pool.clone(),
            state.mek_manager.clone(),
            user_id,
            app_handle,
        )
        .await
        .map_err(|e| e.to_string())?;
    let new_state = REALTIME_SERVICE.get_state().await;
    Ok(ApiResponse::success(new_state))
}

/// 停止实时会话
///
/// 前端调用：await ipc.invoke('xin_realtime_stop')
/// 清理会话上下文，状态回到 Idle
#[tauri::command]
pub async fn xin_realtime_stop(
    state: State<'_, AppState>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    REALTIME_SERVICE
        .stop_session()
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(()))
}

/// 推送 PCM 音频块
///
/// 前端 AudioWorklet 每 20-30ms 调用一次：
///   await ipc.invoke('xin_realtime_push_chunk', {
///     samples: Int16Array,  // 已转为普通数组
///     timestamp_ms: number
///   })
///
/// 返回当前状态，前端可据此切换 UI（Listening → Thinking → Speaking）
#[tauri::command]
pub async fn xin_realtime_push_chunk(
    state: State<'_, AppState>,
    samples: Vec<i16>,
    timestamp_ms: u64,
) -> Result<ApiResponse<RealtimeState>, String> {
    crate::commands::common::require_auth(&state).await?;
    let chunk = PcmChunk {
        samples,
        timestamp_ms,
    };
    let state = REALTIME_SERVICE
        .push_pcm_chunk(chunk)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(state))
}

/// 查询当前会话状态
///
/// 前端轮询或事件丢失时恢复状态用
#[tauri::command]
pub async fn xin_realtime_get_state(
    state: State<'_, AppState>,
) -> Result<ApiResponse<RealtimeState>, String> {
    crate::commands::common::require_auth(&state).await?;
    let state = REALTIME_SERVICE.get_state().await;
    Ok(ApiResponse::success(state))
}
