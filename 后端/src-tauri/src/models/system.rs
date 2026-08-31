use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemConfig {
    pub id: i64,
    pub config_key: String,
    pub config_value: String,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetConfigRequest {
    pub key: String,
    pub value: String,
}