use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::timer::Timer;

pub async fn get_all_timers(pool: &SqlitePool, user_id: i64) -> Result<Vec<Timer>, AppError> {
    sqlx::query_as::<_, Timer>(
        "SELECT * FROM timers WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn create_timer(
    pool: &SqlitePool,
    user_id: i64,
    name: Option<&str>,
    r#type: &str,
    target_time: Option<i64>,
    now: i64,
) -> Result<Timer, AppError> {
    sqlx::query_as::<_, Timer>(
        "INSERT INTO timers (user_id, name, type, target_time, is_running, elapsed, created_at, updated_at)
         VALUES (?, ?, ?, ?, 0, 0, ?, ?)
         RETURNING *",
    )
    .bind(user_id)
    .bind(name)
    .bind(r#type)
    .bind(target_time)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn update_timer_state(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    is_running: bool,
    elapsed: i64,
    now: i64,
) -> Result<Timer, AppError> {
    sqlx::query_as::<_, Timer>(
        "UPDATE timers SET is_running = ?, elapsed = ?, updated_at = ?
         WHERE user_id = ? AND id = ? RETURNING *",
    )
    .bind(is_running)
    .bind(elapsed)
    .bind(now)
    .bind(user_id)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_timer(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM timers WHERE user_id = ? AND id = ?")
        .bind(user_id)
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn reset_timer(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    now: i64,
) -> Result<Timer, AppError> {
    sqlx::query_as::<_, Timer>(
        "UPDATE timers SET is_running = 0, elapsed = 0, updated_at = ?
         WHERE user_id = ? AND id = ? RETURNING *",
    )
    .bind(now)
    .bind(user_id)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}
