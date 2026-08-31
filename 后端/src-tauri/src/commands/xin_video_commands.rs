//! D3.5 小欣视频输入多模态 IPC 命令
//!
//! 设计依据：
//!   - 功能展望/模块深化/03_小欣_多模态融合_深度.md §2.4
//!   - .trae/rules/项目核心设计意图.md §四（小欣走云端 API）
//!
//! 命令：
//!   - xin_video_analyze — 抽取视频关键帧返回 base64 图片列表
//!   - xin_video_summarize — D3.5 完整流程：抽帧 + 调用云端多模态 LLM 生成文字摘要
//!   - xin_video_check_ffmpeg — 检查系统 ffmpeg 是否可用
//!
//! 前端拿到帧列表后，可直接作为图片附件走多模态云端 API（GPT-4o 等），
//! 或调用 xin_video_summarize 一步到位生成视频摘要。

use tauri::State;

use crate::db::connection::AppState;
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;
use crate::services::ai_model_service::AiModelService;
use crate::services::xin_video_service::{VideoSummaryResult, VideoUnderstandingResult, XinVideoService};

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

/// 分析视频：抽取关键帧 + 转 base64
///
/// 在 spawn_blocking 线程中执行 ffmpeg（避免阻塞 async runtime）。
/// 返回的帧列表可直接作为图片附件发送给多模态模型。
#[tauri::command]
pub async fn xin_video_analyze(
    state: State<'_, AppState>,
    video_path: String,
    max_frames: Option<usize>,
) -> Result<ApiResponse<VideoUnderstandingResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let result = tokio::task::spawn_blocking(move || {
        XinVideoService::analyze_video(&video_path, max_frames)
    })
    .await
    .map_err(|e| format!("视频分析任务失败: {}", e))?;

    result.map(ApiResponse::success).map_err(|e| e.to_string())
}

/// D3.5 视频摘要：抽帧 + 调用云端多模态 LLM 生成文字摘要
///
/// 完整流程：
///   1. 通过 AiModelService 获取用户配置的云端模型（API Key 加密存储）
///   2. 调用 XinVideoService::summarize_video 抽帧 + 调用多模态 LLM
///   3. 返回摘要 + 关键要点 + 帧（帧可供前端二次询问）
///
/// 设计意图（§四）：小欣走云端 API 多模态模型，非本地底层智能模型
#[tauri::command]
pub async fn xin_video_summarize(
    state: State<'_, AppState>,
    video_path: String,
    max_frames: Option<usize>,
    model_id: Option<i64>,
) -> Result<ApiResponse<VideoSummaryResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    // D3.1: 获取当前用户 ID（未登录时默认 1，向后兼容）
    let user_id = state.current_user.read().await.unwrap_or(1);

    // D3.1: model_id 为空时，回退到第一个可用模型
    let resolved_model_id = if let Some(mid) = model_id {
        mid
    } else {
        let models = crate::db::repositories::ai_repo::get_all_models(&state.pool, user_id)
            .await
            .map_err(|e| e.to_string())?;
        models
            .first()
            .map(|m| m.id)
            .ok_or_else(|| "未配置任何 AI 模型，请先在设置中添加模型".to_string())?
    };

    // 获取模型配置（API Key 通过 MEK 解密）
    let (model, api_key) = AiModelService::get_model_config(
        &state.pool,
        &state.mek_manager,
        user_id,
        resolved_model_id,
    )
    .await
    .map_err(|e| e.to_string())?;

    // 解析 endpoint 和 model_name（复用 xin_dialogue_service 的逻辑）
    let api_url = model
        .api_url
        .as_deref()
        .or_else(|| crate::models::ai_model::get_provider_default_url(&model.provider))
        .unwrap_or("https://api.openai.com/v1")
        .to_string();
    let model_name = model
        .model_name
        .as_deref()
        .or_else(|| crate::models::ai_model::get_provider_default_model(&model.provider))
        .unwrap_or("gpt-4o-mini")
        .to_string();

    // 调用视频摘要服务（抽帧 + LLM 调用）
    let result = XinVideoService::summarize_video(
        &video_path,
        max_frames,
        &api_url,
        &api_key,
        &model_name,
    )
    .await
    .map_err(|e: AppError| e.to_string())?;

    Ok(ApiResponse::success(result))
}

/// 检查系统 ffmpeg 是否可用
#[tauri::command]
pub async fn xin_video_check_ffmpeg(
    state: State<'_, AppState>,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    let available = tokio::task::spawn_blocking(XinVideoService::check_ffmpeg_available)
        .await
        .map_err(|e| format!("检查任务失败: {}", e))?;
    Ok(ApiResponse::success(available))
}
