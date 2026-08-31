use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecycleBinItem {
    pub id: i64,
    pub user_id: i64,
    pub original_path: String,
    pub item_type: String,
    pub item_id: Option<i64>,
    pub title: Option<String>,
    pub metadata_json: Option<String>,
    pub file_size: Option<i64>,
    pub deleted_by: Option<i64>,
    pub deleted_at: i64,
    pub auto_delete_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MoveToRecycleRequest {
    pub item_type: String,
    pub item_ids: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchIdsRequest {
    pub ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecycleStats {
    pub total_size: i64,
    pub file_count: i64,
    pub oldest_date: i64,
    pub expiring_count: i64,
}