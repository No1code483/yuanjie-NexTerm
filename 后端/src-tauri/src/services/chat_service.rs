use sqlx::SqlitePool;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::collections::HashMap;
use tauri::Emitter;
use tokio::sync::RwLock;

use crate::crypto::mek_manager::MekManager;
use crate::db::repositories::chat_repo;
use crate::error::app_error::AppError;
use crate::models::ai_model::ConversationWithPreview;
use crate::models::chat::{
    Conversation, ConversationParticipant, Message, ModelHealthStatus,
};
use crate::services::ai_model_service::AiModelService;
use crate::models::chat::{OrchestratorConfig, OrchestratorResult};
use crate::services::ai_orchestrator::AiOrchestrator;
use crate::services::intelligence_v4_service;

// 安全审计修复（多用户隔离批次 2）：所有方法添加 user_id 参数并透传到 chat_repo。
// 原实现：会话与消息无用户隔离，任何登录用户可读取/修改/删除他人的会话历史。
// 现实现：所有读写强制按 user_id 过滤。
// 规范：安全审计报告/2026-07-25-代码安全审计报告.md 附录七

pub async fn create_conversation(
    pool: &SqlitePool,
    user_id: i64,
    title: Option<&str>,
    r#type: &str,
    is_temp: bool,
    token_budget: Option<i64>,
    participants: &[(Option<i64>, Option<i64>, &str)],
) -> Result<Conversation, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let budget = token_budget.unwrap_or(50000);

    let conv = chat_repo::create_conversation(pool, user_id, title, r#type, is_temp, budget, now).await?;

    for (model_id, agent_id, role) in participants {
        chat_repo::add_participant(pool, user_id, conv.id, *model_id, *agent_id, role).await?;
    }

    let _ = intelligence_v4_service::instrument(
        pool, "system", "chat", "create_conversation",
        Some(&format!("title: {:?}", title)),
    ).await;

    Ok(conv)
}

pub async fn get_conversations(
    pool: &SqlitePool,
    user_id: i64,
    r#type: Option<&str>,
) -> Result<Vec<Conversation>, AppError> {
    if let Some(t) = r#type {
        chat_repo::get_conversations_by_type(pool, user_id, t).await
    } else {
        chat_repo::get_conversations(pool, user_id).await
    }
}

// spec ai-chat-enhancement Phase 2 §2.1: 会话列表带最后消息预览
//
// 服务层包装：委托给 chat_repo::get_conversations_with_preview。
// 与 get_conversations 的差异：返回 ConversationWithPreview（含 unread_count /
// sort_order / 最后消息预览三元组），排序改为 sort_order ASC + updated_at DESC。
pub async fn get_conversations_with_preview(
    pool: &SqlitePool,
    user_id: i64,
    r#type: Option<&str>,
) -> Result<Vec<ConversationWithPreview>, AppError> {
    chat_repo::get_conversations_with_preview(pool, user_id, r#type).await
}

pub async fn update_conversation(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
    title: Option<&str>,
    token_budget: Option<i64>,
) -> Result<(), AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    chat_repo::update_conversation(pool, id, user_id, title, token_budget, now).await
}

pub async fn delete_conversation(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
) -> Result<(), AppError> {
    chat_repo::delete_conversation(pool, id, user_id).await
}

pub async fn delete_message(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
) -> Result<(), AppError> {
    chat_repo::delete_message(pool, id, user_id).await
}

pub async fn send_message(
    pool: &SqlitePool,
    conversation_id: i64,
    sender_type: &str,
    sender_id: Option<i64>,
    content: &str,
    user_id: i64,
    mek_manager: &Arc<RwLock<MekManager>>,
    app_handle: &tauri::AppHandle,
) -> Result<Message, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let current_round = chat_repo::get_max_round(pool, conversation_id, user_id).await?;
    let round = if current_round == 0 { 0 } else { current_round + 1 };

    let msg = chat_repo::add_message(
        pool, user_id, conversation_id, sender_type, sender_id, content, round, now,
    )
    .await?;
    chat_repo::update_conversation_timestamp(pool, conversation_id, user_id, now).await?;

    let conv = chat_repo::get_conversation_by_id(pool, conversation_id, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound)?;

    if conv.r#type == "single" && sender_type == "user" {
        let participants = chat_repo::get_participants(pool, conversation_id, user_id).await?;
        let model_participants: Vec<_> = participants
            .iter()
            .filter(|p| p.model_id.is_some())
            .collect();

        if model_participants.is_empty() {
            return Ok(msg);
        }

        if model_participants.len() == 1 {
            // 单模型：异步生成（事件命名与多模型保持一致）
            let participant = model_participants[0].clone();
            if let Some(model_id) = participant.model_id {
                let pool_clone = pool.clone();
                let app_clone = app_handle.clone();
                let mek_clone = mek_manager.clone();
                let conv_id = conversation_id;
                let uid = user_id;
                let user_msg = content.to_string();
                let ai_round = round + 1;

                tokio::spawn(async move {
                    let ai_service = AiModelService::new();
                    let model_config = crate::db::repositories::ai_repo::get_model_by_id(&pool_clone, model_id, uid).await;
                    let system_prompt = match model_config {
                        Ok(Some(m)) => {
                            if let Some(agent_id) = participant.agent_id {
                                crate::db::repositories::ai_repo::get_all_agents(&pool_clone, uid)
                                    .await
                                    .ok()
                                    .and_then(|agents| {
                                        agents
                                            .into_iter()
                                            .find(|a| a.id == agent_id)
                                            .and_then(|a| a.system_prompt.clone())
                                    })
                            } else {
                                m.system_prompt.clone()
                            }
                        }
                        _ => None,
                    };

                    let _ = app_clone.emit(
                        "ai-stream",
                        serde_json::json!({
                            "conversation_id": conv_id,
                            "event": "model_generation_start",
                            "sender_id": participant.id,
                            "model_id": model_id,
                        }),
                    );

                    match ai_service
                        .call_model_stream(
                            &pool_clone,
                            &mek_clone,
                            uid,
                            model_id,
                            &user_msg,
                            system_prompt.as_deref(),
                            &app_clone,
                            conv_id,
                            Some(participant.id),
                        )
                        .await
                    {
                        Ok(response) => {
                            if response.trim().is_empty() {
                                let err_now = chrono::Utc::now().timestamp_millis();
                                let empty_err = "AI 模型返回空响应".to_string();
                                let (user_friendly_msg, suggestion) = classify_ai_error(&empty_err);
                                let display_msg = if suggestion.is_empty() {
                                    user_friendly_msg.clone()
                                } else {
                                    format!("{}\n\n💡 {}", user_friendly_msg, suggestion)
                                };
                                let err_msg = format!("[错误] {}", display_msg);
                                let _ = chat_repo::add_message(
                                    &pool_clone, uid, conv_id, "error", None, &err_msg, ai_round, err_now,
                                ).await;
                                let _ = chat_repo::update_conversation_timestamp(&pool_clone, conv_id, uid, err_now).await;
                                let _ = app_clone.emit(
                                    "ai-stream",
                                    serde_json::json!({
                                        "conversation_id": conv_id,
                                        "event": "model_generation_error",
                                        "error": empty_err,
                                        "user_message": user_friendly_msg,
                                        "suggestion": suggestion,
                                        "error_code": 0,
                                        "category": "empty_response",
                                        "sender_id": participant.id,
                                        "model_id": model_id,
                                    }),
                                );
                            } else {
                                let ai_now = chrono::Utc::now().timestamp_millis();
                                let _ = chat_repo::add_message(
                                    &pool_clone, uid, conv_id, "model", Some(participant.id), &response, ai_round, ai_now,
                                ).await;
                                let _ = chat_repo::update_conversation_timestamp(&pool_clone, conv_id, uid, ai_now).await;
                                let _ = chat_repo::increment_unread_count(&pool_clone, conv_id, uid).await;
                                let _ = app_clone.emit(
                                    "ai-stream",
                                    serde_json::json!({
                                        "conversation_id": conv_id,
                                        "event": "model_generation_complete",
                                        "sender_id": participant.id,
                                        "model_id": model_id,
                                    }),
                                );
                            }
                        }
                        Err(e) => {
                            let err_now = chrono::Utc::now().timestamp_millis();
                            let error_code = e.error_code();
                            let category = e.category().to_string();
                            let raw_msg = e.to_string();
                            let (user_friendly_msg, suggestion) = classify_ai_error(&raw_msg);
                            let display_msg = if suggestion.is_empty() {
                                user_friendly_msg.clone()
                            } else {
                                format!("{}\n\n💡 {}", user_friendly_msg, suggestion)
                            };
                            let err_msg = format!("[错误] {}", display_msg);
                            let _ = chat_repo::add_message(
                                &pool_clone, uid, conv_id, "error", None, &err_msg, ai_round, err_now,
                            ).await;
                            let _ = chat_repo::update_conversation_timestamp(&pool_clone, conv_id, uid, err_now).await;
                            let _ = app_clone.emit(
                                "ai-stream",
                                serde_json::json!({
                                    "conversation_id": conv_id,
                                    "event": "model_generation_error",
                                    "error": raw_msg,
                                    "user_message": user_friendly_msg,
                                    "suggestion": suggestion,
                                    "error_code": error_code,
                                    "category": category,
                                    "sender_id": participant.id,
                                    "model_id": model_id,
                                }),
                            );
                        }
                    }
                });
            }
        } else {
            // 多模型并行生成
            let pool_clone = pool.clone();
            let app_clone = app_handle.clone();
            let mek_clone = mek_manager.clone();
            let conv_id = conversation_id;
            let uid = user_id;
            let user_msg = content.to_string();
            let ai_round = round + 1;
            let participants_clone: Vec<ConversationParticipant> = model_participants
                .iter()
                .map(|p| (*p).clone())
                .collect();

            let _ = app_handle.emit(
                "ai-stream",
                serde_json::json!({
                    "conversation_id": conv_id,
                    "event": "generation_start",
                    "multi_model": true,
                    "total_models": participants_clone.len(),
                }),
            );

            tokio::spawn(async move {
                let mut handles = Vec::new();

                for participant in &participants_clone {
                    if let Some(model_id) = participant.model_id {
                        let pool = pool_clone.clone();
                        let app = app_clone.clone();
                        let mek = mek_clone.clone();
                        let user_msg = user_msg.clone();
                        let participant = participant.clone();

                        let handle = tokio::spawn(async move {
                            let ai_service = AiModelService::new();
                            let model_config = crate::db::repositories::ai_repo::get_model_by_id(&pool, model_id, uid).await;
                            let system_prompt = match model_config {
                                Ok(Some(m)) => m.system_prompt.clone(),
                                _ => None,
                            };

                            let _ = app.emit(
                                "ai-stream",
                                serde_json::json!({
                                    "conversation_id": conv_id,
                                    "event": "model_generation_start",
                                    "sender_id": participant.id,
                                    "model_id": model_id,
                                }),
                            );

                            match ai_service
                                .call_model_stream(
                                    &pool,
                                    &mek,
                                    uid,
                                    model_id,
                                    &user_msg,
                                    system_prompt.as_deref(),
                                    &app,
                                    conv_id,
                                    Some(participant.id),
                                )
                                .await
                            {
                                Ok(response) => {
                                    if response.trim().is_empty() {
                                        let err_now = chrono::Utc::now().timestamp_millis();
                                        let err_msg = "[错误] AI 模型返回空响应".to_string();
                                        let _ = chat_repo::add_message(
                                            &pool, uid, conv_id, "error", Some(participant.id), &err_msg, ai_round, err_now,
                                        ).await;
                                        let _ = app.emit(
                                            "ai-stream",
                                            serde_json::json!({
                                                "conversation_id": conv_id,
                                                "event": "model_generation_error",
                                                "sender_id": participant.id,
                                                "model_id": model_id,
                                                "error": "empty_response",
                                            }),
                                        );
                                    } else {
                                        let ai_now = chrono::Utc::now().timestamp_millis();
                                        let _ = chat_repo::add_message(
                                            &pool, uid, conv_id, "model", Some(participant.id), &response, ai_round, ai_now,
                                        ).await;
                                        let _ = chat_repo::update_conversation_timestamp(&pool, conv_id, uid, ai_now).await;
                                        let _ = chat_repo::increment_unread_count(&pool, conv_id, uid).await;
                                        let _ = app.emit(
                                            "ai-stream",
                                            serde_json::json!({
                                                "conversation_id": conv_id,
                                                "event": "model_generation_complete",
                                                "sender_id": participant.id,
                                                "model_id": model_id,
                                            }),
                                        );
                                    }
                                }
                                Err(e) => {
                                    let err_now = chrono::Utc::now().timestamp_millis();
                                    let err_msg = format!("[错误] {}", e);
                                    let _ = chat_repo::add_message(
                                        &pool, uid, conv_id, "error", Some(participant.id), &err_msg, ai_round, err_now,
                                    ).await;
                                    let _ = app.emit(
                                        "ai-stream",
                                        serde_json::json!({
                                            "conversation_id": conv_id,
                                            "event": "model_generation_error",
                                            "sender_id": participant.id,
                                            "model_id": model_id,
                                            "error": e.to_string(),
                                        }),
                                    );
                                }
                            }
                        });
                        handles.push(handle);
                    }
                }

                for handle in handles {
                    let _ = handle.await;
                }

                let _ = app_clone.emit(
                    "ai-stream",
                    serde_json::json!({
                        "conversation_id": conv_id,
                        "event": "generation_complete",
                        "multi_model": true,
                    }),
                );
            });
        }
    } else if conv.r#type == "group" && sender_type == "user" {
        let pool_clone = pool.clone();
        let app_clone = app_handle.clone();
        let mek_clone = mek_manager.clone();
        let conv_id = conversation_id;
        let uid = user_id;
        let user_msg = content.to_string();

        let _ = app_handle.emit(
            "ai-stream",
            serde_json::json!({
                "conversation_id": conv_id,
                "event": "generation_start",
            }),
        );

        tokio::spawn(async move {
            let config = OrchestratorConfig::default();
            let force_stop = Arc::new(AtomicBool::new(false));
            match run_orchestrator(
                &pool_clone, conv_id, uid, &user_msg, &config,
                &app_clone, &mek_clone, force_stop,
            ).await {
                Ok(_) => {
                    let _ = app_clone.emit(
                        "ai-stream",
                        serde_json::json!({
                            "conversation_id": conv_id,
                            "event": "generation_complete",
                        }),
                    );
                }
                Err(e) => {
                    let err_now = chrono::Utc::now().timestamp_millis();
                    let raw_msg = e.to_string();
                    let (user_friendly_msg, suggestion) = classify_ai_error(&raw_msg);
                    let display_msg = if suggestion.is_empty() {
                        user_friendly_msg.clone()
                    } else {
                        format!("{}\n\n💡 {}", user_friendly_msg, suggestion)
                    };
                    let err_msg = format!("[错误] {}", display_msg);

                    let _ = chat_repo::add_message(
                        &pool_clone, uid, conv_id, "error", None, &err_msg, 1, err_now,
                    ).await;
                    let _ = chat_repo::update_conversation_timestamp(
                        &pool_clone, conv_id, uid, err_now,
                    ).await;

                    let _ = app_clone.emit(
                        "ai-stream",
                        serde_json::json!({
                            "conversation_id": conv_id,
                            "event": "generation_error",
                            "error": raw_msg,
                            "user_message": user_friendly_msg,
                            "suggestion": suggestion,
                            "error_code": 0,
                            "category": "group_chat_error",
                        }),
                    );
                }
            }
        });

        return Ok(msg);
    }

    let _ = intelligence_v4_service::instrument(
        pool, "system", "chat", "send_message",
        Some(&format!("conv_id: {}", conversation_id)),
    ).await;

    Ok(msg)
}

pub async fn get_messages(
    pool: &SqlitePool,
    conversation_id: i64,
    user_id: i64,
) -> Result<Vec<Message>, AppError> {
    chat_repo::get_messages(pool, conversation_id, user_id).await
}

pub async fn get_participants(
    pool: &SqlitePool,
    conversation_id: i64,
    user_id: i64,
) -> Result<Vec<ConversationParticipant>, AppError> {
    chat_repo::get_participants(pool, conversation_id, user_id).await
}

pub async fn run_orchestrator(
    pool: &SqlitePool,
    conversation_id: i64,
    user_id: i64,
    user_message: &str,
    config: &OrchestratorConfig,
    app_handle: &tauri::AppHandle,
    mek_manager: &Arc<RwLock<MekManager>>,
    force_stop: Arc<AtomicBool>,
) -> Result<OrchestratorResult, AppError> {
    AiOrchestrator::run_group_chat(
        pool,
        conversation_id,
        user_id,
        user_message,
        config,
        app_handle,
        mek_manager,
        force_stop,
    )
    .await
}

pub async fn stop_generation(
    _pool: &SqlitePool,
    conversation_id: i64,
    force_stop_flags: &Arc<RwLock<HashMap<i64, Arc<AtomicBool>>>>,
) -> Result<(), AppError> {
    if let Some(flag) = force_stop_flags.read().await.get(&conversation_id) {
        flag.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    tracing::info!(conversation_id, "停止生成请求");
    Ok(())
}

pub async fn check_model_health(
    pool: &SqlitePool,
    model_id: i64,
    user_id: i64,
    mek_manager: &Arc<RwLock<MekManager>>,
) -> Result<ModelHealthStatus, AppError> {
    let model = crate::db::repositories::ai_repo::get_model_by_id(pool, model_id, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound)?;

    // 检测前先标记 status='checking'（避免重复检测 + 让前端能立即看到检测中状态）
    let now = chrono::Utc::now().timestamp_millis();
    crate::db::repositories::ai_repo::update_model_health(
        pool,
        model_id,
        "checking",
        None,
        now,
    )
    .await?;

    let ai_service = AiModelService::new();
    let start = std::time::Instant::now();

    let is_online = match ai_service
        .call_model(pool, mek_manager, user_id, model_id, "ping", None)
        .await
    {
        Ok(resp) => resp.len() > 0,
        Err(_) => false,
    };

    let latency_ms = if is_online {
        Some(start.elapsed().as_millis() as i64)
    } else {
        None
    };

    // 检测后持久化结果到数据库
    let final_status = if is_online { "online" } else { "offline" };
    let now_after = chrono::Utc::now().timestamp_millis();
    crate::db::repositories::ai_repo::update_model_health(
        pool,
        model_id,
        final_status,
        latency_ms,
        now_after,
    )
    .await?;

    Ok(ModelHealthStatus {
        model_id: model.id,
        name: model.name,
        provider: model.provider,
        is_online,
        latency_ms,
    })
}

// spec ai-chat-enhancement Phase 2 §3.2: 标记会话已读（unread_count 清零）
pub async fn mark_conversation_read(
    pool: &SqlitePool,
    conversation_id: i64,
    user_id: i64,
) -> Result<(), AppError> {
    crate::db::repositories::chat_repo::mark_conversation_read(pool, conversation_id, user_id).await
}

// spec ai-chat-enhancement Phase 2 §3.3: 批量更新会话排序（拖拽自定义排序）
pub async fn reorder_conversations(
    pool: &SqlitePool,
    user_id: i64,
    items: Vec<crate::models::ai_model::ReorderItem>,
) -> Result<(), AppError> {
    crate::db::repositories::chat_repo::reorder_conversations(pool, user_id, &items).await
}

/// Classifies AI errors into user-friendly messages and suggestions
fn classify_ai_error(raw_msg: &str) -> (String, String) {
    let lower = raw_msg.to_lowercase();

    if lower.contains("空响应") || lower.contains("empty response") || lower.contains("返回空") {
        (
            "AI 模型无响应 (空响应)".to_string(),
            "模型返回了空内容，可能原因：1) API 未授权（401/403 被静默处理）2) API 地址不正确 3) 模型名称错误 4) 服务商线路问题。请检查模型配置或更换 API 提供商。".to_string(),
        )
    } else if lower.contains("401") || lower.contains("403") || lower.contains("unauthorized")
        || lower.contains("forbidden") || lower.contains("invalid api key")
        || lower.contains("authentication") {
        (
            "API 授权失败 (401/403)".to_string(),
            "请检查 API Key 是否正确，或确认该 API 服务商是否对当前来源授权。部分公益/第三方 API 仅限特定平台使用。".to_string(),
        )
    } else if lower.contains("404") || lower.contains("not found") {
        (
            "模型不存在 (404)".to_string(),
            "请检查模型 ID (model_name) 是否正确，确认该服务商是否提供此模型。".to_string(),
        )
    } else if lower.contains("timeout") || lower.contains("timed out") || lower.contains("deadline") {
        (
            "请求超时".to_string(),
            "网络连接超时，可能是网络不稳定、服务端响应慢或防火墙限制。请检查网络后重试。".to_string(),
        )
    } else if lower.contains("connection") || lower.contains("refused")
        || lower.contains("network") || lower.contains("dns")
        || lower.contains("reset") || lower.contains("broken pipe") {
        (
            "网络连接失败".to_string(),
            "无法连接到 API 服务器。请检查：1) 网络是否正常 2) API 地址是否正确 3) 是否需要代理 4) 服务商是否在线。".to_string(),
        )
    } else if lower.contains("rate limit") || lower.contains("429") || lower.contains("too many requests") {
        (
            "请求频率超限 (429)".to_string(),
            "API 调用过于频繁，请稍后重试。如需提高限额，请联系 API 服务商升级套餐。".to_string(),
        )
    } else if lower.contains("insufficient") || lower.contains("quota") || lower.contains("billing")
        || lower.contains("balance") || lower.contains("payment") {
        (
            "余额不足或配额用尽".to_string(),
            "API 账户余额不足或配额已耗尽，请到 API 服务商后台充值或查看使用情况。".to_string(),
        )
    } else if lower.contains("context length") || lower.contains("token limit")
        || lower.contains("maximum context") {
        (
            "超出上下文长度限制".to_string(),
            "消息内容过长，超过模型的上下文窗口限制。请在高级配置中增大上下文窗口值，或缩短对话内容。".to_string(),
        )
    } else {
        (
            format!("AI 调用失败: {}", raw_msg),
            String::new(),
        )
    }
}

// ===== 搜索 =====

pub async fn search_conversations(
    pool: &SqlitePool,
    user_id: i64,
    query: &str,
) -> Result<Vec<crate::db::repositories::chat_repo::ConversationSearchResult>, AppError> {
    chat_repo::search_conversations(pool, user_id, query).await
}

// ===== 星标 =====

pub async fn toggle_star_conversation(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
) -> Result<bool, AppError> {
    chat_repo::toggle_star_conversation(pool, id, user_id).await
}

// ===== 分支 =====

pub async fn branch_conversation(
    pool: &SqlitePool,
    user_id: i64,
    convoy_id: i64,
    message_id: i64,
) -> Result<Conversation, AppError> {
    let source = chat_repo::get_conversation_by_id(pool, convoy_id, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound)?;
    let now = chrono::Utc::now().timestamp_millis();
    let title = source.title.as_deref().unwrap_or("未命名");
    let new_title = format!("{} (分支)", title);
    let new_conv = chat_repo::create_conversation(
        pool,
        user_id,
        Some(&new_title),
        &source.r#type,
        source.is_temp,
        source.token_budget,
        now,
    )
    .await?;
    // 复制参与者
    let parts = chat_repo::get_participants(pool, convoy_id, user_id).await?;
    for p in &parts {
        chat_repo::add_participant(pool, user_id, new_conv.id, p.model_id, p.agent_id, &p.role).await?;
    }
    // 复制消息到分支点
    chat_repo::copy_messages_to_conversation(pool, user_id, convoy_id, new_conv.id, message_id, now).await?;
    Ok(new_conv)
}

// ===== Prompt 模板 =====
// 注：prompt_templates 为公共模板表，无 user_id 隔离需求，保持原样。

pub async fn prompt_template_list(
    pool: &SqlitePool,
) -> Result<Vec<crate::models::chat::PromptTemplate>, AppError> {
    chat_repo::get_prompt_templates(pool).await
}

pub async fn prompt_template_create(
    pool: &SqlitePool,
    title: &str,
    category: &str,
    content: &str,
) -> Result<crate::models::chat::PromptTemplate, AppError> {
    chat_repo::create_prompt_template(pool, title, category, content).await
}

pub async fn prompt_template_update(
    pool: &SqlitePool,
    id: i64,
    title: Option<&str>,
    category: Option<&str>,
    content: Option<&str>,
) -> Result<(), AppError> {
    chat_repo::update_prompt_template(pool, id, title, category, content).await
}

pub async fn prompt_template_delete(pool: &SqlitePool, id: i64) -> Result<(), AppError> {
    chat_repo::delete_prompt_template(pool, id).await
}

// ===== 导出 =====

pub async fn export_conversation(
    pool: &SqlitePool,
    conversation_id: i64,
    user_id: i64,
    format: &str,
) -> Result<String, AppError> {
    let conv = chat_repo::get_conversation_by_id(pool, conversation_id, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound)?;
    let msgs = chat_repo::get_messages(pool, conversation_id, user_id).await?;

    match format {
        "json" => {
            let export = serde_json::json!({
                "title": conv.title,
                "type": conv.r#type,
                "created_at": conv.created_at,
                "messages": msgs.iter().map(|m| serde_json::json!({
                    "role": m.sender_type,
                    "content": m.content,
                    "created_at": m.created_at,
                })).collect::<Vec<_>>(),
            });
            Ok(serde_json::to_string_pretty(&export).unwrap_or_default())
        }
        _ => {
            let mut md = String::new();
            md.push_str(&format!("# {}\n\n", conv.title.as_deref().unwrap_or("未命名会话")));
            for m in &msgs {
                let role_label = match m.sender_type.as_str() {
                    "user" => "**用户**",
                    "model" => "**AI**",
                    "error" => "**错误**",
                    _ => &m.sender_type,
                };
                md.push_str(&format!("{}\n\n{}\n\n---\n\n", role_label, m.content));
            }
            Ok(md)
        }
    }
}
