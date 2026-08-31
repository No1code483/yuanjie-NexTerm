use sqlx::SqlitePool;

use crate::db::repositories::timer_repo;
use crate::error::app_error::AppError;
use crate::models::timer::Timer;
use crate::services::intelligence_v4_service;

pub async fn get_timers(pool: &SqlitePool, user_id: i64) -> Result<Vec<Timer>, AppError> {
    timer_repo::get_all_timers(pool, user_id).await
}

pub async fn create_timer(
    pool: &SqlitePool,
    user_id: i64,
    name: Option<&str>,
    r#type: &str,
    target_time: Option<i64>,
) -> Result<Timer, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let result = timer_repo::create_timer(pool, user_id, name, r#type, target_time, now).await?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "timer", "create",
        Some(&format!("type: {}, name: {:?}", r#type, name)),
    ).await;
    Ok(result)
}

pub async fn update_timer_state(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    is_running: bool,
    elapsed: i64,
) -> Result<Timer, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let result = timer_repo::update_timer_state(pool, user_id, id, is_running, elapsed, now).await?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "timer",
        if is_running { "start" } else { "pause" },
        Some(&format!("id: {}, elapsed: {}", id, elapsed)),
    ).await;
    Ok(result)
}

pub async fn delete_timer(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    timer_repo::delete_timer(pool, user_id, id).await?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "timer", "delete",
        Some(&format!("id: {}", id)),
    ).await;
    Ok(())
}

pub async fn timer_action(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    action: &str,
) -> Result<Timer, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let result = match action {
        "start" => timer_repo::update_timer_state(pool, user_id, id, true, 0, now).await,
        "pause" => timer_repo::update_timer_state(pool, user_id, id, false, 0, now).await,
        "reset" => timer_repo::reset_timer(pool, user_id, id, now).await,
        _ => Err(AppError::Validation(format!("未知的计时器操作: {}", action))),
    }?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "timer", action,
        Some(&format!("id: {}", id)),
    ).await;
    Ok(result)
}
