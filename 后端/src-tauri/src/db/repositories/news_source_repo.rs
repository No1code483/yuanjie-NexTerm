use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::news::NewsSourceRow;

pub async fn get_all_news_sources(pool: &SqlitePool, user_id: i64) -> Result<Vec<NewsSourceRow>, AppError> {
    sqlx::query_as::<_, NewsSourceRow>(
        "SELECT * FROM news_sources WHERE user_id = ? ORDER BY id ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn add_news_source(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    url: &str,
    category: &str,
    feed_type: &str,
) -> Result<NewsSourceRow, AppError> {
    sqlx::query_as::<_, NewsSourceRow>(
        "INSERT INTO news_sources (user_id, name, url, category, feed_type) VALUES (?, ?, ?, ?, ?) RETURNING *",
    )
    .bind(user_id)
    .bind(name)
    .bind(url)
    .bind(category)
    .bind(feed_type)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_news_source(pool: &SqlitePool, user_id: i64, id: i64) -> Result<bool, AppError> {
    let result = sqlx::query("DELETE FROM news_sources WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

    Ok(result.rows_affected() > 0)
}
