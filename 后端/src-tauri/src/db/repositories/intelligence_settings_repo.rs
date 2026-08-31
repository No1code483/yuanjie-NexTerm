use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::intelligence_cross_module::IntelligenceSettings;

pub async fn get_settings(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Option<IntelligenceSettings>, AppError> {
    sqlx::query_as::<_, IntelligenceSettings>(
        "SELECT * FROM intelligence_settings WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn upsert_settings(
    pool: &SqlitePool,
    settings: &IntelligenceSettings,
) -> Result<IntelligenceSettings, AppError> {
    sqlx::query_as::<_, IntelligenceSettings>(
        "INSERT INTO intelligence_settings (
            user_id, log_retention_days, suggestion_retention_days,
            behavior_analysis_enabled, suggestion_enabled, use_llm_enhancement,
            llm_model, behavior_analysis_period, dashboard_default_period,
            activity_log_batch_size, show_productivity_score, show_behavior_analysis,
            show_suggestions, notification_frequency, extra_config
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(user_id) DO UPDATE SET
            log_retention_days = excluded.log_retention_days,
            suggestion_retention_days = excluded.suggestion_retention_days,
            behavior_analysis_enabled = excluded.behavior_analysis_enabled,
            suggestion_enabled = excluded.suggestion_enabled,
            use_llm_enhancement = excluded.use_llm_enhancement,
            llm_model = excluded.llm_model,
            behavior_analysis_period = excluded.behavior_analysis_period,
            dashboard_default_period = excluded.dashboard_default_period,
            activity_log_batch_size = excluded.activity_log_batch_size,
            show_productivity_score = excluded.show_productivity_score,
            show_behavior_analysis = excluded.show_behavior_analysis,
            show_suggestions = excluded.show_suggestions,
            notification_frequency = excluded.notification_frequency,
            extra_config = excluded.extra_config,
            updated_at = datetime('now')
        RETURNING *",
    )
    .bind(&settings.user_id)
    .bind(settings.log_retention_days)
    .bind(settings.suggestion_retention_days)
    .bind(settings.behavior_analysis_enabled)
    .bind(settings.suggestion_enabled)
    .bind(settings.use_llm_enhancement)
    .bind(&settings.llm_model)
    .bind(&settings.behavior_analysis_period)
    .bind(&settings.dashboard_default_period)
    .bind(settings.activity_log_batch_size)
    .bind(settings.show_productivity_score)
    .bind(settings.show_behavior_analysis)
    .bind(settings.show_suggestions)
    .bind(&settings.notification_frequency)
    .bind(&settings.extra_config)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_or_create_default(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<IntelligenceSettings, AppError> {
    let existing = get_settings(pool, user_id).await?;
    if let Some(s) = existing {
        return Ok(s);
    }

    sqlx::query_as::<_, IntelligenceSettings>(
        "INSERT INTO intelligence_settings (user_id) VALUES (?)
         ON CONFLICT(user_id) DO NOTHING
         RETURNING *",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::Internal("无法创建默认设置".into()))
}

pub async fn delete_settings(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM intelligence_settings WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}