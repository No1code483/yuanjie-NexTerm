use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KbCategory {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub library: String,
    pub sort_order: i32,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KbEntry {
    pub id: i64,
    pub user_id: i64,
    pub category_id: i64,
    pub name: String,
    pub path_url: String,
    pub entry_type: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_favorited: Option<i64>,
    pub content: Option<String>,
    pub source_path: Option<String>,
    #[sqlx(default)]
    #[serde(default = "default_true")]
    pub file_exists: bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KbTag {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub color: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbEntryWithTags {
    pub id: i64,
    pub category_id: i64,
    pub name: String,
    pub path_url: String,
    pub entry_type: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_favorited: bool,
    pub tags: Vec<KbTag>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddKbEntryRequest {
    pub category_id: i64,
    pub name: String,
    pub path_url: String,
    pub entry_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateKbEntryRequest {
    pub id: i64,
    pub name: Option<String>,
    pub path_url: Option<String>,
    pub entry_type: Option<String>,
    pub category_id: Option<i64>,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryTagsResult {
    pub entry_id: i64,
    pub tags: Vec<KbTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagStats {
    pub tag_id: i64,
    pub tag_name: String,
    pub tag_color: String,
    pub entry_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanDirFileInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub extension: String,
    pub file_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KbTrackedPath {
    pub id: i64,
    pub user_id: i64,
    pub path: String,
    pub category_id: i64,
    pub library: String,
    pub last_imported_at: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KbTemplate {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub icon: String,
    pub description: String,
    pub entry_type: String,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateKbTemplateRequest {
    pub name: String,
    pub icon: String,
    pub description: String,
    pub entry_type: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateKbTemplateRequest {
    pub id: i64,
    pub name: Option<String>,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub entry_type: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathCheckResult {
    pub total_tracked: usize,
    pub valid_tracked: usize,
    pub invalid_tracked: Vec<i64>,
    pub total_external_entries: usize,
    pub valid_external_entries: usize,
    pub invalid_external_entries: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KbReference {
    pub id: i64,
    pub user_id: i64,
    pub source_entry_id: i64,
    pub target_entry_id: i64,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BacklinkResult {
    pub entry: KbEntry,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KbSnapshot {
    pub id: i64,
    pub user_id: i64,
    pub entry_id: i64,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KbSnapshotResult {
    pub id: i64,
    pub entry_id: i64,
    pub entry_name: String,
    pub content: String,
    pub content_length: usize,
    pub created_at: i64,
}