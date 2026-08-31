pub mod strategy;

use std::collections::HashMap;
use std::time::Instant;

use crate::models::compact::{
    BudgetStatus, CompactionConfig, CompactionReason, CompactionRequest, CompactionResult,
    CompactionSession, CompactionStrategy, ConversationMessage, MessageTier,
    TokenBudget, TokenEstimate, message_importance,
};

pub struct CompactionManager {
    sessions: HashMap<String, CompactionSession>,
    config: CompactionConfig,
}

impl CompactionManager {
    pub fn new(config: Option<CompactionConfig>) -> Self {
        Self {
            sessions: HashMap::new(),
            config: config.unwrap_or_default(),
        }
    }

    pub fn update_config(&mut self, config: CompactionConfig) {
        self.config = config;
    }

    pub fn get_config(&self) -> &CompactionConfig {
        &self.config
    }

    pub fn estimate_tokens(&self, messages: &[ConversationMessage]) -> TokenBudget {
        let mut total = 0usize;
        for msg in messages {
            total += msg.token_count;
            if let Some(ref summary) = msg.summary {
                total += TokenEstimate::from_text(summary).estimated_tokens;
            }
        }

        let ratio = if self.config.max_tokens == 0 {
            1.0
        } else {
            total as f64 / self.config.max_tokens as f64
        };

        TokenBudget {
            max_tokens: self.config.max_tokens,
            used_tokens: total,
            available_tokens: self
                .config
                .max_tokens
                .saturating_sub(total),
            usage_ratio: ratio,
            status: BudgetStatus::from_ratio(ratio),
        }
    }

    pub fn needs_compaction(&self, messages: &[ConversationMessage]) -> bool {
        let budget = self.estimate_tokens(messages);
        budget.usage_ratio >= self.config.trigger_threshold
    }

    pub fn compact(&mut self, request: CompactionRequest) -> CompactionResult {
        let start = Instant::now();
        let config = request.config.clone().unwrap_or_else(|| self.config.clone());
        let strategy = config.strategy.clone();

        let result = match strategy {
            CompactionStrategy::Tiered => self.compact_tiered(&request, &config),
            CompactionStrategy::SlidingWindow => self.compact_sliding_window(&request, &config),
            CompactionStrategy::Summary => self.compact_summary(&request, &config),
            CompactionStrategy::Hybrid => self.compact_hybrid(&request, &config),
        };

        let _duration_ms = start.elapsed().as_millis() as u64;

        let session = self
            .sessions
            .entry(request.session_id.clone())
            .or_insert_with(|| CompactionSession {
                session_id: request.session_id.clone(),
                compaction_count: 0,
                total_tokens_saved: 0,
                last_compaction_at: None,
                strategy_used: None,
            });

        session.compaction_count += 1;
        session.total_tokens_saved += result.original_token_count.saturating_sub(result.compacted_token_count);
        session.last_compaction_at = Some(
            chrono::Utc::now().timestamp_millis(),
        );
        session.strategy_used = Some(strategy);

        result
    }

    fn compact_tiered(
        &self,
        request: &CompactionRequest,
        config: &CompactionConfig,
    ) -> CompactionResult {
        let original_tokens: usize = request.messages.iter().map(|m| m.token_count).sum();

        let now = chrono::Utc::now().timestamp_millis();

        let classified: Vec<ConversationMessage> = request
            .messages
            .iter()
            .map(|m| {
                let age = (now - m.timestamp).max(0) as f64 / 1000.0;
                let importance = message_importance(&m.content, &m.role, !m.file_changes.is_empty());
                let is_decision = m.role == "assistant"
                    && (m.content.to_lowercase().contains("决定")
                        || m.content.to_lowercase().contains("选择")
                        || m.content.to_lowercase().contains("最终"));
                let tier = MessageTier::from_importance(importance, age, is_decision);

                let mut msg = m.clone();
                msg.importance = importance;
                msg.is_decision = is_decision;
                msg.tier = Some(tier.clone());
                msg
            })
            .collect();

        let mut hot: Vec<ConversationMessage> = Vec::new();
        let mut warm: Vec<ConversationMessage> = Vec::new();
        let mut cold: Vec<ConversationMessage> = Vec::new();

        for msg in &classified {
            match msg.tier.as_ref() {
                Some(MessageTier::Hot) => hot.push(msg.clone()),
                Some(MessageTier::Warm) => warm.push(msg.clone()),
                Some(MessageTier::Cold) => cold.push(msg.clone()),
                None => warm.push(msg.clone()),
            }
        }

        let hot_count = hot.len();
        let warm_count = warm.len();
        let cold_count = cold.len();

        let mut kept = Vec::new();
        let mut summarized = Vec::new();
        let mut dropped = Vec::new();

        if config.system_prompt_always {
            if let Some(sp) = &request.system_prompt {
                kept.push(ConversationMessage {
                    id: "system_prompt".to_string(),
                    role: "system".to_string(),
                    content: sp.clone(),
                    timestamp: now,
                    importance: 1.0,
                    is_decision: false,
                    is_error: false,
                    file_changes: Vec::new(),
                    token_count: TokenEstimate::from_text(sp).estimated_tokens,
                    tier: Some(MessageTier::Hot),
                    summary: None,
                });
            }
        }

        let keep_last = config.keep_last_n.min(classified.len());
        let mut last_indices: Vec<usize> = Vec::new();
        for i in classified.len().saturating_sub(keep_last)..classified.len() {
            last_indices.push(i);
        }

        for (i, msg) in classified.iter().enumerate() {
            if last_indices.contains(&i) {
                kept.push(msg.clone());
                continue;
            }

            match msg.tier.as_ref() {
                Some(MessageTier::Hot) => kept.push(msg.clone()),
                Some(MessageTier::Warm) => {
                    let summary = Self::generate_tier_summary(msg, config);
                    let mut summarized_msg = msg.clone();
                    summarized_msg.summary = Some(summary);
                    summarized.push(summarized_msg.clone());
                    kept.push(summarized_msg);
                }
                Some(MessageTier::Cold) => dropped.push(msg.clone()),
                None => {
                    let summary = Self::generate_tier_summary(msg, config);
                    let mut summarized_msg = msg.clone();
                    summarized_msg.summary = Some(summary);
                    summarized.push(summarized_msg.clone());
                    kept.push(summarized_msg);
                }
            }
        }

        let compacted_tokens: usize = kept.iter().map(|m| {
            let base = if let Some(ref s) = m.summary {
                TokenEstimate::from_text(s).estimated_tokens
            } else {
                m.token_count
            };
            base
        }).sum();

        let decisions: Vec<String> = kept
            .iter()
            .filter(|m| m.is_decision)
            .map(|m| m.content.chars().take(200).collect::<String>() + "...")
            .collect();

        let errors: Vec<String> = kept
            .iter()
            .filter(|m| m.is_error)
            .map(|m| m.content.chars().take(200).collect::<String>() + "...")
            .collect();

        let summary = if cold_count > 0 {
            Some(format!(
                "已压缩对话: 保留{}条(热{} + 温{}摘要{}), 丢弃{}条冷数据",
                kept.len(), hot_count, warm_count, summarized.len(), cold_count
            ))
        } else {
            None
        };

        CompactionResult {
            strategy_used: CompactionStrategy::Tiered,
            original_token_count: original_tokens,
            compacted_token_count: compacted_tokens,
            compaction_ratio: if original_tokens > 0 {
                compacted_tokens as f64 / original_tokens as f64
            } else {
                0.0
            },
            messages_kept: kept.len(),
            messages_summarized: summarized.len(),
            messages_dropped: dropped.len(),
            hot_count,
            warm_count,
            cold_count,
            compacted_messages: kept,
            summary,
            decisions_preserved: decisions,
            errors_preserved: errors,
            truncated: compacted_tokens > config.max_tokens,
            duration_ms: 0,
        }
    }

    fn compact_sliding_window(
        &self,
        request: &CompactionRequest,
        config: &CompactionConfig,
    ) -> CompactionResult {
        let original_tokens: usize = request.messages.iter().map(|m| m.token_count).sum();
        let mut kept = Vec::new();
        let target = (config.max_tokens as f64 * config.target_ratio) as usize;

        let mut remain = Vec::new();
        for msg in &request.messages {
            let importance = message_importance(&msg.content, &msg.role, !msg.file_changes.is_empty());
            let mut m = msg.clone();
            m.importance = importance;
            remain.push(m);
        }

        let system_msgs: Vec<_> = remain.iter().filter(|m| m.role == "system").cloned().collect();
        let non_system: Vec<_> = remain.iter().filter(|m| m.role != "system").cloned().collect();

        if config.system_prompt_always && !system_msgs.is_empty() {
            kept.extend(system_msgs.clone());
        }

        let keep_last = config.keep_last_n.min(non_system.len());
        let split_idx = non_system.len().saturating_sub(keep_last);
        let (older, recent) = non_system.split_at(split_idx);

        let mut current_tokens: usize = kept.iter().map(|m| m.token_count).sum();
        current_tokens += recent.iter().map(|m| m.token_count).sum::<usize>();

        for msg in older.iter().rev() {
            if msg.importance >= 0.8 || (config.preserve_decisions && msg.is_decision) {
                if current_tokens + msg.token_count <= target {
                    kept.insert(if config.system_prompt_always { system_msgs.len() } else { 0 }, msg.clone());
                    current_tokens += msg.token_count;
                } else {
                    let summary = Self::generate_tier_summary(msg, config);
                    let mut summarized_msg = msg.clone();
                    summarized_msg.summary = Some(summary.clone());
                    kept.insert(
                        if config.system_prompt_always { system_msgs.len() } else { 0 },
                        summarized_msg.clone(),
                    );
                    current_tokens += TokenEstimate::from_text(&summary).estimated_tokens;
                }
            } else {
                let summary = Self::generate_tier_summary(msg, config);
                if current_tokens + TokenEstimate::from_text(&summary).estimated_tokens <= target {
                    let mut summarized_msg = msg.clone();
                    summarized_msg.summary = Some(summary.clone());
                    kept.insert(
                        if config.system_prompt_always { system_msgs.len() } else { 0 },
                        summarized_msg.clone(),
                    );
                    current_tokens += TokenEstimate::from_text(&summary).estimated_tokens;
                }
            }
        }

        kept.extend(recent.iter().cloned());

        CompactionResult {
            strategy_used: CompactionStrategy::SlidingWindow,
            original_token_count: original_tokens,
            compacted_token_count: current_tokens,
            compaction_ratio: if original_tokens > 0 {
                current_tokens as f64 / original_tokens as f64
            } else {
                0.0
            },
            messages_kept: kept.len(),
            messages_summarized: kept.iter().filter(|m| m.summary.is_some()).count() + 0,
            messages_dropped: original_tokens.saturating_sub(
                kept.iter().filter(|m| m.summary.is_some()).count(),
            ).min(0) as usize + original_tokens.saturating_sub(kept.iter().count() + kept.iter().filter(|m| m.summary.is_some()).count()) as usize,
            hot_count: kept.iter().filter(|m| m.tier == Some(MessageTier::Hot)).count(),
            warm_count: kept.iter().filter(|m| m.tier == Some(MessageTier::Warm)).count(),
            cold_count: 0,
            compacted_messages: kept,
            summary: None,
            decisions_preserved: Vec::new(),
            errors_preserved: Vec::new(),
            truncated: current_tokens > config.max_tokens,
            duration_ms: 0,
        }
    }

    fn compact_summary(
        &self,
        request: &CompactionRequest,
        config: &CompactionConfig,
    ) -> CompactionResult {
        let original_tokens: usize = request.messages.iter().map(|m| m.token_count).sum();

        let mut kept = Vec::new();
        let mut decisions = Vec::new();
        let mut errors = Vec::new();

        for msg in &request.messages {
            let importance = message_importance(&msg.content, &msg.role, !msg.file_changes.is_empty());
            let mut m = msg.clone();
            m.importance = importance;

            if m.role == "system" && config.system_prompt_always {
                kept.push(m.clone());
                continue;
            }
            if importance >= 0.8 {
                kept.push(m.clone());
                if m.is_decision {
                    decisions.push(m.content.chars().take(200).collect::<String>() + "...");
                }
                if m.is_error {
                    errors.push(m.content.chars().take(200).collect::<String>() + "...");
                }
            } else {
                let summary = Self::generate_tier_summary(&m, config);
                let mut summarized_msg = m.clone();
                summarized_msg.summary = Some(summary);
                kept.push(summarized_msg);
            }
        }

        let compacted_tokens: usize = kept.iter().map(|m| {
            if let Some(ref s) = m.summary {
                TokenEstimate::from_text(s).estimated_tokens
            } else {
                m.token_count
            }
        }).sum();

        CompactionResult {
            strategy_used: CompactionStrategy::Summary,
            original_token_count: original_tokens,
            compacted_token_count: compacted_tokens,
            compaction_ratio: if original_tokens > 0 {
                compacted_tokens as f64 / original_tokens as f64
            } else {
                0.0
            },
            messages_kept: kept.len(),
            messages_summarized: kept.iter().filter(|m| m.summary.is_some()).count(),
            messages_dropped: 0,
            hot_count: 0,
            warm_count: 0,
            cold_count: 0,
            compacted_messages: kept,
            summary: Some(format!(
                "摘要式压缩: {} → {} tokens",
                original_tokens, compacted_tokens
            )),
            decisions_preserved: decisions,
            errors_preserved: errors,
            truncated: compacted_tokens > config.max_tokens,
            duration_ms: 0,
        }
    }

    fn compact_hybrid(
        &self,
        request: &CompactionRequest,
        config: &CompactionConfig,
    ) -> CompactionResult {
        let mut result = self.compact_tiered(request, config);
        if result.truncated {
            let slim_request = CompactionRequest {
                session_id: request.session_id.clone(),
                messages: result.compacted_messages.clone(),
                config: Some(CompactionConfig {
                    strategy: CompactionStrategy::SlidingWindow,
                    ..config.clone()
                }),
                reason: Some(CompactionReason::TokenBudgetExceeded),
                system_prompt: request.system_prompt.clone(),
            };
            result = self.compact_sliding_window(&slim_request, config);
            result.strategy_used = CompactionStrategy::Hybrid;
        }
        result
    }

    fn generate_tier_summary(msg: &ConversationMessage, _config: &CompactionConfig) -> String {
        let prefix = match &msg.role {
            role if role == "user" => "[用户]",
            role if role == "assistant" => "[助手]",
            role if role == "system" => "[系统]",
            _ => "[其他]",
        };

        let content_preview = if msg.content.len() > 150 {
            format!("{}...", &msg.content[..150])
        } else {
            msg.content.clone()
        };

        let mut parts = vec![format!("{} {}", prefix, content_preview)];

        if !msg.file_changes.is_empty() {
            parts.push(format!(
                "(修改文件: {})",
                msg.file_changes.join(", ")
            ));
        }

        if msg.is_error {
            parts.push("⚠️ 含错误信息".to_string());
        }

        if msg.is_decision {
            parts.push("📋 含关键决策".to_string());
        }

        parts.join(" ")
    }

    pub fn get_session(&self, session_id: &str) -> Option<&CompactionSession> {
        self.sessions.get(session_id)
    }

    pub fn reset_session(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
    }
}