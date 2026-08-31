use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::terminal::{SshProfile, SshProfileInput};

pub async fn list_profiles(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<SshProfile>, AppError> {
    sqlx::query_as::<_, SshProfile>(
        "SELECT * FROM ssh_profiles WHERE user_id = ? ORDER BY name ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn save_profile(
    pool: &SqlitePool,
    user_id: i64,
    input: &SshProfileInput,
    id: &str,
    now: i64,
) -> Result<SshProfile, AppError> {
    sqlx::query_as::<_, SshProfile>(
        "INSERT OR REPLACE INTO ssh_profiles (id, user_id, name, host, port, username, auth_type, private_key_path, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(id)
    .bind(user_id)
    .bind(&input.name)
    .bind(&input.host)
    .bind(input.port.unwrap_or(22))
    .bind(&input.username)
    .bind(&input.auth_type)
    .bind(&input.private_key_path)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_profile(
    pool: &SqlitePool,
    user_id: i64,
    id: &str,
) -> Result<u64, AppError> {
    let affected = sqlx::query("DELETE FROM ssh_profiles WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?
        .rows_affected();
    Ok(affected)
}

pub async fn get_profile(
    pool: &SqlitePool,
    user_id: i64,
    id: &str,
) -> Result<Option<SshProfile>, AppError> {
    sqlx::query_as::<_, SshProfile>("SELECT * FROM ssh_profiles WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}
