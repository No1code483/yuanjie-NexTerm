use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRoleConfig {
    pub name: String,
    pub system_prompt: String,
    pub available_tools: Vec<String>,
    pub permission_level: PermissionLevel,
    pub max_turns: u32,
    pub token_budget: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PermissionLevel {
    Full,
    Restricted,
    ReadOnly,
}

impl PermissionLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "full" => PermissionLevel::Full,
            "restricted" => PermissionLevel::Restricted,
            "readonly" | "read_only" => PermissionLevel::ReadOnly,
            _ => PermissionLevel::Restricted,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            PermissionLevel::Full => "full",
            PermissionLevel::Restricted => "restricted",
            PermissionLevel::ReadOnly => "read_only",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpawnRequest {
    pub session_id: String,
    pub role: AgentRoleConfig,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessageRequest {
    pub from_agent_id: String,
    pub to_agent_id: String,
    pub content: String,
    pub message_type: AgentMessageType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentMessageType {
    Task,
    Result,
    Question,
    Notification,
}

impl AgentMessageType {
    pub fn as_str(&self) -> &str {
        match self {
            AgentMessageType::Task => "task",
            AgentMessageType::Result => "result",
            AgentMessageType::Question => "question",
            AgentMessageType::Notification => "notification",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub agent_id: String,
    pub role_name: String,
    pub status: String,
    pub parent_id: Option<String>,
    pub spawned_at: i64,
    pub turns_executed: u32,
    pub tokens_used: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveAgentSnapshot {
    pub agent_id: String,
    pub session_id: String,
    pub role_name: String,
    pub status: String,
    pub parent_id: Option<String>,
    pub max_turns: u32,
    pub turns_executed: u32,
    pub token_budget: Option<i64>,
    pub tokens_used: i64,
    pub spawned_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDeployItem {
    pub id: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub max_iterations: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDeployRequest {
    pub agents: Vec<AgentDeployItem>,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDeployResponse {
    pub success: bool,
    pub message: String,
    pub deployed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentListResponse {
    pub session_id: String,
    pub agents: Vec<LiveAgentSnapshot>,
    pub total_count: usize,
}