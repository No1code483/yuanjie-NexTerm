use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::system::SystemConfig;

pub async fn get_config(pool: &SqlitePool, key: &str) -> Result<Option<SystemConfig>, AppError> {
    sqlx::query_as::<_, SystemConfig>("SELECT * FROM system_config WHERE config_key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn set_config(
    pool: &SqlitePool,
    key: &str,
    value: &str,
    now: i64,
) -> Result<SystemConfig, AppError> {
    sqlx::query_as::<_, SystemConfig>(
        "INSERT INTO system_config (config_key, config_value, updated_at)
         VALUES (?, ?, ?)
         ON CONFLICT(config_key) DO UPDATE SET config_value = ?, updated_at = ?
         RETURNING *",
    )
    .bind(key)
    .bind(value)
    .bind(now)
    .bind(value)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_all_configs(pool: &SqlitePool) -> Result<Vec<SystemConfig>, AppError> {
    sqlx::query_as::<_, SystemConfig>("SELECT * FROM system_config ORDER BY config_key ASC")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
}