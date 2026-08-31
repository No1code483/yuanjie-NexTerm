use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct YuanGoal {
    pub id: i64,
    pub goal_uuid: String,
    pub session_id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: i32,
    pub progress_pct: i32,
    pub parent_goal_id: Option<i64>,
    pub agent_id: Option<String>,
    pub token_budget: Option<i64>,
    pub tokens_used: i64,
    pub thread_json: String,
    pub result_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalStatus {
    Pending,
    InProgress,
    Paused,
    Completed,
    Aborted,
}

impl GoalStatus {
    pub fn as_str(&self) -> &str {
        match self {
            GoalStatus::Pending => "pending",
            GoalStatus::InProgress => "in_progress",
            GoalStatus::Paused => "paused",
            GoalStatus::Completed => "completed",
            GoalStatus::Aborted => "aborted",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => GoalStatus::Pending,
            "in_progress" => GoalStatus::InProgress,
            "paused" => GoalStatus::Paused,
            "completed" => GoalStatus::Completed,
            "aborted" => GoalStatus::Aborted,
            _ => GoalStatus::Pending,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, GoalStatus::Completed | GoalStatus::Aborted)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, GoalStatus::InProgress)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGoalRequest {
    pub session_id: String,
    pub title: String,
    pub description: String,
    pub priority: Option<i32>,
    pub parent_goal_id: Option<i64>,
    pub agent_id: Option<String>,
    pub token_budget: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGoalRequest {
    pub goal_id: i64,
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<i32>,
    pub token_budget: Option<i64>,
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSnapshot {
    pub goal_uuid: String,
    pub session_id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: i32,
    pub progress_pct: i32,
    pub parent_goal_id: Option<i64>,
    pub agent_id: Option<String>,
    pub token_budget: Option<i64>,
    pub tokens_used: i64,
    pub token_pct: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<YuanGoal> for GoalSnapshot {
    fn from(g: YuanGoal) -> Self {
        let token_pct = match g.token_budget {
            Some(b) if b > 0 => (g.tokens_used as f64 / b as f64) * 100.0,
            _ => 0.0,
        };
        GoalSnapshot {
            goal_uuid: g.goal_uuid,
            session_id: g.session_id,
            title: g.title,
            description: g.description,
            status: g.status,
            priority: g.priority,
            progress_pct: g.progress_pct,
            parent_goal_id: g.parent_goal_id,
            agent_id: g.agent_id,
            token_budget: g.token_budget,
            tokens_used: g.tokens_used,
            token_pct,
            created_at: g.created_at,
            updated_at: g.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalListResponse {
    pub session_id: String,
    pub goals: Vec<GoalSnapshot>,
    pub total: usize,
    pub active: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConsumeRequest {
    pub goal_id: i64,
    pub tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalContinuationData {
    pub goal_id: i64,
    pub last_context: String,
    pub parent_agent_id: Option<String>,
    pub checkpoint_data: Option<String>,
}