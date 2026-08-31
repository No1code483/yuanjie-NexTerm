/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::xin::{
    ConversationSummary, DailyBriefing, MemoryCategory, MultimodalInput, PersonalityInsight,
    TtsSpeakRequest, UserMemory, XinConfig,
};

#[tauri::command]
pub async fn xin_get_config(
    state: State<'_, AppState>,
) -> Result<ApiResponse<XinConfig>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(state.xiaoxin_service.get_config(user_id).await))
}

#[tauri::command]
pub async fn xin_set_config(
    state: State<'_, AppState>,
    config: XinConfig,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    state.xiaoxin_service.update_config(user_id, config).await;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn xin_get_personas(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<crate::models::xin::Persona>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(state.xiaoxin_service.get_personas(user_id).await))
}

#[tauri::command]
pub async fn xin_get_active_persona(
    state: State<'_, AppState>,
) -> Result<ApiResponse<crate::models::xin::Persona>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        state.xiaoxin_service.get_active_persona(user_id).await,
    ))
}

#[tauri::command]
pub async fn xin_set_active_persona(
    state: State<'_, AppState>,
    persona_id: String,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    // D3.8: 记录切换前的人格 ID，用于切换历史审计
    let from_persona_id = state.xiaoxin_service.get_config(user_id).await.active_persona_id;
    state
        .xiaoxin_service
        .set_active_persona(user_id, &persona_id)
        .await
        .map(|_| {
            // D3.8: 记录人格切换历史（失败不阻塞切换流程）
            let _ = crate::services::xin_personality_service::XinPersonalityService::log_switch(
                &state.pool,
                user_id,
                Some(&from_persona_id),
                &persona_id,
                Some("manual"),
            );
            ApiResponse::success(())
        })
        .map_err(|e| e.to_string())
}

/// D3.8: 查询某人格的历史记忆（交互摘要 + 用户偏好 + 话题标签）
#[tauri::command]
pub async fn xin_persona_memory(
    state: State<'_, AppState>,
    persona_id: String,
) -> Result<ApiResponse<crate::services::xin_personality_service::PersonaMemory>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let memory = crate::services::xin_personality_service::XinPersonalityService::get_persona_memory(
        &state.pool, user_id, &persona_id,
    ).await.map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(memory))
}

/// D3.8: 查询人格切换历史（前端时间线展示）
#[tauri::command]
pub async fn xin_persona_switch_history(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<ApiResponse<Vec<crate::services::xin_personality_service::PersonaSwitchRecord>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let limit = limit.unwrap_or(50);
    let history = crate::services::xin_personality_service::XinPersonalityService::get_switch_history(
        &state.pool, user_id, limit
    ).await.map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(history))
}

/// D3.8: 列出所有有记忆记录的人格 ID（按最后交互时间倒序）
#[tauri::command]
pub async fn xin_memorized_personas(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<String>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let personas = crate::services::xin_personality_service::XinPersonalityService::list_memorized_personas(
        &state.pool, user_id
    ).await.map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(personas))
}

/// D3.8.3: 触发自生长人格生长（接收用户画像 JSON）
///
/// 边界：用户画像由调用方提供（前端测试用 / 底层智能 V5 完善后自动调用）。
/// 当前 V4 阶段：前端可手动传入画像测试生长效果；V5 后由底层智能自动生成画像。
#[tauri::command]
pub async fn xin_grow_persona(
    state: State<'_, AppState>,
    profile: crate::services::xin_personality_service::UserProfile,
) -> Result<ApiResponse<Option<crate::services::xin_personality_service::GrownPersona>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let grown = crate::services::xin_personality_service::XinPersonalityService::grow_persona(&profile);
    Ok(ApiResponse::success(grown))
}

/// D3.8.3: 查询自生长人格当前状态
///
/// 基于已积累的人格记忆估算 confidence，返回生长结果（数据不足时返回 None）。
#[tauri::command]
pub async fn xin_self_growing_persona(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Option<crate::services::xin_personality_service::GrownPersona>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    // 从 self_growing 人格记忆中估算 confidence
    let memory = crate::services::xin_personality_service::XinPersonalityService::get_persona_memory(
        &state.pool, user_id, "self_growing"
    ).await.map_err(|e| e.to_string())?;

    // 基于 interaction_count 估算 confidence（200 次交互达 1.0）
    let confidence = (memory.interaction_count as f64 / 200.0).clamp(0.0, 1.0);

    // 从 user_preferences 和 topic_tags 提取画像（V4 阶段：使用已有记忆数据）
    let formality = memory.user_preferences.get("formality")
        .and_then(|v| v.as_f64()).unwrap_or(0.5);
    let verbosity = memory.user_preferences.get("verbosity")
        .and_then(|v| v.as_f64()).unwrap_or(0.5);
    let humor = memory.user_preferences.get("humor")
        .and_then(|v| v.as_f64()).unwrap_or(0.5);
    let technical_depth = memory.user_preferences.get("technical_depth")
        .and_then(|v| v.as_f64()).unwrap_or(0.5);
    let empathy = memory.user_preferences.get("empathy")
        .and_then(|v| v.as_f64()).unwrap_or(0.5);
    let interest_domains: Vec<String> = memory.topic_tags.as_array()
        .map(|arr| arr.iter().filter_map(|t| t.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let profile = crate::services::xin_personality_service::UserProfile {
        formality, verbosity, humor, technical_depth, empathy, interest_domains, confidence,
    };
    let grown = crate::services::xin_personality_service::XinPersonalityService::grow_persona(&profile);
    Ok(ApiResponse::success(grown))
}

#[tauri::command]
pub async fn xin_save_memory(
    state: State<'_, AppState>,
    memory: UserMemory,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    state.xiaoxin_service.save_memory(user_id, memory).await;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn xin_get_memories(
    state: State<'_, AppState>,
    category: Option<MemoryCategory>,
    limit: Option<usize>,
) -> Result<ApiResponse<Vec<UserMemory>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        state.xiaoxin_service.get_memories(user_id, category, limit).await,
    ))
}

#[tauri::command]
pub async fn xin_search_memories(
    state: State<'_, AppState>,
    query: String,
) -> Result<ApiResponse<Vec<UserMemory>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        state.xiaoxin_service.search_memories(user_id, &query).await,
    ))
}

#[tauri::command]
pub async fn xin_delete_memory(
    state: State<'_, AppState>,
    id: String,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    state
        .xiaoxin_service
        .delete_memory(user_id, &id)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn xin_analyze_sentiment(
    state: State<'_, AppState>,
    text: String,
) -> Result<ApiResponse<crate::models::xin::SentimentResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        state.xiaoxin_service.analyze_sentiment(&text).await,
    ))
}

#[tauri::command]
pub async fn xin_get_tts_status(
    state: State<'_, AppState>,
) -> Result<ApiResponse<crate::models::xin::TtsStatus>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(state.xiaoxin_service.get_tts_status().await))
}

#[tauri::command]
pub async fn xin_tts_speak(
    state: State<'_, AppState>,
    request: TtsSpeakRequest,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    state
        .xiaoxin_service
        .tts_speak(user_id, request)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn xin_process_multimodal(
    state: State<'_, AppState>,
    input: MultimodalInput,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    state
        .xiaoxin_service
        .process_multimodal(input)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn xin_get_mood(
    state: State<'_, AppState>,
) -> Result<ApiResponse<crate::models::xin::Mood>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        state.xiaoxin_service.get_current_mood(user_id).await,
    ))
}

#[tauri::command]
pub async fn xin_update_mood(
    state: State<'_, AppState>,
    text: String,
) -> Result<ApiResponse<crate::models::xin::Mood>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        state.xiaoxin_service.update_mood(user_id, &text).await,
    ))
}

/// D3.7: 查询用户情绪历史趋势（基于 xin_moods 表）
///
/// 前端用于情绪时间线展示。返回最近的情绪记录（类别 + 强度 + 时间 + 触发文本）。
#[tauri::command]
pub async fn xin_emotion_trend(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<ApiResponse<Vec<crate::services::xin_emotion_service::EmotionTrendPoint>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let limit = limit.unwrap_or(20);
    let trend = crate::services::xin_emotion_service::XinEmotionService::get_emotion_trend(
        &state.pool,
        user_id,
        limit,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(trend))
}

#[tauri::command]
pub async fn xin_add_summary(
    state: State<'_, AppState>,
    summary: ConversationSummary,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    state.xiaoxin_service.add_conversation_summary(user_id, summary).await;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn xin_get_summaries(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<ApiResponse<Vec<ConversationSummary>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        state
            .xiaoxin_service
            .get_conversation_summaries(user_id, limit)
            .await,
    ))
}

#[tauri::command]
pub async fn xin_daily_briefing(
    state: State<'_, AppState>,
) -> Result<ApiResponse<DailyBriefing>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        state.xiaoxin_service.generate_daily_briefing(user_id).await,
    ))
}

#[tauri::command]
pub async fn xin_personality_insights(
    state: State<'_, AppState>,
) -> Result<ApiResponse<PersonalityInsight>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        state.xiaoxin_service.generate_personality_insights(user_id).await,
    ))
}

/// D3.3 STT 语音转写命令。
///
/// 接收音频文件路径（前端先录音保存到临时文件，再传路径），调用 Whisper API 转写。
#[tauri::command]
pub async fn xin_voice_input(
    state: State<'_, AppState>,
    audio_path: String,
    language: Option<String>,
    model_id: Option<i64>,
) -> Result<ApiResponse<crate::services::xin_stt_service::SttTranscribeResponse>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = state.current_user.read().await.unwrap_or(1);

    // 默认模型：第一个 provider=openai-compatible 的模型
    let resolved_model_id = match model_id {
        Some(mid) => mid,
        None => {
            let models = crate::db::repositories::ai_repo::get_all_models(&state.pool, user_id)
                .await
                .map_err(|e| e.to_string())?;
            models
                .first()
                .map(|m| m.id)
                .ok_or_else(|| "未配置任何 AI 模型，请先在设置中添加支持 Whisper 的模型".to_string())?
        }
    };

    let request = crate::services::xin_stt_service::SttTranscribeRequest {
        audio_path,
        language,
        model_id: resolved_model_id,
    };

    crate::services::xin_stt_service::XinSttService::transcribe(
        &state.pool,
        &state.mek_manager,
        user_id,
        request,
    )
    .await
    .map(ApiResponse::success)
    .map_err(|e| e.to_string())
}

/// D3.2 TTS 合成命令：将文本合成为语音文件，返回本地文件路径。
///
/// 前端通过 `convertFileSrc` 将本地路径转为 webview 可访问的 URL，再用 `<audio>` 播放。
#[tauri::command]
pub async fn xin_tts(
    state: State<'_, AppState>,
    text: String,
    voice: Option<String>,
    speed: Option<f64>,
    pitch: Option<f64>,
) -> Result<ApiResponse<crate::services::xin_tts_service::TtsSynthesizeResponse>, String> {
    crate::commands::common::require_auth(&state).await?;
    let request = crate::services::xin_tts_service::TtsSynthesizeRequest {
        text,
        voice,
        speed,
        pitch,
    };
    state
        .xin_tts_service
        .synthesize(request)
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// D3.2 列出当前 TTS 引擎与可用音色。
#[tauri::command]
pub async fn xin_tts_list_voices(
    state: State<'_, AppState>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(serde_json::json!({
        "engine": state.xin_tts_service.engine_name(),
        "voices": state.xin_tts_service.list_voices(),
    })))
}