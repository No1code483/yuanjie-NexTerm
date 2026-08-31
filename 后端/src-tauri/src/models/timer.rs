use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Timer {
    pub id: i64,
    pub user_id: i64,
    pub name: Option<String>,
    pub r#type: String,
    pub target_time: Option<i64>,
    pub is_running: bool,
    pub elapsed: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTimerRequest {
    pub name: Option<String>,
    pub r#type: String,
    pub target_time: Option<i64>,
}