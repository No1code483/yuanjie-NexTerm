/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

use tauri::State;

use crate::db::connection::AppState;
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;
use crate::models::intelligence::{
    ChatModuleContext, DailyBriefingData, GameModuleContext, IntelligenceConfig, UserActivity,
    UserContext,
};
use crate::services::{journal_service, news_service, timer_service, todo_service};

#[tauri::command]
pub async fn intelligence_get_config(
    state: State<'_, AppState>,
) -> Result<ApiResponse<IntelligenceConfig>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(state.intelligence_service.get_config().await))
}

/// D2.1 可关闭性：查询底层智能全局开关状态
#[tauri::command]
pub async fn intelligence_get_enabled(
    state: State<'_, AppState>,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(state.intelligence_service.is_enabled().await))
}

/// D2.1 可关闭性：一键开关底层智能（非侵入式，关闭后各模块核心功能仍可用）
#[tauri::command]
pub async fn intelligence_set_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    state.intelligence_service.set_enabled(enabled).await;
    Ok(ApiResponse::success(()))
}

/// D2.2 触发执行式：主动检测上下文 → 返回可执行动作列表
/// 设计意图（§2.3）：底层智能输出「优化建议/触发执行动作」，不是「对话回复」
#[tauri::command]
pub async fn intelligence_trigger_proactive_actions(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<crate::models::intelligence::Suggestion>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        state.intelligence_service.trigger_proactive_actions().await,
    ))
}

#[tauri::command]
pub async fn intelligence_set_config(
    state: State<'_, AppState>,
    config: IntelligenceConfig,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    state.intelligence_service.update_config(config).await;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn intelligence_get_context(
    state: State<'_, AppState>,
) -> Result<ApiResponse<UserContext>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(state.intelligence_service.get_context().await))
}

#[tauri::command]
pub async fn intelligence_track_activity(
    state: State<'_, AppState>,
    activity: UserActivity,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    state.intelligence_service.track_activity(activity).await;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn intelligence_set_active_file(
    state: State<'_, AppState>,
    path: Option<String>,
    language: Option<String>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    state.intelligence_service.set_active_file(path, language).await;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn intelligence_add_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    state.intelligence_service.add_terminal_session(session_id).await;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn intelligence_remove_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    state.intelligence_service.remove_terminal_session(&session_id).await;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn intelligence_set_window_title(
    state: State<'_, AppState>,
    title: Option<String>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    state.intelligence_service.set_window_title(title).await;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn intelligence_get_ollama_status(
    state: State<'_, AppState>,
) -> Result<ApiResponse<crate::models::intelligence::OllamaStatus>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        state.intelligence_service.check_ollama_status().await,
    ))
}

#[tauri::command]
pub async fn intelligence_get_suggestions(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<crate::models::intelligence::Suggestion>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        state.intelligence_service.generate_suggestions().await,
    ))
}

#[tauri::command]
pub async fn intelligence_take_snapshot(
    state: State<'_, AppState>,
) -> Result<ApiResponse<crate::models::intelligence::ContextSnapshot>, String> {
    crate::commands::common::require_auth(&state).await?;
    state
        .intelligence_service
        .take_snapshot()
        .await
        .map(ApiResponse::success)
        .map_err(|e: crate::error::app_error::AppError| e.to_string())
}

#[tauri::command]
pub async fn intelligence_get_snapshots(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<crate::models::intelligence::ContextSnapshot>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        state.intelligence_service.get_snapshots().await,
    ))
}

#[tauri::command]
pub async fn intelligence_query_local_llm(
    state: State<'_, AppState>,
    prompt: String,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    match state
        .intelligence_service
        .query_local_llm(&prompt)
        .await
    {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => {
            tracing::error!("[LLM] query_local_llm 失败: {:?}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn intelligence_terminal_suggest(
    state: State<'_, AppState>,
    prompt: String,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let system_prompt = "You are a terminal command expert. Convert the user's natural language request into a single shell command. Only respond with the command, no explanation. If the request is ambiguous, provide the most likely command. Platform: Windows (PowerShell/CMD).";
    let full_prompt = format!("{}\n\nUser request: {}", system_prompt, prompt);
    state
        .intelligence_service
        .query_local_llm(&full_prompt)
        .await
        .map(|response| {
            let cmd = response.trim().trim_matches('"').trim_matches('\'').to_string();
            ApiResponse::success(cmd)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_resume_polish(
    state: State<'_, AppState>,
    text: String,
    section: Option<String>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let section_hint = section
        .as_deref()
        .map(|s| format!("\nThis text belongs to the resume section: {}.", s))
        .unwrap_or_default();
    let system_prompt = format!(
        "You are a professional resume editor. Polish the following resume text to be more professional, concise, and impactful. Only return the polished text, no explanations.{}",
        section_hint
    );
    let full_prompt = format!("{}\n\nResume text to polish:\n{}", system_prompt, text);
    state
        .intelligence_service
        .query_local_llm(&full_prompt)
        .await
        .map(|response| ApiResponse::success(response.trim().to_string()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_news_summary(
    state: State<'_, AppState>,
    title: String,
    content: String,
    url: Option<String>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let url_hint = url
        .as_deref()
        .map(|u| format!("\nSource URL: {}", u))
        .unwrap_or_default();
    let system_prompt = format!(
        "You are a news summarizer. Provide a concise TL;DR summary (2-3 sentences in Chinese) of the following news article. Only return the summary, no explanations.{}",
        url_hint
    );
    let full_prompt = format!("{}\n\nTitle: {}\nContent: {}", system_prompt, title, content);
    state
        .intelligence_service
        .query_local_llm(&full_prompt)
        .await
        .map(|response| ApiResponse::success(response.trim().to_string()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_todo_suggest(
    state: State<'_, AppState>,
    partial_text: String,
) -> Result<ApiResponse<Vec<String>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let system_prompt = "You are a task management assistant. The user is typing a task. Based on the partial input, suggest 3-5 specific, actionable task completions. Return each suggestion on a new line, prefixed with '- '. Only return the suggestions, no explanations.";
    let full_prompt = format!("{}\n\nPartial task input: \"{}\"", system_prompt, partial_text);
    state
        .intelligence_service
        .query_local_llm(&full_prompt)
        .await
        .map(|response| {
            let suggestions: Vec<String> = response
                .lines()
                .filter(|l| l.starts_with("- "))
                .map(|l| l[2..].trim().to_string())
                .collect();
            if suggestions.is_empty() {
                vec![response.trim().to_string()]
            } else {
                suggestions
            }
        })
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_timer_remind(
    state: State<'_, AppState>,
    timer_state: String,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let system_prompt = "You are a productivity assistant following the Pomodoro technique. Based on the timer state, suggest whether to take a break or continue. Provide a concise reminder message in Chinese. Only return the message, no explanations.";
    let full_prompt = format!("{}\n\nTimer state: {}", system_prompt, timer_state);
    state
        .intelligence_service
        .query_local_llm(&full_prompt)
        .await
        .map(|response| ApiResponse::success(response.trim().to_string()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_kb_classify(
    state: State<'_, AppState>,
    _entry_id: Option<i64>,
    content: String,
) -> Result<ApiResponse<crate::models::intelligence_cross_module::ClassifyRecommendation>, String> {
    crate::commands::common::require_auth(&state).await?;
    let system_prompt = "You are a knowledge base classifier. Analyze the content and suggest the most appropriate category. Respond with a JSON object: {\"category_name\": \"...\", \"confidence\": 0.95, \"reason\": \"...\", \"description\": \"...\"}. Only return the JSON, no explanations.";
    let full_prompt = format!("{}\n\nContent to classify:\n{}", system_prompt, content);
    state
        .intelligence_service
        .query_local_llm(&full_prompt)
        .await
        .and_then(|response| {
            let trimmed = response.trim();
            // Try to extract JSON from the response
            let json_str = if let Some(start) = trimmed.find('{') {
                if let Some(end) = trimmed.rfind('}') {
                    &trimmed[start..=end]
                } else {
                    trimmed
                }
            } else {
                trimmed
            };
            serde_json::from_str::<crate::models::intelligence_cross_module::ClassifyRecommendation>(json_str)
                .map_err(|e| AppError::Internal(format!("Failed to parse classification: {}", e)))
        })
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_daily_briefing(
    state: State<'_, AppState>,
) -> Result<ApiResponse<DailyBriefingData>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let generated_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Gather today's data
    let todos = todo_service::get_todos(&state.pool, &today, None)
        .await
        .unwrap_or_default();
    let todo_count = todos.len();
    let todo_completed = todos.iter().filter(|t| t.completed).count();
    let todo_text: Vec<String> = todos.iter().map(|t| {
        format!("- [{}] {} {}",
            if t.completed { "x" } else { " " },
            t.title,
            t.priority.as_str()
        )
    }).collect();

    let journal = journal_service::get_journal(&state.pool, user_id, &today)
        .await
        .unwrap_or(None);
    let journal_content = journal.as_ref().and_then(|j| j.content.clone()).unwrap_or_default();
    let journal_words = journal_content.chars().count();

    let news = news_service::get_news(&state.pool, user_id)
        .await
        .unwrap_or_default();
    let news_count = news.len();
    let news_text: Vec<String> = news.iter().take(10).map(|n| {
        format!("- {} ({})", n.title, n.category.as_deref().unwrap_or("未分类"))
    }).collect();

    let timers = timer_service::get_timers(&state.pool, user_id)
        .await
        .unwrap_or_default();
    let timer_sessions = timers.len();
    let total_focus_seconds = timers.iter().map(|t| t.elapsed).sum::<i64>() / 1000;

    // Build AI prompt
    let prompt = format!(
        "你是一位个人效率分析助手。请根据以下今日数据，生成一份简洁的每日简报（用中文）。\n\n\
        ## 今日待办 ({}/{} 完成)\n{}\n\n\
        ## 今日日志 ({} 字)\n{}\n\n\
        ## 今日新闻 ({} 条)\n{}\n\n\
        ## 计时器会话 ({} 次，总计 {} 秒)\n\n\
        请按以下格式输出简报：\n\n\
        📋 今日待办总结\n\
        (1-2句话总结待办完成情况)\n\n\
        📝 今日日志摘要\n\
        (1-2句话总结日志内容)\n\n\
        📰 今日新闻要点\n\
        (1-2句话总结新闻要点)\n\n\
        ⏱️ 时间使用分析\n\
        (1-2句话分析时间使用)\n\n\
        🎯 明日建议\n\
        (1-2条明日行动建议)\n\n\
        请保持简洁，每个部分不超过2句话。",
        todo_count, todo_completed,
        if todo_text.is_empty() { "无待办事项".to_string() } else { todo_text.join("\n") },
        journal_words,
        if journal_content.is_empty() { "无日志记录".to_string() } else { journal_content[..journal_content.len().min(500)].to_string() },
        news_count,
        if news_text.is_empty() { "无新闻".to_string() } else { news_text.join("\n") },
        timer_sessions, total_focus_seconds,
    );

    let briefing = state
        .intelligence_service
        .query_local_llm(&prompt)
        .await
        .unwrap_or_else(|_| {
            "⚠️ AI 服务暂时不可用，请检查 Ollama 是否运行。\n\n📋 今日待办总结\n暂无数据\n\n📝 今日日志摘要\n暂无日志\n\n📰 今日新闻要点\n暂无新闻\n\n⏱️ 时间使用分析\n暂无计时数据\n\n🎯 明日建议\n请确保 Ollama 服务正常运行后重试生成简报。".to_string()
        });

    Ok(ApiResponse::success(DailyBriefingData {
        date: today,
        briefing,
        todo_count,
        todo_completed,
        journal_words,
        news_count,
        timer_sessions,
        total_focus_seconds,
        generated_at,
    }))
}

/// D2.3 模块渗透：群聊智能建议
///
/// 前端在群聊事件发生时调用（如发送消息、切换会话），传入群聊上下文。
/// 底层智能基于上下文生成「触发执行式」建议（如冷场提醒、辩论触发）。
/// 底层智能关闭后返回空列表（非阻塞，群聊核心功能不受影响）。
///
/// 设计依据：.trae/rules/项目核心设计意图.md §二（底层智能输出是优化建议/触发执行，非对话回复）
#[tauri::command]
pub async fn intelligence_chat_suggest(
    state: State<'_, AppState>,
    context: ChatModuleContext,
) -> Result<ApiResponse<Vec<crate::models::intelligence::Suggestion>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        state
            .intelligence_service
            .generate_chat_suggestions(context)
            .await,
    ))
}

/// D2.3 模块渗透：游戏智能建议
///
/// 前端在游戏事件发生时调用（如突破、建筑、进入游戏），传入游戏上下文。
/// 底层智能基于上下文生成「触发执行式」建议（如修炼提醒、建筑升级提醒）。
/// 底层智能关闭后返回空列表（非阻塞，游戏核心功能不受影响）。
///
/// 设计依据：.trae/rules/项目核心设计意图.md §二（底层智能输出是优化建议/触发执行，非对话回复）
#[tauri::command]
pub async fn intelligence_game_suggest(
    state: State<'_, AppState>,
    context: GameModuleContext,
) -> Result<ApiResponse<Vec<crate::models::intelligence::Suggestion>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        state
            .intelligence_service
            .generate_game_suggestions(context)
            .await,
    ))
}