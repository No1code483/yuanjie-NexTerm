use sqlx::SqlitePool;

use crate::db::repositories::editor_repo;
use crate::error::app_error::AppError;
use crate::models::editor::{SearchRequest, SearchResponse, SearchResult};

pub async fn search(
    pool: &SqlitePool,
    req: SearchRequest,
) -> Result<SearchResponse, AppError> {
    let limit = req.limit.unwrap_or(20).min(100);

    let query = sanitize_fts_query(&req.query);
    if query.is_empty() {
        return Ok(SearchResponse {
            results: vec![],
            total: 0,
        });
    }

    let total = editor_repo::count_fts_matches(pool, &query).await?;
    let rows = editor_repo::search_fts(pool, &query, limit).await?;

    let results = rows
        .into_iter()
        .map(|row| SearchResult {
            doc_uuid: row.doc_uuid,
            title: row.title,
            content_type: row.content_type,
            snippet: row.content,
            rank: row.rank,
        })
        .collect();

    Ok(SearchResponse { results, total })
}

pub async fn index_document(
    pool: &SqlitePool,
    doc_uuid: &str,
    title: &str,
    content_type: &str,
    content: &str,
) -> Result<(), AppError> {
    editor_repo::delete_fts_index(pool, doc_uuid).await?;
    editor_repo::upsert_fts_index(pool, doc_uuid, title, content_type, content).await
}

pub async fn remove_from_index(pool: &SqlitePool, doc_uuid: &str) -> Result<(), AppError> {
    editor_repo::delete_fts_index(pool, doc_uuid).await
}

fn sanitize_fts_query(input: &str) -> String {
    let cleaned: String = input
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '*' || *c == '"')
        .collect();

    let terms: Vec<&str> = cleaned.split_whitespace().collect();
    if terms.is_empty() {
        return String::new();
    }

    let with_wildcards: Vec<String> = terms
        .iter()
        .map(|t| {
            if t.ends_with('*') || t.starts_with('"') {
                t.to_string()
            } else {
                format!("{}*", t)
            }
        })
        .collect();

    with_wildcards.join(" ")
}