use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Journal {
    pub id: i64,
    pub user_id: i64,
    pub date: String,
    pub content: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}