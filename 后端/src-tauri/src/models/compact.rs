use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompactionStrategy {
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "tiered")]
    Tiered,
    #[serde(rename = "sliding_window")]
    SlidingWindow,
    #[serde(rename = "hybrid")]
    Hybrid,
}

impl CompactionStrategy {
    pub fn as_str(&self) -> &str {
        match self {
            CompactionStrategy::Summary => "summary",
            CompactionStrategy::Tiered => "tiered",
            CompactionStrategy::SlidingWindow => "sliding_window",
            CompactionStrategy::Hybrid => "hybrid",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "summary" => CompactionStrategy::Summary,
            "tiered" => CompactionStrategy::Tiered,
            "sliding_window" => CompactionStrategy::SlidingWindow,
            "hybrid" => CompactionStrategy::Hybrid,
            _ => CompactionStrategy::Tiered,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompactionReason {
    #[serde(rename = "token_budget_exceeded")]
    TokenBudgetExceeded,
    #[serde(rename = "user_requested")]
    UserRequested,
    #[serde(rename = "session_boundary")]
    SessionBoundary,
    #[serde(rename = "idle_timeout")]
    IdleTimeout,
    #[serde(rename = "pre_turn")]
    PreTurn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompactionTrigger {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "scheduled")]
    Scheduled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageTier {
    #[serde(rename = "hot")]
    Hot,
    #[serde(rename = "warm")]
    Warm,
    #[serde(rename = "cold")]
    Cold,
}

impl MessageTier {
    pub fn from_importance(importance: f64, age_seconds: f64, is_decision: bool) -> Self {
        if is_decision || importance >= 0.8 {
            MessageTier::Hot
        } else if importance >= 0.4 && age_seconds < 3600.0 {
            MessageTier::Warm
        } else {
            MessageTier::Cold
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    pub importance: f64,
    pub is_decision: bool,
    pub is_error: bool,
    pub file_changes: Vec<String>,
    pub token_count: usize,
    pub tier: Option<MessageTier>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    pub strategy: CompactionStrategy,
    pub max_tokens: usize,
    pub trigger_threshold: f64,
    pub target_ratio: f64,
    pub keep_last_n: usize,
    pub system_prompt_always: bool,
    pub preserve_decisions: bool,
    pub preserve_errors: bool,
    pub preserve_file_changes: bool,
    pub include_timestamp: bool,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            strategy: CompactionStrategy::Tiered,
            max_tokens: 128000,
            trigger_threshold: 0.85,
            target_ratio: 0.5,
            keep_last_n: 4,
            system_prompt_always: true,
            preserve_decisions: true,
            preserve_errors: true,
            preserve_file_changes: true,
            include_timestamp: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionRequest {
    pub session_id: String,
    pub messages: Vec<ConversationMessage>,
    pub config: Option<CompactionConfig>,
    pub reason: Option<CompactionReason>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    pub strategy_used: CompactionStrategy,
    pub original_token_count: usize,
    pub compacted_token_count: usize,
    pub compaction_ratio: f64,
    pub messages_kept: usize,
    pub messages_summarized: usize,
    pub messages_dropped: usize,
    pub hot_count: usize,
    pub warm_count: usize,
    pub cold_count: usize,
    pub compacted_messages: Vec<ConversationMessage>,
    pub summary: Option<String>,
    pub decisions_preserved: Vec<String>,
    pub errors_preserved: Vec<String>,
    pub truncated: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    pub max_tokens: usize,
    pub used_tokens: usize,
    pub available_tokens: usize,
    pub usage_ratio: f64,
    pub status: BudgetStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BudgetStatus {
    #[serde(rename = "safe")]
    Safe,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "critical")]
    Critical,
    #[serde(rename = "exhausted")]
    Exhausted,
}

impl BudgetStatus {
    pub fn from_ratio(ratio: f64) -> Self {
        if ratio >= 1.0 {
            BudgetStatus::Exhausted
        } else if ratio >= 0.85 {
            BudgetStatus::Critical
        } else if ratio >= 0.65 {
            BudgetStatus::Warning
        } else {
            BudgetStatus::Safe
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEstimate {
    pub char_count: usize,
    pub estimated_tokens: usize,
    pub method: String,
}

impl TokenEstimate {
    pub fn from_text(text: &str) -> Self {
        let chars = text.chars().count();
        let en_chars = text.chars().filter(|c| c.is_ascii()).count();
        let cjk_chars = chars - en_chars;

        let tokens = (en_chars / 4) + (cjk_chars * 2);
        Self {
            char_count: chars,
            estimated_tokens: tokens,
            method: "en_4_cjk_2".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionSession {
    pub session_id: String,
    pub compaction_count: usize,
    pub total_tokens_saved: usize,
    pub last_compaction_at: Option<i64>,
    pub strategy_used: Option<CompactionStrategy>,
}

pub fn message_importance(content: &str, role: &str, has_file_changes: bool) -> f64 {
    let mut score: f64 = 0.3;
    if role == "system" {
        score = 1.0;
    } else if role == "assistant" && has_file_changes {
        score += 0.4;
    } else if role == "user" {
        score += 0.2;
    }

    let lower = content.to_lowercase();
    if lower.contains("error") || lower.contains("fail") || lower.contains("bug") {
        score += 0.2;
    }
    if lower.contains("decide") || lower.contains("choose") || lower.contains("final") {
        score += 0.2;
    }
    if lower.contains("important") || lower.contains("critical") || lower.contains("must") {
        score += 0.1;
    }

    score.min(1.0)
}