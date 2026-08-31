use sqlx::SqlitePool;

use crate::db::repositories::journal_repo;
use crate::error::app_error::AppError;
use crate::models::journal::Journal;
use crate::services::intelligence_v4_service;

pub async fn get_journal(
    pool: &SqlitePool,
    user_id: i64,
    date: &str,
) -> Result<Option<Journal>, AppError> {
    journal_repo::get_journal_by_date(pool, user_id, date).await
}

pub async fn save_journal(
    pool: &SqlitePool,
    user_id: i64,
    date: &str,
    content: &str,
) -> Result<Journal, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let result = journal_repo::upsert_journal(pool, user_id, date, content, now).await?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "journal", "save",
        Some(date),
    ).await;
    Ok(result)
}

pub async fn delete_journal(
    pool: &SqlitePool,
    user_id: i64,
    date: &str,
) -> Result<(), AppError> {
    journal_repo::delete_journal(pool, user_id, date).await?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "journal", "delete",
        Some(date),
    ).await;
    Ok(())
}
