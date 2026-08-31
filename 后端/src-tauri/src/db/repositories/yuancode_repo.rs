use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::yuancode::CodeSnippet;

pub async fn create_snippet(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    language: &str,
    code: &str,
    description: Option<&str>,
    tags: Option<&str>,
    now: i64,
) -> Result<CodeSnippet, AppError> {
    sqlx::query_as::<_, CodeSnippet>(
        "INSERT INTO yuan_code_snippets (user_id, name, language, code, description, tags, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING *"
    )
    .bind(user_id)
    .bind(name)
    .bind(language)
    .bind(code)
    .bind(description)
    .bind(tags)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_snippets(
    pool: &SqlitePool,
    user_id: i64,
    language: Option<&str>,
) -> Result<Vec<CodeSnippet>, AppError> {
    if let Some(lang) = language {
        sqlx::query_as::<_, CodeSnippet>(
            "SELECT * FROM yuan_code_snippets WHERE user_id = ? AND language = ? ORDER BY updated_at DESC"
        )
        .bind(user_id)
        .bind(lang)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    } else {
        sqlx::query_as::<_, CodeSnippet>(
            "SELECT * FROM yuan_code_snippets WHERE user_id = ? ORDER BY updated_at DESC"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    }
}

pub async fn get_snippet_by_id(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
) -> Result<Option<CodeSnippet>, AppError> {
    sqlx::query_as::<_, CodeSnippet>(
        "SELECT * FROM yuan_code_snippets WHERE id = ? AND user_id = ?"
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn update_snippet(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    name: Option<&str>,
    language: Option<&str>,
    code: Option<&str>,
    description: Option<&str>,
    tags: Option<&str>,
    now: i64,
) -> Result<Option<CodeSnippet>, AppError> {
    let existing = get_snippet_by_id(pool, user_id, id).await?;
    let existing = match existing {
        Some(s) => s,
        None => return Ok(None),
    };

    let new_name = name.unwrap_or(&existing.name);
    let new_language = language.unwrap_or(&existing.language);
    let new_code = code.unwrap_or(&existing.code);
    let new_description = description.or(existing.description.as_deref());
    let new_tags = tags.or(existing.tags.as_deref());

    sqlx::query_as::<_, CodeSnippet>(
        "UPDATE yuan_code_snippets SET name = ?, language = ?, code = ?, description = ?, tags = ?, updated_at = ? WHERE id = ? AND user_id = ?
         RETURNING *"
    )
    .bind(new_name)
    .bind(new_language)
    .bind(new_code)
    .bind(new_description)
    .bind(new_tags)
    .bind(now)
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_snippet(pool: &SqlitePool, user_id: i64, id: i64) -> Result<bool, AppError> {
    let result = sqlx::query("DELETE FROM yuan_code_snippets WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(result.rows_affected() > 0)
}

pub async fn search_snippets(
    pool: &SqlitePool,
    user_id: i64,
    query: &str,
) -> Result<Vec<CodeSnippet>, AppError> {
    let like = format!("%{}%", query);
    sqlx::query_as::<_, CodeSnippet>(
        "SELECT id, user_id, name, language, code, description, tags, created_at, updated_at
        FROM yuan_code_snippets
        WHERE user_id = ? AND (name LIKE ? OR description LIKE ? OR code LIKE ?)
        ORDER BY updated_at DESC
        LIMIT 50",
    )
    .bind(user_id)
    .bind(&like)
    .bind(&like)
    .bind(&like)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn save_workspace(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    workspace_path: &str,
    tabs_json: &str,
    active_tab_index: usize,
) -> Result<i64, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM yuan_code_workspaces WHERE name = ? AND workspace_path = ? AND user_id = ?",
    )
    .bind(name)
    .bind(workspace_path)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    if let Some(id) = existing {
        sqlx::query(
            "UPDATE yuan_code_workspaces SET tabs_json = ?, active_tab_index = ?, updated_at = ? WHERE id = ? AND user_id = ?",
        )
        .bind(tabs_json)
        .bind(active_tab_index as i64)
        .bind(now)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(id)
    } else {
        let id = sqlx::query(
            "INSERT INTO yuan_code_workspaces (user_id, name, workspace_path, tabs_json, active_tab_index, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(user_id)
        .bind(name)
        .bind(workspace_path)
        .bind(tabs_json)
        .bind(active_tab_index as i64)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .map_err(AppError::Database)?
        .last_insert_rowid();
        Ok(id)
    }
}

pub async fn load_workspace(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
) -> Result<Option<(i64, String, String, String, usize, i64, i64)>, AppError> {
    sqlx::query_as::<_, (i64, String, String, String, i64, i64, i64)>(
        "SELECT id, name, workspace_path, tabs_json, active_tab_index, created_at, updated_at
        FROM yuan_code_workspaces WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
    .map(|r| r.map(|(id, n, wp, tj, ati, ca, ua)| (id, n, wp, tj, ati as usize, ca, ua)))
}

pub async fn list_workspaces(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<(i64, String, String, i64, i64)>, AppError> {
    sqlx::query_as::<_, (i64, String, String, i64, i64)>(
        "SELECT id, name, workspace_path, created_at, updated_at FROM yuan_code_workspaces WHERE user_id = ? ORDER BY updated_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_workspace(pool: &SqlitePool, user_id: i64, id: i64) -> Result<bool, AppError> {
    let result = sqlx::query("DELETE FROM yuan_code_workspaces WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(result.rows_affected() > 0)
}
