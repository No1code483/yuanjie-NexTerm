use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Conversation {
    pub id: i64,
    pub user_id: i64,
    pub title: Option<String>,
    pub r#type: String,
    pub is_temp: bool,
    pub dissolve_at: Option<i64>,
    pub token_budget: i64,
    pub starred: Option<bool>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConversationParticipant {
    pub id: i64,
    pub conversation_id: i64,
    pub model_id: Option<i64>,
    pub agent_id: Option<i64>,
    pub role: String,
    pub user_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: i64,
    pub conversation_id: i64,
    pub user_id: i64,
    pub sender_type: String,
    pub sender_id: Option<i64>,
    pub content: String,
    pub round: i32,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
    pub r#type: String,
    pub is_temp: bool,
    pub token_budget: Option<i64>,
    pub participants: Vec<ParticipantInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParticipantInfo {
    pub model_id: Option<i64>,
    pub agent_id: Option<i64>,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub conversation_id: i64,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateConversationRequest {
    pub id: i64,
    pub title: Option<String>,
    pub token_budget: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHealthStatus {
    pub model_id: i64,
    pub name: String,
    pub provider: String,
    pub is_online: bool,
    pub latency_ms: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub max_rounds: u32,
    pub timeout_secs: u64,
    pub convergence_rounds: u32,
    pub token_budget: i64,
    pub token_warn_ratio: f64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_rounds: 10,
            timeout_secs: 60,
            convergence_rounds: 2,
            token_budget: 50000,
            token_warn_ratio: 0.8,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrchestratorResult {
    pub total_rounds: u32,
    pub total_tokens: i64,
    pub summary: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchConversationsRequest {
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BranchConversationRequest {
    pub conversation_id: i64,
    pub message_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportConversationRequest {
    pub conversation_id: i64,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PromptTemplate {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub category: String,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePromptTemplateRequest {
    pub title: String,
    pub category: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePromptTemplateRequest {
    pub id: i64,
    pub title: Option<String>,
    pub category: Option<String>,
    pub content: Option<String>,
}