use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NewsCache {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub url: Option<String>,
    pub source: Option<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub category: Option<String>,
    pub published_at: Option<String>,
    pub fetched_at: i64,
    pub is_read: bool,
    pub is_favorite: bool,
    pub ai_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsSource {
    pub name: String,
    pub url: String,
    pub category: String,
    pub feed_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NewsSourceRow {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub url: String,
    pub category: String,
    pub feed_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NewsPendingDelete {
    pub id: i64,
    pub title: String,
    pub url: Option<String>,
    pub source: Option<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub category: Option<String>,
    pub published_at: Option<String>,
    pub original_fetched_at: i64,
    pub moved_at: i64,
    pub is_read: bool,
    pub is_favorite: bool,
}