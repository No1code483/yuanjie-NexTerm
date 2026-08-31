use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::services::xin_context_service::{
    estimate_tokens, ChatMessage, ChatRole, ContextWindow, XinContextManager,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    pub enabled: bool,
    pub trigger_threshold_ratio: f64,
    pub keep_recent_tokens: usize,
    pub model_override: Option<String>,
    pub memory_flush_enabled: bool,
    pub notify_user: bool,
    pub compaction_mode: String,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            trigger_threshold_ratio: 0.8,
            keep_recent_tokens: 4000,
            model_override: None,
            memory_flush_enabled: true,
            notify_user: false,
            compaction_mode: "sliding_window".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionRecord {
    pub id: String,
    pub conversation_id: String,
    pub trigger_type: String,
    pub pre_message_count: i64,
    pub pre_token_count: i64,
    pub post_message_count: i64,
    pub post_token_count: i64,
    pub summary_text: String,
    pub memory_flush_count: i64,
    pub guidance_text: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionRequest {
    pub conversation_id: String,
    pub messages_json: String,
    pub model_id: String,
    pub guidance: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResponse {
    pub compacted: bool,
    pub record: Option<CompactionRecord>,
    pub new_context_json: String,
    pub messages_before: usize,
    pub messages_after: usize,
    pub tokens_before: usize,
    pub tokens_after: usize,
    pub summary_text: String,
    pub memory_flush_count: usize,
    pub needs_compaction: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFlushResult {
    pub saved_count: usize,
    pub memories: Vec<MemoryFlushItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFlushItem {
    pub key: String,
    pub value: String,
    pub importance: f64,
}

pub struct XinCompactionService;

impl XinCompactionService {
    pub async fn get_config(
        pool: &SqlitePool,
        user_id: i64,
    ) -> Result<CompactionConfig, AppError> {
        let row: Option<(i64, f64, i64, Option<String>, i64, i64, String)> = sqlx::query_as(
            "SELECT enabled, trigger_threshold_ratio, keep_recent_tokens, model_override,
                    memory_flush_enabled, notify_user, compaction_mode
             FROM xin_compaction_config WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        match row {
            Some((enabled, ratio, keep, model, flush, notify, mode)) => Ok(CompactionConfig {
                enabled: enabled != 0,
                trigger_threshold_ratio: ratio,
                keep_recent_tokens: keep as usize,
                model_override: model,
                memory_flush_enabled: flush != 0,
                notify_user: notify != 0,
                compaction_mode: mode,
            }),
            None => {
                let config = CompactionConfig::default();
                Self::save_config(pool, user_id, &config).await?;
                Ok(config)
            }
        }
    }

    pub async fn update_config(
        pool: &SqlitePool,
        user_id: i64,
        config: &CompactionConfig,
    ) -> Result<CompactionConfig, AppError> {
        // 多用户隔离：migration 0119 仅添加 user_id 列（无 UNIQUE 约束），
        // 无法使用 ON CONFLICT(user_id)。沿用 xiaoxin_repo::upsert_habit 模式：
        // 先 UPDATE WHERE user_id = ?，未命中（新用户首次保存配置）再 INSERT。
        let affected = sqlx::query(
            "UPDATE xin_compaction_config SET enabled = ?, trigger_threshold_ratio = ?,
             keep_recent_tokens = ?, model_override = ?, memory_flush_enabled = ?,
             notify_user = ?, compaction_mode = ? WHERE user_id = ?",
        )
        .bind(config.enabled as i64)
        .bind(config.trigger_threshold_ratio)
        .bind(config.keep_recent_tokens as i64)
        .bind(&config.model_override)
        .bind(config.memory_flush_enabled as i64)
        .bind(config.notify_user as i64)
        .bind(&config.compaction_mode)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?
        .rows_affected();

        if affected == 0 {
            sqlx::query(
                "INSERT INTO xin_compaction_config (user_id, enabled, trigger_threshold_ratio,
                 keep_recent_tokens, model_override, memory_flush_enabled, notify_user, compaction_mode)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(user_id)
            .bind(config.enabled as i64)
            .bind(config.trigger_threshold_ratio)
            .bind(config.keep_recent_tokens as i64)
            .bind(&config.model_override)
            .bind(config.memory_flush_enabled as i64)
            .bind(config.notify_user as i64)
            .bind(&config.compaction_mode)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        }

        Ok(config.clone())
    }

    async fn save_config(
        pool: &SqlitePool,
        user_id: i64,
        config: &CompactionConfig,
    ) -> Result<(), AppError> {
        // 多用户隔离：仅在该 user_id 尚无配置时插入默认配置
        sqlx::query(
            "INSERT INTO xin_compaction_config (user_id, enabled, trigger_threshold_ratio,
             keep_recent_tokens, model_override, memory_flush_enabled, notify_user, compaction_mode)
             SELECT ?, ?, ?, ?, ?, ?, ?, ?
             WHERE NOT EXISTS (SELECT 1 FROM xin_compaction_config WHERE user_id = ?)",
        )
        .bind(user_id)
        .bind(config.enabled as i64)
        .bind(config.trigger_threshold_ratio)
        .bind(config.keep_recent_tokens as i64)
        .bind(&config.model_override)
        .bind(config.memory_flush_enabled as i64)
        .bind(config.notify_user as i64)
        .bind(&config.compaction_mode)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn needs_compaction(
        pool: &SqlitePool,
        user_id: i64,
        context: &ContextWindow,
    ) -> Result<bool, AppError> {
        let config = Self::get_config(pool, user_id).await?;
        if !config.enabled {
            return Ok(false);
        }

        let threshold = (context.max_tokens as f64 * config.trigger_threshold_ratio) as usize;
        Ok(context.used_tokens > threshold)
    }

    pub async fn get_records(
        pool: &SqlitePool,
        user_id: i64,
        conversation_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<CompactionRecord>, AppError> {
        let limit = limit.unwrap_or(20) as i64;
        let rows: Vec<CompactionRecordRow> = sqlx::query_as(
            "SELECT id, conversation_id, trigger_type, pre_message_count, pre_token_count,
                    post_message_count, post_token_count, summary_text, memory_flush_count,
                    guidance_text, created_at
             FROM xin_compaction_records
             WHERE user_id = ? AND conversation_id = ?
             ORDER BY created_at DESC
             LIMIT ?",
        )
        .bind(user_id)
        .bind(conversation_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows
            .into_iter()
            .map(|r| CompactionRecord {
                id: r.id,
                conversation_id: r.conversation_id,
                trigger_type: r.trigger_type,
                pre_message_count: r.pre_message_count,
                pre_token_count: r.pre_token_count,
                post_message_count: r.post_message_count,
                post_token_count: r.post_token_count,
                summary_text: r.summary_text,
                memory_flush_count: r.memory_flush_count,
                guidance_text: r.guidance_text,
                created_at: r.created_at,
            })
            .collect())
    }

    pub async fn auto_compact(
        pool: &SqlitePool,
        user_id: i64,
        conversation_id: &str,
        context: &mut ContextWindow,
        model_id: &str,
    ) -> Result<CompactionResponse, AppError> {
        let config = Self::get_config(pool, user_id).await?;

        if !config.enabled {
            return Ok(CompactionResponse {
                compacted: false,
                record: None,
                new_context_json: serde_json::to_string(context).unwrap_or_default(),
                messages_before: context.messages.len(),
                messages_after: context.messages.len(),
                tokens_before: context.used_tokens,
                tokens_after: context.used_tokens,
                summary_text: String::new(),
                memory_flush_count: 0,
                needs_compaction: false,
            });
        }

        let threshold = (context.max_tokens as f64 * config.trigger_threshold_ratio) as usize;
        if context.used_tokens <= threshold {
            return Ok(CompactionResponse {
                compacted: false,
                record: None,
                new_context_json: serde_json::to_string(context).unwrap_or_default(),
                messages_before: context.messages.len(),
                messages_after: context.messages.len(),
                tokens_before: context.used_tokens,
                tokens_after: context.used_tokens,
                summary_text: String::new(),
                memory_flush_count: 0,
                needs_compaction: false,
            });
        }

        let messages_before = context.messages.len();
        let tokens_before = context.used_tokens;

        let mut memory_flush_count = 0usize;
        if config.memory_flush_enabled {
            let flush = Self::memory_flush(pool, user_id, context).await?;
            memory_flush_count = flush.saved_count;
        }

        let (summary_text, compacted_ctx) =
            Self::build_compacted_context(context, &config, model_id);

        let messages_after = compacted_ctx.messages.len();
        let tokens_after = compacted_ctx.used_tokens;
        let context_json = serde_json::to_string(&compacted_ctx).unwrap_or_default();

        *context = compacted_ctx;

        let record = Self::save_record(
            pool,
            user_id,
            conversation_id,
            "auto",
            messages_before as i64,
            tokens_before as i64,
            messages_after as i64,
            tokens_after as i64,
            &summary_text,
            memory_flush_count as i64,
            None,
        )
        .await?;

        Ok(CompactionResponse {
            compacted: true,
            record: Some(record),
            new_context_json: context_json,
            messages_before,
            messages_after,
            tokens_before,
            tokens_after,
            summary_text,
            memory_flush_count,
            needs_compaction: false,
        })
    }

    pub async fn manual_compact(
        pool: &SqlitePool,
        user_id: i64,
        conversation_id: &str,
        context: &mut ContextWindow,
        model_id: &str,
        guidance: Option<&str>,
    ) -> Result<CompactionResponse, AppError> {
        let config = Self::get_config(pool, user_id).await?;
        let messages_before = context.messages.len();
        let tokens_before = context.used_tokens;

        let mut memory_flush_count = 0usize;
        if config.memory_flush_enabled {
            let flush = Self::memory_flush(pool, user_id, context).await?;
            memory_flush_count = flush.saved_count;
        }

        let (summary_text, compacted_ctx) =
            Self::build_compacted_context_with_guidance(context, &config, model_id, guidance);

        let messages_after = compacted_ctx.messages.len();
        let tokens_after = compacted_ctx.used_tokens;
        let context_json = serde_json::to_string(&compacted_ctx).unwrap_or_default();

        *context = compacted_ctx;

        let record = Self::save_record(
            pool,
            user_id,
            conversation_id,
            "manual",
            messages_before as i64,
            tokens_before as i64,
            messages_after as i64,
            tokens_after as i64,
            &summary_text,
            memory_flush_count as i64,
            guidance.map(|s| s.to_string()),
        )
        .await?;

        let needs = context.used_tokens
            > (context.max_tokens as f64 * config.trigger_threshold_ratio) as usize;

        Ok(CompactionResponse {
            compacted: true,
            record: Some(record),
            new_context_json: context_json,
            messages_before,
            messages_after,
            tokens_before,
            tokens_after,
            summary_text,
            memory_flush_count,
            needs_compaction: needs,
        })
    }

    async fn save_record(
        pool: &SqlitePool,
        user_id: i64,
        conversation_id: &str,
        trigger_type: &str,
        pre_message_count: i64,
        pre_token_count: i64,
        post_message_count: i64,
        post_token_count: i64,
        summary_text: &str,
        memory_flush_count: i64,
        guidance_text: Option<String>,
    ) -> Result<CompactionRecord, AppError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO xin_compaction_records (id, user_id, conversation_id, trigger_type,
             pre_message_count, pre_token_count, post_message_count, post_token_count,
             summary_text, memory_flush_count, guidance_text, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(user_id)
        .bind(conversation_id)
        .bind(trigger_type)
        .bind(pre_message_count)
        .bind(pre_token_count)
        .bind(post_message_count)
        .bind(post_token_count)
        .bind(summary_text)
        .bind(memory_flush_count)
        .bind(&guidance_text)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(CompactionRecord {
            id,
            conversation_id: conversation_id.to_string(),
            trigger_type: trigger_type.to_string(),
            pre_message_count,
            pre_token_count,
            post_message_count,
            post_token_count,
            summary_text: summary_text.to_string(),
            memory_flush_count,
            guidance_text,
            created_at: now,
        })
    }

    fn build_compacted_context(
        context: &ContextWindow,
        config: &CompactionConfig,
        model_id: &str,
    ) -> (String, ContextWindow) {
        Self::build_compacted_context_with_guidance(context, config, model_id, None)
    }

    fn build_compacted_context_with_guidance(
        context: &ContextWindow,
        config: &CompactionConfig,
        _model_id: &str,
        guidance: Option<&str>,
    ) -> (String, ContextWindow) {
        match config.compaction_mode.as_str() {
            "hierarchical" => Self::hierarchical_compact(context, config, guidance),
            _ => Self::sliding_window_compact(context, config, guidance),
        }
    }

    fn sliding_window_compact(
        context: &ContextWindow,
        config: &CompactionConfig,
        guidance: Option<&str>,
    ) -> (String, ContextWindow) {
        let mut new_ctx = ContextWindow {
            messages: Vec::new(),
            model_id: context.model_id.clone(),
            max_tokens: context.max_tokens,
            used_tokens: 0,
            summary: None,
        };

        let mut system_msgs: Vec<ChatMessage> = Vec::new();
        let mut non_system: Vec<ChatMessage> = Vec::new();

        for msg in &context.messages {
            if msg.role == ChatRole::System {
                system_msgs.push(msg.clone());
            } else {
                non_system.push(msg.clone());
            }
        }

        for msg in &system_msgs {
            XinContextManager::add_message(
                &mut new_ctx,
                msg.clone(),
            );
        }

        if non_system.is_empty() {
            return (String::new(), new_ctx);
        }

        let mut recent_tokens = 0usize;
        let mut split_idx = non_system.len();

        for (i, msg) in non_system.iter().enumerate().rev() {
            let t = estimate_tokens(&msg.content);
            if recent_tokens + t > config.keep_recent_tokens && i > 1 {
                split_idx = i;
                break;
            }
            recent_tokens += t;
        }

        let old_msgs = &non_system[..split_idx];
        let recent_msgs = &non_system[split_idx..];

        let summary = Self::generate_summary(old_msgs, guidance, &system_msgs);

        let summary_msg = ChatMessage {
            role: ChatRole::System,
            content: format!(
                "【对话历史摘要 — 早期内容已压缩】\n{}",
                summary
            ),
            name: Some("compaction".to_string()),
            timestamp: None,
        };
        XinContextManager::add_message(&mut new_ctx, summary_msg);

        for msg in recent_msgs {
            XinContextManager::add_message(&mut new_ctx, msg.clone());
        }

        new_ctx.summary = Some(summary.clone());

        (summary, new_ctx)
    }

    fn hierarchical_compact(
        context: &ContextWindow,
        config: &CompactionConfig,
        guidance: Option<&str>,
    ) -> (String, ContextWindow) {
        let (first_pass, mid_ctx) =
            Self::sliding_window_compact(context, config, guidance);

        if mid_ctx.messages.len() <= 4 {
            return (first_pass, mid_ctx);
        }

        let non_system_count = mid_ctx
            .messages
            .iter()
            .filter(|m| m.role != ChatRole::System)
            .count();

        if non_system_count <= 4 {
            return (first_pass, mid_ctx);
        }

        let mut system_msgs: Vec<ChatMessage> = Vec::new();
        let mut non_system: Vec<ChatMessage> = Vec::new();
        for msg in &mid_ctx.messages {
            if msg.role == ChatRole::System || msg.name.as_deref() == Some("compaction") {
                system_msgs.push(msg.clone());
            } else {
                non_system.push(msg.clone());
            }
        }

        let all_system_text: Vec<String> = system_msgs.iter().map(|m| m.content.clone()).collect();
        let combined_system = all_system_text.join("\n\n");

        let final_ctx = ContextWindow {
            messages: vec![
                ChatMessage {
                    role: ChatRole::System,
                    content: combined_system,
                    name: Some("compaction".to_string()),
                    timestamp: None,
                },
            ],
            model_id: mid_ctx.model_id.clone(),
            max_tokens: mid_ctx.max_tokens,
            used_tokens: 0,
            summary: Some(first_pass.clone()),
        };

        let mut merged_ctx = final_ctx;
        for msg in non_system {
            XinContextManager::add_message(&mut merged_ctx, msg);
        }

        (first_pass, merged_ctx)
    }

    fn generate_summary(
        messages: &[ChatMessage],
        guidance: Option<&str>,
        _system_msgs: &[ChatMessage],
    ) -> String {
        if messages.is_empty() {
            return "（无历史对话）".to_string();
        }

        let mut parts: Vec<String> = Vec::new();

        if let Some(g) = guidance {
            parts.push(format!("【压缩指导】{}", g));
        }

        parts.push(format!(
            "以下为 {} 条消息的摘要（原始约 {} tokens）：",
            messages.len(),
            messages.iter().map(|m| estimate_tokens(&m.content)).sum::<usize>()
        ));

        let mut qa_pairs: Vec<(Option<&ChatMessage>, Option<&ChatMessage>)> = Vec::new();
        let mut current_q: Option<&ChatMessage> = None;

        for msg in messages {
            match msg.role {
                ChatRole::User => {
                    if current_q.is_some() {
                        qa_pairs.push((current_q, None));
                    }
                    current_q = Some(msg);
                }
                ChatRole::Assistant => {
                    if current_q.is_some() {
                        qa_pairs.push((current_q, Some(msg)));
                        current_q = None;
                    } else {
                        qa_pairs.push((None, Some(msg)));
                    }
                }
                _ => {}
            }
        }

        if current_q.is_some() {
            qa_pairs.push((current_q, None));
        }

        for (i, (q, a)) in qa_pairs.iter().enumerate() {
            if let Some(q_msg) = q {
                let q_preview: String = q_msg.content
                    .chars()
                    .take(120)
                    .collect();
                let q_ellipsis = if q_msg.content.chars().count() > 120 {
                    "…"
                } else {
                    ""
                };
                parts.push(format!("Q{}: {}{}", i + 1, q_preview, q_ellipsis));
            }

            if let Some(a_msg) = a {
                let a_summary = Self::summarize_assistant_response(&a_msg.content);
                parts.push(format!("A{}: {}", i + 1, a_summary));
            }
        }

        let topic = Self::extract_main_topic(messages);
        parts.push(format!("\n主要话题：{}", topic));

        parts.join("\n")
    }

    fn summarize_assistant_response(content: &str) -> String {
        let char_count = content.chars().count();

        if char_count <= 150 {
            return content.to_string();
        }

        let first_sentence = content
            .chars()
            .take(200)
            .collect::<String>();

        let last_sentence: String = if char_count > 400 {
            content
                .chars()
                .rev()
                .take(100)
                .collect::<String>()
                .chars()
                .rev()
                .collect()
        } else {
            String::new()
        };

        if last_sentence.is_empty() {
            format!("{}…（共{}字）", first_sentence, char_count)
        } else {
            format!(
                "{}…（省略{}字）…{}",
                first_sentence,
                char_count.saturating_sub(300),
                last_sentence
            )
        }
    }

    fn extract_main_topic(messages: &[ChatMessage]) -> String {
        let mut keyword_freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        let stop_words: std::collections::HashSet<&str> = [
            "的", "了", "在", "是", "我", "有", "和", "就", "不", "人", "都", "一", "一个",
            "上", "也", "很", "到", "说", "要", "去", "你", "会", "着", "没有", "看", "好",
            "自己", "这", "他", "她", "它", "们", "那", "什么", "怎么", "哪个", "为什么",
            "可以", "这个", "那个", "还", "让", "被", "把", "给", "对", "从", "与", "但",
            "而", "或", "因为", "所以", "如果", "虽然", "然后", "就是", "还是", "只是",
            "吗", "吧", "呢", "啊", "哦", "嗯", "哈", "the", "a", "an", "is", "are",
            "was", "were", "be", "been", "being", "have", "has", "had", "do", "does",
            "did", "will", "would", "could", "should", "may", "might", "can", "shall",
            "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into",
            "through", "during", "before", "after", "above", "below", "between",
            "and", "but", "or", "nor", "not", "so", "yet", "both", "either", "neither",
            "each", "every", "all", "any", "few", "more", "most", "other", "some",
            "such", "no", "only", "own", "same", "than", "too", "very", "just",
            "about", "over", "under", "again", "further", "then", "once",
            "here", "there", "when", "where", "why", "how", "which", "who",
            "whom", "this", "that", "these", "those", "it", "its",
        ]
        .iter()
        .cloned()
        .collect();

        for msg in messages {
            if msg.role == ChatRole::System {
                continue;
            }
            let words: Vec<String> = msg
                .content
                .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                .filter(|w| w.len() >= 2)
                .map(|w| w.to_lowercase())
                .collect();

            for word in words {
                if !stop_words.contains(word.as_str()) {
                    *keyword_freq.entry(word).or_insert(0) += 1;
                }
            }
        }

        let mut sorted: Vec<(&String, &usize)> = keyword_freq.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        sorted.truncate(5);

        if sorted.is_empty() {
            return "一般对话".to_string();
        }

        sorted
            .iter()
            .map(|(w, _)| w.as_str())
            .collect::<Vec<&str>>()
            .join("、")
    }

    pub async fn memory_flush(
        pool: &SqlitePool,
        user_id: i64,
        context: &ContextWindow,
    ) -> Result<MemoryFlushResult, AppError> {
        let user_msgs: Vec<&ChatMessage> = context
            .messages
            .iter()
            .filter(|m| m.role == ChatRole::User)
            .collect();

        let mut items = Vec::new();
        let now = chrono::Utc::now().to_rfc3339();

        for msg in &user_msgs {
            if let Some((key, value, importance)) = Self::extract_memory_candidate(msg) {
                let id = uuid::Uuid::new_v4().to_string();

                let exists: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM xin_memories WHERE user_id = ? AND key = ? LIMIT 1",
                )
                .bind(user_id)
                .bind(&key)
                .fetch_optional(pool)
                .await
                .map_err(AppError::Database)?;

                if exists.is_some() {
                    sqlx::query(
                        "UPDATE xin_memories SET value = ?, importance = MAX(importance, ?),
                         last_recalled_at = ? WHERE user_id = ? AND key = ?",
                    )
                    .bind(&value)
                    .bind(importance)
                    .bind(&now)
                    .bind(user_id)
                    .bind(&key)
                    .execute(pool)
                    .await
                    .map_err(AppError::Database)?;
                } else {
                    sqlx::query(
                        "INSERT INTO xin_memories (id, user_id, category, key, value, importance, source,
                         confidence, created_at, last_recalled_at)
                         VALUES (?, ?, 'knowledge', ?, ?, ?, 'compaction', ?, ?, ?)",
                    )
                    .bind(&id)
                    .bind(user_id)
                    .bind(&key)
                    .bind(&value)
                    .bind(importance)
                    .bind(0.6)
                    .bind(&now)
                    .bind(&now)
                    .execute(pool)
                    .await
                    .map_err(AppError::Database)?;
                }

                items.push(MemoryFlushItem {
                    key,
                    value,
                    importance,
                });
            }

            if items.len() >= 10 {
                break;
            }
        }

        Ok(MemoryFlushResult {
            saved_count: items.len(),
            memories: items,
        })
    }

    fn extract_memory_candidate(msg: &ChatMessage) -> Option<(String, String, f64)> {
        let content = msg.content.trim();
        if content.len() < 10 || content.len() > 500 {
            return None;
        }

        let indicators = [
            "我是", "我叫", "我喜欢", "我住在", "我的", "记住", "别忘了",
            "我的邮箱", "我的手机", "我的地址", "我的生日", "我偏好",
            "my name is", "i am", "i like", "i live", "my email",
        ];

        for indicator in &indicators {
            if content.to_lowercase().contains(&indicator.to_lowercase()) {
                let key = format!(
                    "user_{}",
                    indicator
                        .chars()
                        .filter(|c| c.is_alphanumeric())
                        .collect::<String>()
                        .to_lowercase()
                );
                return Some((key, content.to_string(), 0.7));
            }
        }

        if content.len() < 50 {
            return None;
        }

        let importance_keywords = ["重要", "关键", "核心", "必须", "一定", "important", "critical"];
        let has_importance = importance_keywords
            .iter()
            .any(|kw| content.to_lowercase().contains(kw));

        if has_importance {
            let key = format!(
                "important_{}",
                uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x")
            );
            return Some((key, content.to_string(), 0.5));
        }

        let question_count = content.matches('?').count()
            + content.matches('？').count();
        if question_count > 2 {
            let key = format!(
                "question_{}",
                uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x")
            );
            return Some((key, content.to_string(), 0.4));
        }

        None
    }
}

#[derive(sqlx::FromRow)]
struct CompactionRecordRow {
    id: String,
    conversation_id: String,
    trigger_type: String,
    pre_message_count: i64,
    pre_token_count: i64,
    post_message_count: i64,
    post_token_count: i64,
    summary_text: String,
    memory_flush_count: i64,
    guidance_text: Option<String>,
    created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::xin_context_service::ChatRole;

    fn make_msg(role: ChatRole, content: &str) -> ChatMessage {
        ChatMessage {
            role,
            content: content.to_string(),
            name: None,
            timestamp: None,
        }
    }

    fn make_context(model_id: &str, messages: Vec<ChatMessage>) -> ContextWindow {
        let mut ctx = XinContextManager::new_context(model_id);
        for msg in messages {
            XinContextManager::add_message(&mut ctx, msg);
        }
        ctx
    }

    #[test]
    fn test_sliding_window_compact_preserves_system() {
        let ctx = make_context(
            "gpt-4",
            vec![
                make_msg(ChatRole::System, "你是一个助手"),
                make_msg(ChatRole::User, "你好"),
                make_msg(ChatRole::Assistant, "你好！"),
            ],
        );

        let config = CompactionConfig {
            keep_recent_tokens: 10,
            ..CompactionConfig::default()
        };

        let (summary, new_ctx) =
            XinCompactionService::sliding_window_compact(&ctx, &config, None);

        assert!(new_ctx.messages.len() > 0);
        let system_count = new_ctx
            .messages
            .iter()
            .filter(|m| m.role == ChatRole::System)
            .count();
        assert!(system_count >= 1);
    }

    #[test]
    fn test_sliding_window_compact_large_context() {
        let long_text = "A".repeat(500);
        let mut messages = vec![make_msg(ChatRole::System, "你是一个助手")];
        for i in 0..80 {
            messages.push(make_msg(
                if i % 2 == 0 {
                    ChatRole::User
                } else {
                    ChatRole::Assistant
                },
                &format!("消息 {}: {}", i, long_text),
            ));
        }

        let ctx = make_context("gpt-4", messages);

        let config = CompactionConfig {
            keep_recent_tokens: 200,
            ..CompactionConfig::default()
        };

        let (summary, new_ctx) =
            XinCompactionService::sliding_window_compact(&ctx, &config, None);

        assert!(!summary.is_empty());
        assert!(new_ctx.messages.len() < ctx.messages.len());
        assert!(new_ctx.used_tokens < ctx.used_tokens);
    }

    #[test]
    fn test_generate_summary_handles_empty() {
        let summary =
            XinCompactionService::generate_summary(&[], None, &[]);
        assert!(summary.contains("无历史对话"));
    }

    #[test]
    fn test_generate_summary_with_guidance() {
        let messages = vec![
            make_msg(ChatRole::User, "什么是Rust的Ownership?"),
            make_msg(
                ChatRole::Assistant,
                "Rust的Ownership是内存管理的核心概念...",
            ),
        ];
        let summary = XinCompactionService::generate_summary(
            &messages,
            Some("重点关注技术问题"),
            &[],
        );
        assert!(summary.contains("压缩指导"));
        assert!(summary.contains("什么是Rust"));
    }

    #[test]
    fn test_extract_main_topic() {
        let messages = vec![
            make_msg(ChatRole::User, "Rust编程语言的内存管理模型是什么？"),
            make_msg(ChatRole::Assistant, "Rust使用所有权系统来管理内存..."),
            make_msg(ChatRole::User, "那Borrow Checker呢？"),
            make_msg(ChatRole::Assistant, "Borrow Checker是Rust的借用检查器..."),
        ];
        let topic = XinCompactionService::extract_main_topic(&messages);
        assert!(!topic.is_empty());
    }

    #[test]
    fn test_summarize_assistant_response_short() {
        let short = "简短回复";
        let result = XinCompactionService::summarize_assistant_response(short);
        assert_eq!(result, short);
    }

    #[test]
    fn test_summarize_assistant_response_long() {
        let long = "A".repeat(500);
        let result = XinCompactionService::summarize_assistant_response(&long);
        assert!(result.len() < 500);
        assert!(result.contains("字"));
    }

    #[test]
    fn test_memory_flush_extract_candidate() {
        let msg = make_msg(ChatRole::User, "我是张三，我喜欢编程");
        let result = XinCompactionService::extract_memory_candidate(&msg);
        assert!(result.is_some());
        let (key, value, importance) = result.unwrap();
        assert!(key.contains("user_"));
        assert_eq!(value, "我是张三，我喜欢编程");
        assert!(importance > 0.5);
    }

    #[test]
    fn test_memory_flush_extract_short_ignored() {
        let msg = make_msg(ChatRole::User, "好的");
        let result = XinCompactionService::extract_memory_candidate(&msg);
        assert!(result.is_none());
    }

    #[test]
    fn test_hierarchical_compact_small_context() {
        let ctx = make_context(
            "gpt-4",
            vec![
                make_msg(ChatRole::System, "你是助手"),
                make_msg(ChatRole::User, "你好"),
                make_msg(ChatRole::Assistant, "你好！"),
            ],
        );

        let config = CompactionConfig::default();
        let (summary, new_ctx) =
            XinCompactionService::hierarchical_compact(&ctx, &config, None);

        assert!(!summary.is_empty());
        assert!(new_ctx.messages.len() <= ctx.messages.len());
    }
}