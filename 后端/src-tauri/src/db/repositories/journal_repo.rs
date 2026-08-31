use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::journal::Journal;

pub async fn get_journal_by_date(
    pool: &SqlitePool,
    user_id: i64,
    date: &str,
) -> Result<Option<Journal>, AppError> {
    sqlx::query_as::<_, Journal>(
        "SELECT * FROM journals WHERE user_id = ? AND date = ?",
    )
    .bind(user_id)
    .bind(date)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn upsert_journal(
    pool: &SqlitePool,
    user_id: i64,
    date: &str,
    content: &str,
    now: i64,
) -> Result<Journal, AppError> {
    sqlx::query_as::<_, Journal>(
        "INSERT INTO journals (user_id, date, content, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(user_id, date) DO UPDATE SET content = ?, updated_at = ?
         RETURNING *",
    )
    .bind(user_id)
    .bind(date)
    .bind(content)
    .bind(now)
    .bind(now)
    .bind(content)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_journal(
    pool: &SqlitePool,
    user_id: i64,
    date: &str,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM journals WHERE user_id = ? AND date = ?")
        .bind(user_id)
        .bind(date)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}
