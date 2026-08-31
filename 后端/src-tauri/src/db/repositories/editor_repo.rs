use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::editor::{EditorDocument, EditorSession, EditorVersion};

#[derive(Debug, sqlx::FromRow)]
pub struct FtsSearchRow {
    pub doc_uuid: String,
    pub title: String,
    pub content_type: String,
    pub content: String,
    pub rank: f64,
}

pub async fn search_fts(pool: &SqlitePool, query: &str, limit: i64) -> Result<Vec<FtsSearchRow>, AppError> {
    sqlx::query_as::<_, FtsSearchRow>(
        "SELECT doc_uuid, title, content_type, snippet(editor_fts, 2, '<mark>', '</mark>', '...', 40) AS content, rank
         FROM editor_fts
         WHERE editor_fts MATCH ?
         ORDER BY rank
         LIMIT ?"
    )
    .bind(query)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn count_fts_matches(pool: &SqlitePool, query: &str) -> Result<i64, AppError> {
    let result: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM editor_fts WHERE editor_fts MATCH ?"
    )
    .bind(query)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(result.0)
}

pub async fn upsert_fts_index(
    pool: &SqlitePool,
    doc_uuid: &str,
    title: &str,
    content_type: &str,
    content: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO editor_fts(doc_uuid, title, content_type, content)
         VALUES (?, ?, ?, ?)"
    )
    .bind(doc_uuid)
    .bind(title)
    .bind(content_type)
    .bind(content)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn delete_fts_index(pool: &SqlitePool, doc_uuid: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM editor_fts WHERE doc_uuid = ?")
        .bind(doc_uuid)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn create_document(
    pool: &SqlitePool,
    doc_uuid: &str,
    title: &str,
    source_type: &str,
    source_id: Option<i64>,
    content_type: &str,
    file_size: i64,
    now: i64,
) -> Result<EditorDocument, AppError> {
    sqlx::query_as::<_, EditorDocument>(
        "INSERT INTO editor_documents (doc_uuid, title, source_type, source_id, content_type, file_size, version, version_count, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, 1, 0, ?, ?)
         RETURNING *",
    )
    .bind(doc_uuid)
    .bind(title)
    .bind(source_type)
    .bind(source_id)
    .bind(content_type)
    .bind(file_size)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_document(pool: &SqlitePool, doc_uuid: &str) -> Result<Option<EditorDocument>, AppError> {
    sqlx::query_as::<_, EditorDocument>("SELECT * FROM editor_documents WHERE doc_uuid = ?")
        .bind(doc_uuid)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn get_document_by_source(
    pool: &SqlitePool,
    source_type: &str,
    source_id: i64,
) -> Result<Option<EditorDocument>, AppError> {
    sqlx::query_as::<_, EditorDocument>(
        "SELECT * FROM editor_documents WHERE source_type = ? AND source_id = ?",
    )
    .bind(source_type)
    .bind(source_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn update_document_size(
    pool: &SqlitePool,
    doc_uuid: &str,
    file_size: i64,
    now: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE editor_documents SET file_size = ?, updated_at = ? WHERE doc_uuid = ?",
    )
    .bind(file_size)
    .bind(now)
    .bind(doc_uuid)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn update_document_with_version(
    pool: &SqlitePool,
    doc_uuid: &str,
    file_size: i64,
    expected_version: i32,
    now: i64,
) -> Result<u64, AppError> {
    let result = sqlx::query(
        "UPDATE editor_documents SET file_size = ?, version = version + 1, updated_at = ? WHERE doc_uuid = ? AND version = ?"
    )
    .bind(file_size)
    .bind(now)
    .bind(doc_uuid)
    .bind(expected_version)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(result.rows_affected())
}

pub async fn increment_version_count(pool: &SqlitePool, doc_uuid: &str, now: i64) -> Result<i64, AppError> {
    sqlx::query(
        "UPDATE editor_documents SET version_count = version_count + 1, updated_at = ? WHERE doc_uuid = ?",
    )
    .bind(now)
    .bind(doc_uuid)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    let doc = get_document(pool, doc_uuid)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(doc.version_count)
}

pub async fn delete_document(pool: &SqlitePool, doc_uuid: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM editor_documents WHERE doc_uuid = ?")
        .bind(doc_uuid)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn create_version(
    pool: &SqlitePool,
    doc_uuid: &str,
    version_num: i64,
    file_path: &str,
    file_size: i64,
    change_summary: Option<&str>,
    now: i64,
) -> Result<EditorVersion, AppError> {
    sqlx::query_as::<_, EditorVersion>(
        "INSERT INTO editor_versions (doc_uuid, version_num, file_path, file_size, change_summary, created_at)
         VALUES (?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(doc_uuid)
    .bind(version_num)
    .bind(file_path)
    .bind(file_size)
    .bind(change_summary)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_versions(pool: &SqlitePool, doc_uuid: &str) -> Result<Vec<EditorVersion>, AppError> {
    sqlx::query_as::<_, EditorVersion>(
        "SELECT * FROM editor_versions WHERE doc_uuid = ? ORDER BY version_num DESC",
    )
    .bind(doc_uuid)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_version(
    pool: &SqlitePool,
    doc_uuid: &str,
    version_num: i64,
) -> Result<Option<EditorVersion>, AppError> {
    sqlx::query_as::<_, EditorVersion>(
        "SELECT * FROM editor_versions WHERE doc_uuid = ? AND version_num = ?",
    )
    .bind(doc_uuid)
    .bind(version_num)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_max_version_num(pool: &SqlitePool, doc_uuid: &str) -> Result<i64, AppError> {
    let result: Option<(i64,)> = sqlx::query_as(
        "SELECT COALESCE(MAX(version_num), 0) FROM editor_versions WHERE doc_uuid = ?",
    )
    .bind(doc_uuid)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(result.map(|(n,)| n).unwrap_or(0))
}

pub async fn delete_versions_by_doc(pool: &SqlitePool, doc_uuid: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM editor_versions WHERE doc_uuid = ?")
        .bind(doc_uuid)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn upsert_session(
    pool: &SqlitePool,
    doc_uuid: &str,
    is_dirty: i64,
    cursor_line: Option<i64>,
    cursor_column: Option<i64>,
    now: i64,
) -> Result<EditorSession, AppError> {
    sqlx::query_as::<_, EditorSession>(
        "INSERT INTO editor_sessions (doc_uuid, is_dirty, cursor_line, cursor_column, last_activity, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(doc_uuid) DO UPDATE SET
            is_dirty = excluded.is_dirty,
            cursor_line = excluded.cursor_line,
            cursor_column = excluded.cursor_column,
            last_activity = excluded.last_activity,
            updated_at = excluded.updated_at
         RETURNING *",
    )
    .bind(doc_uuid)
    .bind(is_dirty)
    .bind(cursor_line)
    .bind(cursor_column)
    .bind(now)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_session(pool: &SqlitePool, doc_uuid: &str) -> Result<Option<EditorSession>, AppError> {
    sqlx::query_as::<_, EditorSession>("SELECT * FROM editor_sessions WHERE doc_uuid = ?")
        .bind(doc_uuid)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn get_all_dirty_sessions(pool: &SqlitePool) -> Result<Vec<EditorSession>, AppError> {
    sqlx::query_as::<_, EditorSession>(
        "SELECT * FROM editor_sessions WHERE is_dirty = 1 ORDER BY last_activity DESC"
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_session(pool: &SqlitePool, doc_uuid: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM editor_sessions WHERE doc_uuid = ?")
        .bind(doc_uuid)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn cleanup_expired_sessions(pool: &SqlitePool, threshold_ms: i64) -> Result<u64, AppError> {
    let result = sqlx::query(
        "DELETE FROM editor_sessions WHERE last_activity < ? AND is_dirty = 0"
    )
    .bind(threshold_ms)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(result.rows_affected())
}

pub async fn list_documents(pool: &SqlitePool) -> Result<Vec<EditorDocument>, AppError> {
    sqlx::query_as::<_, EditorDocument>(
        "SELECT * FROM editor_documents ORDER BY updated_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}