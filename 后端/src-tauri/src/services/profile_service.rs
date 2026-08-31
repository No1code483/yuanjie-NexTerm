use sqlx::SqlitePool;

use crate::db::repositories::profile_repo;
use crate::error::app_error::AppError;
use crate::models::profile::{Quote, QuoteDuplicateResult, Resume, UserProfile};
use crate::services::intelligence_v4_service;

pub async fn get_profile(pool: &SqlitePool, key: &str) -> Result<Option<UserProfile>, AppError> {
    profile_repo::get_profile(pool, key).await
}

pub async fn set_profile(pool: &SqlitePool, key: &str, value: &str) -> Result<UserProfile, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let result = profile_repo::set_profile(pool, key, value, now).await?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "profile", "save",
        Some(key),
    ).await;
    Ok(result)
}

pub async fn get_resumes(pool: &SqlitePool, user_id: i64) -> Result<Vec<Resume>, AppError> {
    profile_repo::get_resumes(pool, user_id).await
}

pub async fn add_resume(pool: &SqlitePool, user_id: i64, title: &str, content: &str) -> Result<Resume, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let result = profile_repo::add_resume(pool, user_id, title, content, now).await?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "resume", "create",
        Some(title),
    ).await;
    Ok(result)
}

pub async fn update_resume(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    title: &str,
    content: &str,
) -> Result<Resume, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let result = profile_repo::update_resume(pool, user_id, id, title, content, now).await?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "resume", "edit",
        Some(title),
    ).await;
    Ok(result)
}

pub async fn delete_resume(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    profile_repo::delete_resume(pool, user_id, id).await?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "resume", "delete",
        Some(&format!("id: {}", id)),
    ).await;
    Ok(())
}

pub async fn get_random_quote(pool: &SqlitePool, user_id: i64) -> Result<Option<Quote>, AppError> {
    profile_repo::get_random_quote(pool, user_id).await
}

pub async fn add_quote(
    pool: &SqlitePool,
    user_id: i64,
    content: &str,
    source: Option<&str>,
    quote_type: &str,
) -> Result<Quote, AppError> {
    let result = profile_repo::add_quote(pool, user_id, content, source, quote_type).await?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "quote", "add",
        Some(&format!("type: {}", quote_type)),
    ).await;
    Ok(result)
}

pub async fn get_all_quotes(pool: &SqlitePool, user_id: i64) -> Result<Vec<Quote>, AppError> {
    profile_repo::get_all_quotes(pool, user_id).await
}

pub async fn batch_add_quotes(
    pool: &SqlitePool,
    user_id: i64,
    quotes: &[(String, Option<String>, String)],
) -> Result<Vec<Quote>, AppError> {
    let result = profile_repo::batch_add_quotes(pool, user_id, quotes).await?;
    let count = quotes.len();
    let _ = intelligence_v4_service::instrument(
        pool, "system", "quote", "batch_add",
        Some(&format!("count: {}", count)),
    ).await;
    Ok(result)
}

pub async fn delete_all_quotes(pool: &SqlitePool, user_id: i64) -> Result<u64, AppError> {
    let result = profile_repo::delete_all_quotes(pool, user_id).await?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "quote", "delete_all",
        None,
    ).await;
    Ok(result)
}

pub async fn delete_quote_by_id(pool: &SqlitePool, user_id: i64, id: i64) -> Result<u64, AppError> {
    let result = profile_repo::delete_quote_by_id(pool, user_id, id).await?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "quote", "delete",
        Some(&format!("id: {}", id)),
    ).await;
    Ok(result)
}

pub async fn get_personal_info(pool: &SqlitePool) -> Result<Option<String>, AppError> {
    profile_repo::get_personal_info(pool).await
}

pub async fn save_personal_info(pool: &SqlitePool, json: &str) -> Result<(), AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    profile_repo::save_personal_info(pool, json, now).await?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "profile", "save_personal_info",
        None,
    ).await;
    Ok(())
}

pub async fn check_quote_duplicate(
    pool: &SqlitePool,
    user_id: i64,
    content: &str,
) -> Result<Vec<QuoteDuplicateResult>, AppError> {
    profile_repo::find_similar_quotes(pool, user_id, content).await
}