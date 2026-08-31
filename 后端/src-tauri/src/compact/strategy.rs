use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::models::compact::{
    CompactionConfig, ConversationMessage, MessageTier,
};

/// 智能压缩策略引擎
/// 对标 Codex 的 compact.rs: 消息分级(Tier) + 摘要生成 + 决策保留
pub struct CompactStrategy;

/// 消息分级结果
#[derive(Debug, Clone)]
pub struct TierClassification {
    pub hot: Vec<ConversationMessage>,
    pub warm: Vec<ConversationMessage>,
    pub cold: Vec<ConversationMessage>,
    pub stats: TierStats,
}

#[derive(Debug, Clone, Default)]
pub struct TierStats {
    pub hot_count: usize,
    pub warm_count: usize,
    pub cold_count: usize,
    pub hot_tokens: usize,
    pub warm_tokens: usize,
    pub cold_tokens: usize,
}

/// 摘要生成配置
#[derive(Debug, Clone)]
pub struct SummaryConfig {
    pub max_summary_tokens: usize,
    pub preserve_decisions: bool,
    pub preserve_errors: bool,
    pub include_system_prompt: bool,
}

impl Default for SummaryConfig {
    fn default() -> Self {
        Self {
            max_summary_tokens: 2000,
            preserve_decisions: true,
            preserve_errors: true,
            include_system_prompt: true,
        }
    }
}

impl CompactStrategy {
    /// 将消息按 Tier 分级
    pub fn classify_tiers(
        messages: &[ConversationMessage],
        now_ms: i64,
    ) -> TierClassification {
        let mut classification = TierClassification {
            hot: Vec::new(),
            warm: Vec::new(),
            cold: Vec::new(),
            stats: TierStats::default(),
        };

        for msg in messages {
            let age = ((now_ms - msg.timestamp).max(0) as f64) / 1000.0;
            let tier = MessageTier::from_importance(msg.importance, age, msg.is_decision);

            match tier {
                MessageTier::Hot => {
                    classification.stats.hot_count += 1;
                    classification.stats.hot_tokens += msg.token_count;
                    classification.hot.push(msg.clone());
                }
                MessageTier::Warm => {
                    classification.stats.warm_count += 1;
                    classification.stats.warm_tokens += msg.token_count;
                    classification.warm.push(msg.clone());
                }
                MessageTier::Cold => {
                    classification.stats.cold_count += 1;
                    classification.stats.cold_tokens += msg.token_count;
                    classification.cold.push(msg.clone());
                }
            }
        }

        classification
    }

    /// 生成 Tier 摘要
    pub fn generate_tier_summary(
        messages: &[ConversationMessage],
        tier: &MessageTier,
        config: &SummaryConfig,
    ) -> String {
        if messages.is_empty() {
            return String::new();
        }

        let tier_label = match tier {
            MessageTier::Hot => "核心",
            MessageTier::Warm => "一般",
            MessageTier::Cold => "冷区",
        };

        let mut summary = String::new();
        summary.push_str(&format!(
            "\n<{}对话摘要>\n", tier_label
        ));

        // 决策保留
        if config.preserve_decisions {
            let decisions: Vec<&str> = messages
                .iter()
                .filter(|m| m.is_decision)
                .map(|m| m.content.as_str())
                .collect();

            if !decisions.is_empty() {
                summary.push_str("\n关键决策:\n");
                for d in &decisions {
                    summary.push_str(&format!("- {}\n",
                        &d[..d.len().min(200)]
                    ));
                }
            }
        }

        // 错误保留
        if config.preserve_errors {
            let errors: Vec<&str> = messages
                .iter()
                .filter(|m| m.is_error)
                .map(|m| m.content.as_str())
                .collect();

            if !errors.is_empty() {
                summary.push_str("\n错误记录:\n");
                for e in &errors {
                    summary.push_str(&format!("- {}\n",
                        &e[..e.len().min(200)]
                    ));
                }
            }
        }

        // 对话流程摘要
        let user_msgs: Vec<&str> = messages
            .iter()
            .filter(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .collect();

        if !user_msgs.is_empty() {
            summary.push_str("\n用户请求:\n");
            for (i, msg) in user_msgs.iter().enumerate() {
                if i >= 5 {
                    summary.push_str(&format!("... 还有 {} 条消息\n",
                        user_msgs.len() - 5
                    ));
                    break;
                }
                summary.push_str(&format!("- {}\n",
                    &msg[..msg.len().min(150)]
                ));
            }
        }

        summary.push_str(&format!("</{}对话摘要>\n", tier_label));

        // 截断
        if summary.len() > config.max_summary_tokens * 4 {
            summary = summary[..config.max_summary_tokens * 4].to_string();
            summary.push_str("...");
        }

        summary
    }

    /// 计算压缩策略的最优参数
    pub fn optimize_compression(
        config: &CompactionConfig,
        classification: &TierClassification,
    ) -> CompressionPlan {
        let total_tokens = classification.stats.hot_tokens
            + classification.stats.warm_tokens
            + classification.stats.cold_tokens;

        let target_tokens = (config.max_tokens as f64 * config.target_ratio) as usize;

        if total_tokens <= target_tokens {
            return CompressionPlan {
                keep_hot: classification.hot.len(),
                keep_warm: classification.warm.len(),
                keep_cold: classification.cold.len(),
                summarize_warm: 0,
                summarize_cold: 0,
                drop_cold: 0,
                estimated_result_tokens: total_tokens,
            };
        }

        // 优先保留 Hot
        let mut plan = CompressionPlan {
            keep_hot: classification.hot.len(),
            keep_warm: 0,
            keep_cold: 0,
            summarize_warm: 0,
            summarize_cold: 0,
            drop_cold: 0,
            estimated_result_tokens: classification.stats.hot_tokens,
        };

        let remaining = target_tokens.saturating_sub(classification.stats.hot_tokens);

        // 保留最近的 Warm 消息
        let warm_capacity = remaining / 2;
        let mut warm_tokens = 0usize;
        for (i, msg) in classification.warm.iter().enumerate() {
            if warm_tokens + msg.token_count > warm_capacity {
                plan.keep_warm = i;
                plan.summarize_warm = classification.warm.len() - i;
                break;
            }
            warm_tokens += msg.token_count;
            plan.keep_warm = i + 1;
        }
        plan.estimated_result_tokens += warm_tokens;

        // 剩余预算给 Cold 消息
        let cold_capacity = remaining.saturating_sub(warm_tokens);
        let mut cold_tokens = 0usize;
        for (i, msg) in classification.cold.iter().enumerate() {
            if cold_tokens + msg.token_count > cold_capacity {
                plan.keep_cold = i;
                plan.summarize_cold = classification.cold.len() - i;
                plan.drop_cold = 0;
                break;
            }
            cold_tokens += msg.token_count;
            plan.keep_cold = i + 1;
        }
        plan.estimated_result_tokens += cold_tokens;

        plan
    }

    /// 合并消息并生成摘要
    pub fn merge_with_summaries(
        classification: &TierClassification,
        plan: &CompressionPlan,
        config: &SummaryConfig,
    ) -> (Vec<ConversationMessage>, Option<String>) {
        let mut result = Vec::new();
        let mut full_summary = String::new();

        // Hot: 全部保留
        for msg in &classification.hot[..plan.keep_hot.min(classification.hot.len())] {
            result.push(msg.clone());
        }

        // Warm: 部分保留 + 摘要
        if plan.keep_warm < classification.warm.len() {
            let summarized = &classification.warm[plan.keep_warm..];
            let summary = Self::generate_tier_summary(summarized, &MessageTier::Warm, config);
            full_summary.push_str(&summary);
        }
        for msg in &classification.warm[..plan.keep_warm.min(classification.warm.len())] {
            result.push(msg.clone());
        }

        // Cold: 少量保留 + 摘要
        if plan.keep_cold < classification.cold.len() {
            let summarized = &classification.cold[plan.keep_cold..];
            let summary = Self::generate_tier_summary(summarized, &MessageTier::Cold, config);
            full_summary.push_str(&summary);
        }
        for msg in &classification.cold[..plan.keep_cold.min(classification.cold.len())] {
            result.push(msg.clone());
        }

        let final_summary = if full_summary.is_empty() {
            None
        } else {
            Some(full_summary)
        };

        (result, final_summary)
    }

    /// 统计上下文窗口使用情况
    pub fn context_stats(messages: &[ConversationMessage]) -> ContextStats {
        let total_tokens: usize = messages.iter().map(|m| m.token_count).sum();
        let mut role_tokens: HashMap<String, usize> = HashMap::new();
        let mut tier_tokens: HashMap<String, usize> = HashMap::new();

        for msg in messages {
            *role_tokens.entry(msg.role.clone()).or_default() += msg.token_count;
            if let Some(ref tier) = msg.tier {
                let tier_key = match tier {
                    MessageTier::Hot => "hot",
                    MessageTier::Warm => "warm",
                    MessageTier::Cold => "cold",
                };
                *tier_tokens.entry(tier_key.to_string()).or_default() += msg.token_count;
            }
        }

        ContextStats {
            total_tokens,
            message_count: messages.len(),
            role_tokens,
            tier_tokens,
            decisions_count: messages.iter().filter(|m| m.is_decision).count(),
            errors_count: messages.iter().filter(|m| m.is_error).count(),
        }
    }
}

/// 压缩计划
#[derive(Debug, Clone)]
pub struct CompressionPlan {
    pub keep_hot: usize,
    pub keep_warm: usize,
    pub keep_cold: usize,
    pub summarize_warm: usize,
    pub summarize_cold: usize,
    pub drop_cold: usize,
    pub estimated_result_tokens: usize,
}

/// 上下文统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStats {
    pub total_tokens: usize,
    pub message_count: usize,
    pub role_tokens: HashMap<String, usize>,
    pub tier_tokens: HashMap<String, usize>,
    pub decisions_count: usize,
    pub errors_count: usize,
}