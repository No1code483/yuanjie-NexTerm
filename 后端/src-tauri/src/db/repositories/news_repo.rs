use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::news::{NewsCache, NewsPendingDelete};

pub async fn get_news(pool: &SqlitePool, user_id: i64) -> Result<Vec<NewsCache>, AppError> {
    sqlx::query_as::<_, NewsCache>(
        "SELECT * FROM news_cache WHERE user_id = ? ORDER BY COALESCE(CAST(strftime('%s', published_at) AS INTEGER), fetched_at) DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_news_by_id(pool: &SqlitePool, user_id: i64, id: i64) -> Result<NewsCache, AppError> {
    sqlx::query_as::<_, NewsCache>("SELECT * FROM news_cache WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
}

fn get_category_group(category: &str) -> Vec<&str> {
    match category {
        "security" => vec!["security", "vulnerability", "attack_defense"],
        "ai" => vec!["ai", "tech_innovation"],
        "programming" => vec!["programming", "tool_application", "cloud_native", "open_source"],
        "github" => vec!["github"],
        _ => vec![category],
    }
}

pub async fn get_news_by_category(
    pool: &SqlitePool,
    user_id: i64,
    category: &str,
) -> Result<Vec<NewsCache>, AppError> {
    let categories = get_category_group(category);
    let placeholders = categories.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT * FROM news_cache WHERE user_id = ? AND category IN ({}) ORDER BY COALESCE(CAST(strftime('%s', published_at) AS INTEGER), fetched_at) DESC",
        placeholders
    );

    let mut query = sqlx::query_as::<_, NewsCache>(&sql);
    query = query.bind(user_id);
    for cat in &categories {
        query = query.bind(cat);
    }

    query
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn add_news(
    pool: &SqlitePool,
    user_id: i64,
    title: &str,
    url: Option<&str>,
    source: Option<&str>,
    summary: Option<&str>,
    now: i64,
) -> Result<NewsCache, AppError> {
    sqlx::query_as::<_, NewsCache>(
        "INSERT INTO news_cache (user_id, title, url, source, summary, fetched_at, is_read)
         VALUES (?, ?, ?, ?, ?, ?, 0)
         RETURNING *",
    )
    .bind(user_id)
    .bind(title)
    .bind(url)
    .bind(source)
    .bind(summary)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn add_news_full(
    pool: &SqlitePool,
    user_id: i64,
    title: &str,
    url: Option<&str>,
    source: Option<&str>,
    summary: Option<&str>,
    content: Option<&str>,
    category: Option<&str>,
    published_at: Option<&str>,
    now: i64,
) -> Result<NewsCache, AppError> {
    sqlx::query_as::<_, NewsCache>(
        "INSERT INTO news_cache (user_id, title, url, source, summary, content, category, published_at, fetched_at, is_read)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0)
         RETURNING *",
    )
    .bind(user_id)
    .bind(title)
    .bind(url)
    .bind(source)
    .bind(summary)
    .bind(content)
    .bind(category)
    .bind(published_at)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn exists_by_url(pool: &SqlitePool, user_id: i64, url: &str) -> Result<bool, AppError> {
    if url.is_empty() {
        return Ok(false);
    }

    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM news_cache WHERE url = ? AND user_id = ?",
    )
    .bind(url)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(count.0 > 0)
}

pub async fn exists_by_title_and_source(pool: &SqlitePool, user_id: i64, title: &str, source: &str) -> Result<bool, AppError> {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM news_cache WHERE title = ? AND source = ? AND user_id = ?",
    )
    .bind(title)
    .bind(source)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(count.0 > 0)
}

pub async fn remove_duplicate_news(pool: &SqlitePool) -> Result<u64, AppError> {
    let result = sqlx::query(
        "DELETE FROM news_cache WHERE id NOT IN (
            SELECT MIN(id) FROM news_cache 
            GROUP BY COALESCE(url, '') || '|' || COALESCE(source, '') || '|' || title
        ) AND id IN (
            SELECT id FROM news_cache n1 
            WHERE EXISTS (
                SELECT 1 FROM news_cache n2 
                WHERE n2.id < n1.id 
                AND COALESCE(n2.url, '') || '|' || COALESCE(n2.source, '') || '|' || n2.title 
                = COALESCE(n1.url, '') || '|' || COALESCE(n1.source, '') || '|' || n1.title
            )
        )"
    )
    .execute(pool)
    .await
    .map(|r| r.rows_affected())
    .map_err(AppError::Database)?;

    Ok(result)
}

pub async fn delete_expired_non_favorite(pool: &SqlitePool, cutoff_timestamp: i64) -> Result<u64, AppError> {
    let cutoff_dt_utc = chrono::DateTime::from_timestamp_millis(cutoff_timestamp)
        .unwrap_or_else(|| chrono::Utc::now());
    let cutoff_str = cutoff_dt_utc.format("%Y-%m-%d %H:%M:%S").to_string();

    tracing::info!(
        cutoff_ts = cutoff_timestamp,
        cutoff_utc = %cutoff_str,
        "🔍 [delete_expired] 截止时间(UTC): {}",
        cutoff_str
    );

    let result = sqlx::query(
        "DELETE FROM news_cache
         WHERE is_favorite = 0
         AND (
             (published_at IS NOT NULL AND published_at < ?)
             OR
             (published_at IS NULL AND fetched_at < ?)
         )",
    )
    .bind(&cutoff_str)
    .bind(cutoff_timestamp)
    .execute(pool)
    .await
    .map(|r| r.rows_affected())
    .map_err(AppError::Database)?;

    if result > 0 {
        tracing::info!(deleted = result, "✅ [delete_expired] 已删除{}条过期新闻", result);
    }

    Ok(result)
}

pub async fn count_news(pool: &SqlitePool, user_id: i64) -> Result<i64, AppError> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM news_cache WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(count.0)
}

pub async fn count_favorite_news(pool: &SqlitePool, user_id: i64) -> Result<i64, AppError> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM news_cache WHERE is_favorite = 1 AND user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(count.0)
}

pub async fn get_oldest_news_timestamp(pool: &SqlitePool) -> Result<Option<i64>, AppError> {
    let row = sqlx::query_as::<_, (Option<String>, Option<i64>)>(
        "SELECT published_at, fetched_at FROM news_cache WHERE is_favorite = 0 ORDER BY COALESCE(published_at, '9999-12-31') ASC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    match row {
        Some(r) => {
            tracing::info!(
                published = ?r.0,
                fetched = ?r.1,
                "🔍 [诊断] 最旧新闻 - published_at: {}, fetched_at: {}",
                r.0.as_deref().unwrap_or("NULL"),
                r.1.unwrap_or(0)
            );
            Ok(r.1)
        }
        None => Ok(None)
    }
}

pub async fn mark_news_read(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    sqlx::query("UPDATE news_cache SET is_read = 1 WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn clear_old_news(pool: &SqlitePool, before: i64) -> Result<u64, AppError> {
    sqlx::query("DELETE FROM news_cache WHERE fetched_at < ?")
        .bind(before)
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
        .map_err(AppError::Database)
}

pub async fn trim_excess_news(pool: &SqlitePool, keep: i64) -> Result<u64, AppError> {
    sqlx::query(
        "DELETE FROM news_cache WHERE id NOT IN (
            SELECT id FROM news_cache ORDER BY fetched_at DESC LIMIT ?
        )",
    )
    .bind(keep)
    .execute(pool)
    .await
    .map(|r| r.rows_affected())
    .map_err(AppError::Database)
}

pub async fn toggle_favorite(pool: &SqlitePool, user_id: i64, id: i64, is_favorite: bool) -> Result<(), AppError> {
    sqlx::query("UPDATE news_cache SET is_favorite = ? WHERE id = ? AND user_id = ?")
        .bind(is_favorite)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn update_ai_summary(pool: &SqlitePool, user_id: i64, id: i64, ai_summary: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE news_cache SET ai_summary = ? WHERE id = ? AND user_id = ?")
        .bind(ai_summary)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn get_pending_delete_news(pool: &SqlitePool, limit: i64) -> Result<Vec<NewsPendingDelete>, AppError> {
    sqlx::query_as::<_, NewsPendingDelete>(
        "SELECT * FROM news_pending_delete ORDER BY moved_at DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn move_to_pending_delete(
    pool: &SqlitePool,
    ids: Vec<i64>,
) -> Result<u64, AppError> {
    let now = chrono::Utc::now().timestamp_millis();

    for id in &ids {
        let news = sqlx::query_as::<_, NewsCache>(
            "SELECT * FROM news_cache WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        if let Some(n) = news {
            sqlx::query(
                "INSERT INTO news_pending_delete (title, url, source, summary, content, category, published_at, original_fetched_at, moved_at, is_read, is_favorite)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&n.title)
            .bind(&n.url)
            .bind(&n.source)
            .bind(&n.summary)
            .bind(&n.content)
            .bind(&n.category)
            .bind(&n.published_at)
            .bind(n.fetched_at)
            .bind(now)
            .bind(n.is_read)
            .bind(n.is_favorite)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        }
    }

    let ids_str = ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    let result = sqlx::query(&format!("DELETE FROM news_cache WHERE id IN ({ids_str})"))
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
        .map_err(AppError::Database)?;

    Ok(result)
}

pub async fn clear_pending_delete(pool: &SqlitePool) -> Result<u64, AppError> {
    sqlx::query("DELETE FROM news_pending_delete")
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
        .map_err(AppError::Database)
}

pub async fn get_non_favorite_ids_to_remove(pool: &SqlitePool, keep_limit: i64) -> Result<Vec<i64>, AppError> {
    let rows = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM news_cache WHERE is_favorite = 0 ORDER BY fetched_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let all_ids: Vec<i64> = rows.into_iter().map(|r| r.0).collect();

    if all_ids.len() <= keep_limit as usize {
        Ok(Vec::new())
    } else {
        Ok(all_ids[keep_limit as usize..].to_vec())
    }
}

pub async fn get_all_non_favorite_ids(pool: &SqlitePool) -> Result<Vec<i64>, AppError> {
    let rows = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM news_cache WHERE is_favorite = 0",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(rows.into_iter().map(|r| r.0).collect())
}
