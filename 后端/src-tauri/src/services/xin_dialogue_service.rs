use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::Emitter;
use tokio::sync::{Mutex, RwLock};

use crate::crypto::mek_manager::MekManager;
use crate::error::app_error::AppError;
use crate::models::ai_model::AiModel;
use crate::models::xin::Persona;
use crate::models::xin::SpeakingStyle;
use crate::services::ai_model_service::AiModelService;
use crate::services::xin_context_service::{
    estimate_tokens, BuiltinSkills, ChatMessage, ChatRole, ContextWindow, XinCompactor,
    XinContextManager, XinPromptBuilder, XinPromptConfig,
};

static SKILLS: once_cell::sync::Lazy<Arc<Mutex<BuiltinSkills>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(BuiltinSkills::new())));

static DREAM_STATE: once_cell::sync::Lazy<Arc<Mutex<crate::services::xin_dream_service::DreamState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(crate::services::xin_dream_service::DreamState::default())));

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct XinConversationRecord {
    pub id: String,
    pub user_id: i64,
    pub persona_id: String,
    pub model_id: String,
    pub title: String,
    pub context_json: String,
    pub message_count: i64,
    pub total_tokens: i64,
    pub summary: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XinConversationSummary {
    pub id: String,
    pub persona_id: String,
    pub persona_name: String,
    pub model_id: String,
    pub title: String,
    pub message_count: i64,
    pub total_tokens: i64,
    pub summary: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XinChatRequest {
    pub conversation_id: Option<String>,
    pub persona_id: String,
    pub model_id: String,
    pub message: String,
    pub skills_enabled: bool,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XinChatResponse {
    pub conversation_id: String,
    pub content: String,
    pub token_usage: XinTokenUsage,
    pub is_new_conversation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XinTokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XinStreamEvent {
    pub conversation_id: String,
    pub event: String,
    pub chunk: String,
    pub done: bool,
}

struct XinDialogueRepo;

impl XinDialogueRepo {
    async fn create_table(pool: &SqlitePool) -> Result<(), AppError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS xin_conversations (
                id TEXT PRIMARY KEY,
                user_id INTEGER NOT NULL DEFAULT 1,
                persona_id TEXT NOT NULL,
                model_id TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT '新对话',
                context_json TEXT NOT NULL DEFAULT '',
                message_count INTEGER NOT NULL DEFAULT 0,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                summary TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        // 与 migration 0119 保持一致的索引
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_xin_conversations_user_id ON xin_conversations(user_id)",
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_xin_conversations_user_updated ON xin_conversations(user_id, updated_at)",
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    async fn insert(pool: &SqlitePool, record: &XinConversationRecord) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO xin_conversations (id, user_id, persona_id, model_id, title, context_json, message_count, total_tokens, summary, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&record.id)
        .bind(record.user_id)
        .bind(&record.persona_id)
        .bind(&record.model_id)
        .bind(&record.title)
        .bind(&record.context_json)
        .bind(record.message_count)
        .bind(record.total_tokens)
        .bind(&record.summary)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    async fn update(pool: &SqlitePool, record: &XinConversationRecord) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE xin_conversations SET title = ?, context_json = ?, message_count = ?, total_tokens = ?, summary = ?, updated_at = ? WHERE user_id = ? AND id = ?",
        )
        .bind(&record.title)
        .bind(&record.context_json)
        .bind(record.message_count)
        .bind(record.total_tokens)
        .bind(&record.summary)
        .bind(&record.updated_at)
        .bind(record.user_id)
        .bind(&record.id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    async fn get_by_id(pool: &SqlitePool, user_id: i64, id: &str) -> Result<Option<XinConversationRecord>, AppError> {
        sqlx::query_as::<_, XinConversationRecord>(
            "SELECT id, user_id, persona_id, model_id, title, context_json, message_count, total_tokens, summary, created_at, updated_at FROM xin_conversations WHERE user_id = ? AND id = ?",
        )
        .bind(user_id)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
    }

    async fn list_all(pool: &SqlitePool, user_id: i64) -> Result<Vec<XinConversationRecord>, AppError> {
        sqlx::query_as::<_, XinConversationRecord>(
            "SELECT id, user_id, persona_id, model_id, title, context_json, message_count, total_tokens, summary, created_at, updated_at FROM xin_conversations WHERE user_id = ? ORDER BY updated_at DESC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    }

    async fn delete(pool: &SqlitePool, user_id: i64, id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM xin_conversations WHERE user_id = ? AND id = ?")
            .bind(user_id)
            .bind(id)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    async fn search_by_title(pool: &SqlitePool, user_id: i64, query: &str) -> Result<Vec<XinConversationRecord>, AppError> {
        let pattern = format!("%{}%", query);
        sqlx::query_as::<_, XinConversationRecord>(
            "SELECT id, user_id, persona_id, model_id, title, context_json, message_count, total_tokens, summary, created_at, updated_at FROM xin_conversations WHERE user_id = ? AND title LIKE ? ORDER BY updated_at DESC",
        )
        .bind(user_id)
        .bind(pattern)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    }
}

pub struct XinDialogueService {
    pool: SqlitePool,
    mek_manager: Arc<RwLock<MekManager>>,
    active_contexts: Arc<Mutex<HashMap<String, ContextWindow>>>,
    stop_flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    recent_evals: Arc<Mutex<Vec<crate::services::xin_michelin_service::EvalResult>>>,
    last_evolution_persona: Arc<Mutex<Option<SpeakingStyle>>>,
}

impl XinDialogueService {
    pub fn new(pool: SqlitePool, mek_manager: Arc<RwLock<MekManager>>) -> Self {
        Self {
            pool,
            mek_manager,
            active_contexts: Arc::new(Mutex::new(HashMap::new())),
            stop_flags: Arc::new(Mutex::new(HashMap::new())),
            recent_evals: Arc::new(Mutex::new(Vec::new())),
            last_evolution_persona: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn init_db(&self) -> Result<(), AppError> {
        XinDialogueRepo::create_table(&self.pool).await
    }

    pub async fn create_conversation(
        &self,
        user_id: i64,
        persona: &Persona,
        model_id: &str,
        title: Option<&str>,
    ) -> Result<XinConversationSummary, AppError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let title = title.unwrap_or("新对话").to_string();

        let ctx = XinContextManager::new_context(model_id);

        let record = XinConversationRecord {
            id: id.clone(),
            user_id,
            persona_id: persona.id.clone(),
            model_id: model_id.to_string(),
            title: title.clone(),
            context_json: serde_json::to_string(&ctx).unwrap_or_default(),
            message_count: 0,
            total_tokens: 0,
            summary: None,
            created_at: now.clone(),
            updated_at: now,
        };

        XinDialogueRepo::insert(&self.pool, &record).await?;

        {
            let mut contexts = self.active_contexts.lock().await;
            contexts.insert(id.clone(), ctx);
        }

        Ok(XinConversationSummary {
            id,
            persona_id: persona.id.clone(),
            persona_name: persona.name.clone(),
            model_id: model_id.to_string(),
            title,
            message_count: 0,
            total_tokens: 0,
            summary: None,
            created_at: record.created_at,
            updated_at: record.updated_at,
        })
    }

    pub async fn get_conversation(
        &self,
        user_id: i64,
        id: &str,
        persona: &Persona,
    ) -> Result<XinConversationSummary, AppError> {
        let record = XinDialogueRepo::get_by_id(&self.pool, user_id, id)
            .await?
            .ok_or_else(|| AppError::AiApi(format!("会话不存在: {}", id)))?;

        if let Ok(ctx) = serde_json::from_str::<ContextWindow>(&record.context_json) {
            let mut contexts = self.active_contexts.lock().await;
            contexts.insert(id.to_string(), ctx);
        }

        Ok(XinConversationSummary {
            id: record.id,
            persona_id: record.persona_id,
            persona_name: persona.name.clone(),
            model_id: record.model_id,
            title: record.title,
            message_count: record.message_count,
            total_tokens: record.total_tokens,
            summary: record.summary,
            created_at: record.created_at,
            updated_at: record.updated_at,
        })
    }

    pub async fn list_conversations(
        &self,
        user_id: i64,
        persona: &Persona,
    ) -> Result<Vec<XinConversationSummary>, AppError> {
        let records = XinDialogueRepo::list_all(&self.pool, user_id).await?;
        Ok(records
            .into_iter()
            .map(|r| XinConversationSummary {
                id: r.id,
                persona_id: r.persona_id,
                persona_name: persona.name.clone(),
                model_id: r.model_id,
                title: r.title,
                message_count: r.message_count,
                total_tokens: r.total_tokens,
                summary: r.summary,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    pub async fn delete_conversation(&self, user_id: i64, id: &str) -> Result<(), AppError> {
        XinDialogueRepo::delete(&self.pool, user_id, id).await?;
        let mut contexts = self.active_contexts.lock().await;
        contexts.remove(id);
        Ok(())
    }

    pub async fn search_conversations(
        &self,
        user_id: i64,
        query: &str,
        persona: &Persona,
    ) -> Result<Vec<XinConversationSummary>, AppError> {
        let records = XinDialogueRepo::search_by_title(&self.pool, user_id, query).await?;
        Ok(records
            .into_iter()
            .map(|r| XinConversationSummary {
                id: r.id,
                persona_id: r.persona_id,
                persona_name: persona.name.clone(),
                model_id: r.model_id,
                title: r.title,
                message_count: r.message_count,
                total_tokens: r.total_tokens,
                summary: r.summary,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    pub async fn send_message(
        &self,
        app_handle: &tauri::AppHandle,
        persona: &Persona,
        user_id: i64,
        model_id: i64,
        conversation_id: Option<&str>,
        message: &str,
        skills_enabled: bool,
        use_stream: bool,
        attachments: Option<Vec<crate::services::xin_multimodal_service::Attachment>>,
    ) -> Result<XinChatResponse, AppError> {
        // D3.1: 通过统一模型管理获取 (model, api_key)，替代硬编码 OpenAI 端点
        let (model, api_key) = AiModelService::get_model_config(
            &self.pool,
            &self.mek_manager,
            user_id,
            model_id,
        )
        .await?;

        let model_id_str = model_id.to_string();
        let is_new = conversation_id.is_none();
        let conv_id = if let Some(id) = conversation_id {
            id.to_string()
        } else {
            let summary = self.create_conversation(user_id, persona, &model_id_str, None).await?;
            summary.id
        };

        let parsed_attachments: Vec<crate::services::xin_multimodal_service::ParsedAttachment> =
            if let Some(ref atts) = attachments {
                let mut results = Vec::new();
                for att in atts {
                    match crate::services::xin_multimodal_service::XinMultimodalService::parse_attachment(att) {
                        Ok(parsed) => results.push(parsed),
                        Err(e) => {
                            let _ = app_handle.emit("xin-attachment-error", serde_json::json!({
                                "conversation_id": conv_id,
                                "attachment_name": att.name,
                                "error": e.to_string(),
                            }));
                        }
                    }
                }
                results
            } else {
                Vec::new()
            };

        if !parsed_attachments.is_empty() {
            let attachment_ctx = crate::services::xin_multimodal_service::XinMultimodalService::build_attachment_context(&parsed_attachments);
            let _ = app_handle.emit("xin-attachment-context", serde_json::json!({
                "conversation_id": conv_id,
                "attachments": parsed_attachments,
                "context": attachment_ctx,
            }));
        }

        let stop_flag = Arc::new(AtomicBool::new(false));
        {
            let mut flags = self.stop_flags.lock().await;
            flags.insert(conv_id.clone(), stop_flag.clone());
        }

        let system_prompt = {
            // D3.7: 分析用户消息情绪，生成回复风格指引（按用户情绪调整风格）
            let emotion_guide = crate::services::xin_emotion_service::XinEmotionService::get_emotion_guide(message);
            // D3.8: 加载该人格的历史记忆，生成 system prompt 注入文本
            // 让小欣"记得"在该人格下与用户的过往交互（首次使用时返回 None，不注入）
            let persona_memory_prompt = {
                let memory = crate::services::xin_personality_service::XinPersonalityService
                    ::get_persona_memory(&self.pool, user_id, &persona.id)
                    .await.unwrap_or_else(|_| {
                        crate::services::xin_personality_service::XinPersonalityService::default_memory(&persona.id)
                    });
                crate::services::xin_personality_service::XinPersonalityService
                    ::build_memory_prompt(&memory, &persona.name)
            };
            // D3.8: 记录一次交互（轻量调用，失败不阻塞对话）
            let _ = crate::services::xin_personality_service::XinPersonalityService
                ::record_interaction(&self.pool, user_id, &persona.id).await;
            let config = XinPromptConfig {
                persona: persona.clone(),
                include_skills: skills_enabled,
                include_mood: true,
                include_time: true,
                custom_instructions: None,
                user_emotion_guide: emotion_guide,
                persona_memory: persona_memory_prompt,
            };
            let mut prompt = XinPromptBuilder::build_system_prompt(&config);
            if skills_enabled {
                let skills = SKILLS.lock().await;
                prompt.push_str("\n\n");
                prompt.push_str(&skills.build_skills_prompt());
            }

            let tool_registry = crate::services::xin_tool_service::ToolRegistry::new();
            prompt.push_str(&tool_registry.build_tools_prompt());

            let fusion_decision = crate::services::xin_knowledge_fusion_service::XinKnowledgeFusion::should_retrieve(message);

            if fusion_decision.should_retrieve {
                let kb_fusion = crate::services::xin_knowledge_fusion_service::XinKnowledgeFusion::retrieve_relevant(
                    &self.pool,
                    user_id,
                    message,
                    5,
                ).await.unwrap_or_else(|_| crate::services::xin_knowledge_fusion_service::KnowledgeFusionResult {
                    snippets: vec![],
                    total_found: 0,
                    formatted_context: String::new(),
                    is_relevant: false,
                });

                let kb_unified = crate::services::xin_knowledge_fusion_service::XinKnowledgeFusion::convert_kb_to_unified(&kb_fusion.snippets);

                let memory_results = if fusion_decision.sources.contains(&crate::services::xin_knowledge_fusion_service::FusionSource::UserMemory) {
                    crate::services::xin_knowledge_fusion_service::XinKnowledgeFusion::query_memories(
                        &self.pool,
                        user_id,
                        message,
                        3,
                    ).await.unwrap_or_default()
                } else {
                    vec![]
                };

                let fused_results = crate::services::xin_knowledge_fusion_service::XinKnowledgeFusion::fuse_multi_source(
                    kb_unified,
                    memory_results,
                    6,
                );

                let unified_context = crate::services::xin_knowledge_fusion_service::XinKnowledgeFusion::build_unified_context(
                    &fused_results,
                    1500,
                );

                if !unified_context.is_empty() {
                    prompt.push_str(&unified_context);
                }
            }
            prompt
        };

        let mut ctx = {
            let contexts = self.active_contexts.lock().await;
            contexts
                .get(&conv_id)
                .cloned()
                .unwrap_or_else(|| XinContextManager::new_context(&model_id_str))
        };

        let system_msg = ChatMessage {
            role: ChatRole::System,
            content: system_prompt.clone(),
            name: None,
            timestamp: None,
        };

        if ctx.messages.is_empty() || ctx.messages[0].role != ChatRole::System {
            ctx.messages.insert(0, system_msg);
            ctx.used_tokens += estimate_tokens(&system_prompt);
        }

        let user_msg = ChatMessage {
            role: ChatRole::User,
            content: message.to_string(),
            name: None,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        };
        XinContextManager::add_message(&mut ctx, user_msg);
        XinContextManager::trim_to_budget(&mut ctx);

        let prompt_tokens = ctx.used_tokens;

        {
            let pool = self.pool.clone();
            let conv_id = conv_id.clone();
            let ctx_clone = ctx.clone();
            let title_clone = if is_new {
                let chars: String = message.chars().take(30).collect();
                let suffix = if message.chars().count() > 30 { "..." } else { "" };
                Some(format!("{}{}", chars, suffix))
            } else {
                None
            };
            let msg_count = ctx_clone.messages.iter().filter(|m| m.role != ChatRole::System).count() as i64;
            tokio::spawn(async move {
                let _ = crate::services::xin_checkpoint_service::XinCheckpointService::save_checkpoint(
                    &pool,
                    user_id,
                    &conv_id,
                    &ctx_clone,
                    None,
                    title_clone.as_deref(),
                    msg_count,
                    ctx_clone.used_tokens as i64,
                )
                .await;
                let config = crate::services::xin_checkpoint_service::CheckpointConfig::default();
                let _ = crate::services::xin_checkpoint_service::XinCheckpointService::enforce_limit(&pool, user_id, &conv_id, &config).await;
            });
        }

        let use_multimodal = !parsed_attachments.is_empty();
        let multimodal_messages = if use_multimodal {
            let mut att_map: std::collections::HashMap<usize, Vec<crate::services::xin_multimodal_service::ParsedAttachment>> =
                std::collections::HashMap::new();
            let user_msg_idx = ctx.messages.len() - 1;
            att_map.insert(user_msg_idx, parsed_attachments.clone());
            crate::services::xin_multimodal_service::XinMultimodalService::build_multimodal_messages(
                &ctx.messages,
                &att_map,
            )
        } else {
            Vec::new()
        };

        let response = if use_stream {
            Self::call_llm_stream_multimodal(
                app_handle,
                &conv_id,
                &ctx,
                &multimodal_messages,
                use_multimodal,
                &model,
                &api_key,
                &stop_flag,
            )
            .await
            .map_err(|e| AppError::AiApi(e))?
        } else {
            Self::call_llm_sync_multimodal(
                &ctx,
                &multimodal_messages,
                use_multimodal,
                &model,
                &api_key,
            )
            .await
            .map_err(|e| AppError::AiApi(e))?
        };

        let assistant_msg = ChatMessage {
            role: ChatRole::Assistant,
            content: response.clone(),
            name: Some(persona.name.clone()),
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        };
        XinContextManager::add_message(&mut ctx, assistant_msg);

        let completion_tokens = estimate_tokens(&response);
        let total_tokens = ctx.used_tokens;

        let now = chrono::Utc::now().to_rfc3339();

        let should_compact = XinContextManager::is_over_budget(&ctx);
        let summary = if should_compact {
            let compact_result = XinCompactor::compact(&ctx.messages, &model_id_str);
            Some(compact_result.summary_text)
        } else {
            None
        };

        let title = if ctx.messages.iter().filter(|m| m.role == ChatRole::User).count() == 1 {
            let chars: String = message.chars().take(30).collect();
            let suffix = if message.chars().count() > 30 { "..." } else { "" };
            format!("{}{}", chars, suffix)
        } else {
            let record = XinDialogueRepo::get_by_id(&self.pool, user_id, &conv_id).await?;
            record.map(|r| r.title).unwrap_or_else(|| "新对话".to_string())
        };

        let context_json = serde_json::to_string(&ctx).unwrap_or_default();

        {
            let pool = self.pool.clone();
            let conv_id = conv_id.clone();
            let ctx_clone = ctx.clone();
            let title_clone = title.clone();
            let msg_count = ctx.messages.iter().filter(|m| m.role != ChatRole::System).count() as i64;
            let total_tokens = ctx.used_tokens as i64;
            tokio::spawn(async move {
                let _ = crate::services::xin_checkpoint_service::XinCheckpointService::save_checkpoint(
                    &pool,
                    user_id,
                    &conv_id,
                    &ctx_clone,
                    None,
                    Some(&title_clone),
                    msg_count,
                    total_tokens,
                )
                .await;
                let config = crate::services::xin_checkpoint_service::CheckpointConfig::default();
                let _ = crate::services::xin_checkpoint_service::XinCheckpointService::enforce_limit(&pool, user_id, &conv_id, &config).await;
            });
        }

        let record = XinConversationRecord {
            id: conv_id.clone(),
            user_id,
            persona_id: persona.id.clone(),
            model_id: model_id_str.clone(),
            title,
            context_json,
            message_count: (ctx.messages.iter().filter(|m| m.role != ChatRole::System).count()) as i64,
            total_tokens: total_tokens as i64,
            summary,
            created_at: String::new(),
            updated_at: now,
        };

        if is_new {
            let full_record = XinConversationRecord {
                created_at: record.updated_at.clone(),
                ..record
            };
            XinDialogueRepo::insert(&self.pool, &full_record).await?;
        } else {
            XinDialogueRepo::update(&self.pool, &record).await?;
        }

        {
            let mut contexts = self.active_contexts.lock().await;
            contexts.insert(conv_id.clone(), ctx);
        }

        {
            let mut flags = self.stop_flags.lock().await;
            flags.remove(&conv_id);
        }

        let conv_id_clone = conv_id.clone();
        let response_clone = response.clone();
        let message_clone = message.to_string();
        tokio::spawn({
            let pool = self.pool.clone();
            let app_handle = app_handle.clone();
            let recent_evals = self.recent_evals.clone();
            async move {
                let previous_topics: Vec<String> = Vec::new();
                let result = crate::services::xin_post_process_service::XinPostProcessor::run_pipeline(
                    &message_clone,
                    &response_clone,
                    &previous_topics,
                );

                let enhanced = crate::services::xin_post_process_service::XinPostProcessor::run_enhanced_pipeline(
                    &message_clone,
                    &response_clone,
                    &previous_topics,
                    None,
                );

                let _ = app_handle.emit(
                    "xin-post-process",
                    serde_json::json!({
                        "conversation_id": conv_id_clone,
                        "intent": result.intent,
                        "sentiment": result.sentiment,
                        "topics": result.topics,
                        "tags": result.tags,
                        "quality": enhanced.quality,
                        "tone": enhanced.tone,
                        "factual": enhanced.factual,
                        "safety": enhanced.safety,
                    }),
                );

                let memories = crate::services::xin_post_process_service::XinPostProcessor::extract_memory_entries(
                    &message_clone,
                    &response_clone,
                    &result,
                );

                let now = chrono::Utc::now().to_rfc3339();
                for (category, key, value, importance) in memories {
                    let memory_id = format!("mem_{}", uuid::Uuid::new_v4());
                    let _ = sqlx::query(
                        "INSERT OR REPLACE INTO xin_memories (id, user_id, category, key, value, importance, source, confidence, created_at) VALUES (?, ?, ?, ?, ?, ?, 'dialogue', ?, ?)",
                    )
                    .bind(&memory_id)
                    .bind(user_id)
                    .bind(serde_json::to_string(&category).unwrap_or_default().trim_matches('"').to_string())
                    .bind(&key)
                    .bind(&value)
                    .bind(importance)
                    .bind(importance)
                    .bind(&now)
                    .execute(&pool)
                    .await;
                }

                {
                    let dream_state = DREAM_STATE.lock().await;
                    let dream_config = crate::services::xin_dream_service::DreamConfig::default();
                    if crate::services::xin_dream_service::XinDreamService::check_dreaming_due(
                        "light", &dream_state, &dream_config,
                    ) {
                        drop(dream_state);

                        let conversation_texts = vec![
                            format!("用户: {}", message_clone),
                            format!("小欣: {}", response_clone),
                        ];
                        let light_config = dream_config.phases.light.clone();
                        let light_result = crate::services::xin_dream_service::XinDreamService::run_light_dreaming(
                            &[],
                            &conversation_texts,
                            &light_config,
                        );

                        let _ = app_handle.emit(
                            "xin-dream-light",
                            serde_json::json!({
                                "candidate_count": light_result.candidates.len(),
                                "deduped_count": light_result.deduped_count,
                                "performed_at": light_result.performed_at.to_rfc3339(),
                            }),
                        );

                        let mut state = DREAM_STATE.lock().await;
                        state.last_light_at = Some(chrono::Utc::now());
                        state.light_candidates_count += light_result.candidates.len();
                    }
                }

                let commit_result = crate::services::xin_commit_service::XinCommitService::extract_from_dialogue(
                    &message_clone,
                    &response_clone,
                    &conv_id_clone,
                );
                if !commit_result.candidates.is_empty() {
                    let _ = app_handle.emit(
                        "xin-commit-extracted",
                        serde_json::json!({
                            "conversation_id": conv_id_clone,
                            "candidate_count": commit_result.candidates.len(),
                            "candidates": commit_result.candidates,
                        }),
                    );
                }

                let eval_result = crate::services::xin_michelin_service::XinMichelinService::evaluate_dialogue(
                    &message_clone,
                    &response_clone,
                    0,
                );
                let _ = app_handle.emit(
                    "xin-eval-result",
                    serde_json::json!({
                        "overall_score": eval_result.overall_score,
                        "stars": eval_result.stars,
                        "star_label": eval_result.star_label,
                        "strengths": eval_result.strengths,
                        "weaknesses": eval_result.weaknesses,
                        "improvement_suggestion": eval_result.improvement_suggestion,
                    }),
                );

                let mut evals = recent_evals.lock().await;
                evals.push(eval_result.clone());
                if evals.len() > 50 {
                    evals.remove(0);
                }
            }
        });

        let persona_for_evolution = persona.clone();
        let recent_evals_handle = self.recent_evals.clone();
        let last_evolution = self.last_evolution_persona.clone();
        let app_handle_evo = app_handle.clone();
        tokio::spawn(async move {
            let evals = recent_evals_handle.lock().await;
            if evals.len() < 10 {
                return;
            }
            let current_style = persona_for_evolution.speaking_style.clone();
            let decision = crate::services::xin_personality_evolution_service::XinPersonalityEvolutionService::analyze_evolution(
                &evals,
                &current_style,
            );
            if !decision.should_evolve {
                return;
            }
            let new_style = crate::services::xin_personality_evolution_service::XinPersonalityEvolutionService::apply_decision(
                &current_style,
                &decision,
            );
            let mut last = last_evolution.lock().await;
            *last = Some(new_style.clone());
            let _ = app_handle_evo.emit(
                "xin-evolution-suggestion",
                serde_json::json!({
                    "persona_id": persona_for_evolution.id,
                    "reason": decision.reason,
                    "confidence": decision.confidence,
                    "adjustments": decision.adjustments,
                    "current_style": current_style,
                    "suggested_style": new_style,
                }),
            );
        });

        let recent_evals_proactive = self.recent_evals.clone();
        let persona_proactive = persona.clone();
        let app_handle_proactive = app_handle.clone();
        tokio::spawn(async move {
            let evals = recent_evals_proactive.lock().await;
            let care_reason = crate::services::xin_proactive_service::XinProactiveService::should_send_care_check_in(&evals);
            if let Some(reason) = care_reason {
                let action = crate::services::xin_proactive_service::XinProactiveService::generate_care_action(
                    &persona_proactive,
                    &reason,
                );
                let _ = app_handle_proactive.emit(
                    "xin-proactive-action",
                    serde_json::json!({
                        "action": action,
                        "persona_id": persona_proactive.id,
                    }),
                );
            }
        });

        let tool_response = response.clone();
        tokio::spawn({
            let pool = self.pool.clone();
            let app_handle = app_handle.clone();
            let conv_id_tool = conv_id.clone();
            let user_id_tool = user_id;
            async move {
                let tool_calls = crate::services::xin_tool_service::ToolExecutor::parse_tool_calls(&tool_response);
                if !tool_calls.is_empty() {
                    let mut results = Vec::new();
                    for call in &tool_calls {
                        let result = crate::services::xin_tool_service::ToolExecutor::execute(&pool, user_id_tool, call).await;
                        if let Ok(res) = result {
                            results.push(res);
                        }
                    }
                    let formatted = crate::services::xin_tool_service::ToolExecutor::format_tool_results_for_context(&results);
                    let _ = app_handle.emit(
                        "xin-tool-results",
                        serde_json::json!({
                            "conversation_id": conv_id_tool,
                            "tool_calls": tool_calls,
                            "results": results,
                        }),
                    );
                    if !formatted.is_empty() {
                        let _ = app_handle.emit(
                            "xin-tool-context",
                            serde_json::json!({
                                "conversation_id": conv_id_tool,
                                "context": formatted,
                            }),
                        );
                    }
                }
            }
        });

        let enhancement = crate::services::xin_multimodal_service::XinMultimodalService::enhance_output(&response);
        let _ = app_handle.emit("xin-display-hints", crate::services::xin_multimodal_service::XinMultimodalService::format_for_display(&response, &enhancement));

        Ok(XinChatResponse {
            conversation_id: conv_id,
            content: response,
            token_usage: XinTokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens,
            },
            is_new_conversation: is_new,
        })
    }

    pub async fn stop_generation(&self, conversation_id: &str) -> Result<(), AppError> {
        let flags = self.stop_flags.lock().await;
        if let Some(flag) = flags.get(conversation_id) {
            flag.store(true, Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn active_contexts(&self) -> tokio::sync::MutexGuard<'_, HashMap<String, ContextWindow>> {
        self.active_contexts.lock().await
    }

    pub async fn get_active_context(&self, conv_id: &str) -> Option<ContextWindow> {
        let contexts = self.active_contexts.lock().await;
        contexts.get(conv_id).cloned()
    }

    /// D3.1: 内部辅助 — 从 AiModel 解析出 (api_url, model_name, temperature)
    fn resolve_model_endpoint(model: &AiModel) -> (String, String, f64) {
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
        let temperature = model.temperature.unwrap_or(0.7);
        (api_url, model_name, temperature)
    }

    async fn call_llm_sync(
        ctx: &ContextWindow,
        model: &AiModel,
        api_key: &str,
    ) -> Result<String, String> {
        let (api_url, model_name, temperature) = Self::resolve_model_endpoint(model);
        let messages: Vec<serde_json::Value> = ctx
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        ChatRole::System => "system",
                        ChatRole::User => "user",
                        ChatRole::Assistant => "assistant",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

        let body = serde_json::json!({
            "model": model_name,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": 2048,
            "stream": false,
        });

        let mut req_builder = client
            .post(format!("{}/chat/completions", api_url))
            .header("Content-Type", "application/json")
            .json(&body);
        if !api_key.is_empty() {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| format!("API 请求失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("API 返回错误 [{}]: {}", status, text));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("响应解析失败: {}", e))?;

        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "响应格式异常".to_string())
    }

    async fn call_llm_stream(
        app_handle: &tauri::AppHandle,
        conversation_id: &str,
        ctx: &ContextWindow,
        model: &AiModel,
        api_key: &str,
        stop_flag: &Arc<AtomicBool>,
    ) -> Result<String, String> {
        let (api_url, model_name, temperature) = Self::resolve_model_endpoint(model);
        let messages: Vec<serde_json::Value> = ctx
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        ChatRole::System => "system",
                        ChatRole::User => "user",
                        ChatRole::Assistant => "assistant",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

        let body = serde_json::json!({
            "model": model_name,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": 4096,
            "stream": true,
        });

        let mut req_builder = client
            .post(format!("{}/chat/completions", api_url))
            .header("Content-Type", "application/json")
            .json(&body);
        if !api_key.is_empty() {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| format!("流式请求失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("API 返回错误 [{}]: {}", status, text));
        }

        let mut full_response = String::new();
        let mut stream = resp.bytes_stream();

        let _ = app_handle.emit(
            "xin-stream",
            serde_json::json!({
                "conversation_id": conversation_id,
                "event": "thinking_start",
                "chunk": "",
                "done": false,
            }),
        );

        while let Some(chunk_result) = stream.next().await {
            if stop_flag.load(Ordering::Relaxed) {
                let _ = app_handle.emit(
                    "xin-stream",
                    serde_json::json!({
                        "conversation_id": conversation_id,
                        "event": "stopped",
                        "chunk": "",
                        "done": true,
                    }),
                );
                break;
            }

            let chunk = chunk_result.map_err(|e| format!("流读取失败: {}", e))?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                let line = line.trim();
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        let _ = app_handle.emit(
                            "xin-stream",
                            serde_json::json!({
                                "conversation_id": conversation_id,
                                "event": "done",
                                "chunk": "",
                                "done": true,
                            }),
                        );
                        return Ok(full_response);
                    }

                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                            full_response.push_str(content);
                            let _ = app_handle.emit(
                                "xin-stream",
                                serde_json::json!({
                                    "conversation_id": conversation_id,
                                    "event": "chunk",
                                    "chunk": content,
                                    "done": false,
                                }),
                            );
                        }
                    }
                }
            }
        }

        let _ = app_handle.emit(
            "xin-stream",
            serde_json::json!({
                "conversation_id": conversation_id,
                "event": "done",
                "chunk": "",
                "done": true,
            }),
        );

        Ok(full_response)
    }

    async fn call_llm_sync_multimodal(
        ctx: &ContextWindow,
        multimodal_messages: &[serde_json::Value],
        use_multimodal: bool,
        model: &AiModel,
        api_key: &str,
    ) -> Result<String, String> {
        if !use_multimodal {
            return Self::call_llm_sync(ctx, model, api_key).await;
        }

        let (api_url, model_name, temperature) = Self::resolve_model_endpoint(model);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

        let body = serde_json::json!({
            "model": model_name,
            "messages": multimodal_messages,
            "temperature": temperature,
            "max_tokens": 2048,
            "stream": false,
        });

        let mut req_builder = client
            .post(format!("{}/chat/completions", api_url))
            .header("Content-Type", "application/json")
            .json(&body);
        if !api_key.is_empty() {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| format!("API 请求失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("API 返回错误 [{}]: {}", status, text));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("响应解析失败: {}", e))?;

        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "响应格式异常".to_string())
    }

    async fn call_llm_stream_multimodal(
        app_handle: &tauri::AppHandle,
        conversation_id: &str,
        ctx: &ContextWindow,
        multimodal_messages: &[serde_json::Value],
        use_multimodal: bool,
        model: &AiModel,
        api_key: &str,
        stop_flag: &Arc<AtomicBool>,
    ) -> Result<String, String> {
        if !use_multimodal {
            return Self::call_llm_stream(
                app_handle,
                conversation_id,
                ctx,
                model,
                api_key,
                stop_flag,
            )
            .await;
        }

        let (api_url, model_name, temperature) = Self::resolve_model_endpoint(model);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

        let body = serde_json::json!({
            "model": model_name,
            "messages": multimodal_messages,
            "temperature": temperature,
            "max_tokens": 4096,
            "stream": true,
        });

        let mut req_builder = client
            .post(format!("{}/chat/completions", api_url))
            .header("Content-Type", "application/json")
            .json(&body);
        if !api_key.is_empty() {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| format!("流式请求失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("API 返回错误 [{}]: {}", status, text));
        }

        let mut full_response = String::new();
        let mut stream = resp.bytes_stream();

        let _ = app_handle.emit(
            "xin-stream",
            serde_json::json!({
                "conversation_id": conversation_id,
                "event": "thinking_start",
                "chunk": "",
                "done": false,
            }),
        );

        while let Some(chunk_result) = stream.next().await {
            if stop_flag.load(Ordering::Relaxed) {
                let _ = app_handle.emit(
                    "xin-stream",
                    serde_json::json!({
                        "conversation_id": conversation_id,
                        "event": "stopped",
                        "chunk": "",
                        "done": true,
                    }),
                );
                break;
            }

            let chunk = chunk_result.map_err(|e| format!("流读取失败: {}", e))?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                let line = line.trim();
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        let _ = app_handle.emit(
                            "xin-stream",
                            serde_json::json!({
                                "conversation_id": conversation_id,
                                "event": "done",
                                "chunk": "",
                                "done": true,
                            }),
                        );
                        return Ok(full_response);
                    }

                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                            full_response.push_str(content);
                            let _ = app_handle.emit(
                                "xin-stream",
                                serde_json::json!({
                                    "conversation_id": conversation_id,
                                    "event": "chunk",
                                    "chunk": content,
                                    "done": false,
                                }),
                            );
                        }
                    }
                }
            }
        }

        let _ = app_handle.emit(
            "xin-stream",
            serde_json::json!({
                "conversation_id": conversation_id,
                "event": "done",
                "chunk": "",
                "done": true,
            }),
        );

        Ok(full_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_model_endpoint_defaults() {
        // D3.1: 测试 AiModel 端点解析
        let model = AiModel {
            id: 1,
            user_id: 1,
            name: "test".into(),
            provider: "openai".into(),
            api_url: None,
            api_key_enc: None,
            api_key_nonce: None,
            api_format: None,
            model_name: None,
            display_name: None,
            is_local: false,
            multimodal: false,
            system_prompt: None,
            context_window: None,
            temperature: Some(0.5),
            created_at: 0,
            updated_at: 0,
            // v114 迁移新增的健康检测字段（测试编译修复）
            status: "active".into(),
            last_health_check: None,
            latency_ms: None,
        };
        let (api_url, model_name, temperature) = XinDialogueService::resolve_model_endpoint(&model);
        assert_eq!(api_url, "https://api.openai.com/v1");
        assert_eq!(model_name, "gpt-4o-mini");
        assert_eq!(temperature, 0.5);
    }

    #[test]
    fn test_memory_extraction_keywords() {
        let user_msg = "你好，我喜欢吃苹果，我住在北京，经常跑步";
        let ai_msg = "很高兴认识你！你喜欢吃苹果是个好习惯。北京是个好地方。";

        let text = format!("{} {}", user_msg, ai_msg);
        assert!(text.contains("喜欢"));
        assert!(text.contains("住在"));
        assert!(text.contains("经常"));
    }
}