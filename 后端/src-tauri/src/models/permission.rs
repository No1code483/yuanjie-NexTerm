use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Permission {
    pub id: i64,
    pub role: String,
    pub resource: String,
    pub can_read: bool,
    pub can_write: bool,
    pub can_delete: bool,
    pub can_modify: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionCheckRequest {
    pub resource: String,
    pub action: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionCheckResponse {
    pub allowed: bool,
}