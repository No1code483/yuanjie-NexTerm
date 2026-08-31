use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::intelligence_cross_module::Suggestion;

pub async fn insert_suggestion(
    pool: &SqlitePool,
    user_id: &str,
    category: &str,
    title: &str,
    description: &str,
    priority: &str,
    source: &str,
) -> Result<Suggestion, AppError> {
    sqlx::query_as::<_, Suggestion>(
        "INSERT INTO suggestions (user_id, category, title, description, priority, source)
         VALUES (?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(user_id)
    .bind(category)
    .bind(title)
    .bind(description)
    .bind(priority)
    .bind(source)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn query_suggestions(
    pool: &SqlitePool,
    user_id: &str,
    status: Option<&str>,
    category: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<(Vec<Suggestion>, i64), AppError> {
    let mut conditions = vec!["user_id = ?".to_string()];
    let mut params: Vec<String> = vec![user_id.to_string()];

    if let Some(s) = status {
        params.push(s.to_string());
        conditions.push(format!("status = ?{}", params.len()));
    }
    if let Some(c) = category {
        params.push(c.to_string());
        conditions.push(format!("category = ?{}", params.len()));
    }

    let where_clause = conditions.join(" AND ");
    let count_sql = format!("SELECT COUNT(*) FROM suggestions WHERE {}", where_clause);
    let data_sql = format!(
        "SELECT * FROM suggestions WHERE {} ORDER BY created_at DESC LIMIT {} OFFSET {}",
        where_clause, limit, offset
    );

    let total: i64 = {
        let mut q = sqlx::query_scalar::<_, i64>(&count_sql);
        for p in &params {
            q = q.bind(p);
        }
        q.fetch_one(pool).await.unwrap_or(0)
    };

    let suggestions: Vec<Suggestion> = {
        let mut q = sqlx::query_as::<_, Suggestion>(&data_sql);
        for p in &params {
            q = q.bind(p);
        }
        q.fetch_all(pool).await.unwrap_or_default()
    };

    Ok((suggestions, total))
}

pub async fn update_suggestion_status(
    pool: &SqlitePool,
    suggestion_id: i64,
    status: &str,
    user_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE suggestions SET status = ?, updated_at = datetime('now') WHERE id = ? AND user_id = ?",
    )
    .bind(status)
    .bind(suggestion_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn delete_suggestion(
    pool: &SqlitePool,
    suggestion_id: i64,
    user_id: &str,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM suggestions WHERE id = ? AND user_id = ?")
        .bind(suggestion_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn has_similar_suggestion(
    pool: &SqlitePool,
    user_id: &str,
    category: &str,
    title: &str,
) -> Result<bool, AppError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM suggestions
         WHERE user_id = ? AND category = ? AND title = ? AND created_at > datetime('now', '-3 days')",
    )
    .bind(user_id)
    .bind(category)
    .bind(title)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    Ok(count > 0)
}

pub async fn cleanup_old_suggestions(
    pool: &SqlitePool,
    retention_days: i64,
) -> Result<u64, AppError> {
    let result = sqlx::query(
        "DELETE FROM suggestions WHERE status = 'ignored' AND updated_at < datetime('now', ? || ' days')",
    )
    .bind(format!("-{}", retention_days))
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(result.rows_affected())
}