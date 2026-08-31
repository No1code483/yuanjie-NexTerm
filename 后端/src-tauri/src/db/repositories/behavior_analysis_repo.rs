use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::intelligence_cross_module::BehaviorPattern;

pub async fn insert_behavior_pattern(
    pool: &SqlitePool,
    pattern: &BehaviorPattern,
) -> Result<BehaviorPattern, AppError> {
    sqlx::query_as::<_, BehaviorPattern>(
        "INSERT INTO behavior_patterns (
            user_id, date, focus_score, distraction_count,
            kb_avg_duration_secs, kb_entry_count,
            error_operation_count, total_operation_count,
            active_start_hour, active_end_hour, peak_hour,
            module_diversity, consistency_score, summary
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT DO NOTHING
        RETURNING *",
    )
    .bind(&pattern.user_id)
    .bind(&pattern.date)
    .bind(pattern.focus_score)
    .bind(pattern.distraction_count)
    .bind(pattern.kb_avg_duration_secs)
    .bind(pattern.kb_entry_count)
    .bind(pattern.error_operation_count)
    .bind(pattern.total_operation_count)
    .bind(pattern.active_start_hour)
    .bind(pattern.active_end_hour)
    .bind(pattern.peak_hour)
    .bind(pattern.module_diversity)
    .bind(pattern.consistency_score)
    .bind(&pattern.summary)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_pattern_by_date(
    pool: &SqlitePool,
    user_id: &str,
    date: &str,
) -> Result<Option<BehaviorPattern>, AppError> {
    let pattern = sqlx::query_as::<_, BehaviorPattern>(
        "SELECT * FROM behavior_patterns WHERE user_id = ? AND date = ?",
    )
    .bind(user_id)
    .bind(date)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(pattern)
}

pub async fn query_patterns(
    pool: &SqlitePool,
    user_id: &str,
    start_date: &str,
    end_date: &str,
    limit: i64,
    offset: i64,
) -> Result<(Vec<BehaviorPattern>, i64), AppError> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM behavior_patterns
         WHERE user_id = ? AND date >= ? AND date <= ?",
    )
    .bind(user_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let patterns: Vec<BehaviorPattern> = sqlx::query_as::<_, BehaviorPattern>(
        "SELECT * FROM behavior_patterns
         WHERE user_id = ? AND date >= ? AND date <= ?
         ORDER BY date DESC LIMIT ? OFFSET ?",
    )
    .bind(user_id)
    .bind(start_date)
    .bind(end_date)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Ok((patterns, total))
}

pub async fn get_avg_patterns(
    pool: &SqlitePool,
    user_id: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Option<BehaviorPattern>, AppError> {
    let avg = sqlx::query_as::<_, BehaviorPattern>(
        "SELECT
            NULL as id,
            user_id,
            ? as date,
            AVG(focus_score) as focus_score,
            CAST(AVG(distraction_count) AS INTEGER) as distraction_count,
            CAST(AVG(kb_avg_duration_secs) AS INTEGER) as kb_avg_duration_secs,
            CAST(AVG(kb_entry_count) AS INTEGER) as kb_entry_count,
            CAST(AVG(error_operation_count) AS INTEGER) as error_operation_count,
            CAST(AVG(total_operation_count) AS INTEGER) as total_operation_count,
            NULL as active_start_hour,
            NULL as active_end_hour,
            NULL as peak_hour,
            CAST(AVG(module_diversity) AS INTEGER) as module_diversity,
            AVG(consistency_score) as consistency_score,
            NULL as summary,
            NULL as created_at
        FROM behavior_patterns
        WHERE user_id = ? AND date >= ? AND date <= ?",
    )
    .bind(end_date)
    .bind(user_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(avg)
}

pub async fn get_trend_data(
    pool: &SqlitePool,
    user_id: &str,
    days: i64,
) -> Result<Vec<BehaviorPattern>, AppError> {
    let patterns: Vec<BehaviorPattern> = sqlx::query_as::<_, BehaviorPattern>(
        "SELECT * FROM behavior_patterns
         WHERE user_id = ? AND date >= date('now', ? || ' days')
         ORDER BY date ASC",
    )
    .bind(user_id)
    .bind(format!("-{}", days))
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    Ok(patterns)
}

pub async fn has_pattern_today(
    pool: &SqlitePool,
    user_id: &str,
    date: &str,
) -> Result<bool, AppError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM behavior_patterns WHERE user_id = ? AND date = ?",
    )
    .bind(user_id)
    .bind(date)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    Ok(count > 0)
}