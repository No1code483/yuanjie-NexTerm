use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserProfile {
    pub id: i64,
    pub field_key: String,
    pub field_value: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Resume {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Quote {
    pub id: i64,
    pub user_id: i64,
    pub content: String,
    pub source: Option<String>,
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteDuplicateResult {
    pub existing: Quote,
    pub match_type: String,
    pub match_detail: String,
}