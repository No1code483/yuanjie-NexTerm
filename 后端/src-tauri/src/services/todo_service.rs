use sqlx::SqlitePool;

use crate::db::repositories::{recycle_repo, todo_repo};
use crate::error::app_error::AppError;
use crate::models::crud::{PaginatedResult, QueryFilters};
use crate::models::todo::Todo;
use crate::services::intelligence_v4_service;
use crate::services::storage_service::{check_version_conflict, now_ms};

pub async fn get_todos(pool: &SqlitePool, date: &str, user_id: Option<i64>) -> Result<Vec<Todo>, AppError> {
    todo_repo::get_todos_by_date(pool, date, user_id).await
}

pub async fn get_todos_paginated(
    pool: &SqlitePool,
    user_id: Option<i64>,
    filters: QueryFilters,
) -> Result<PaginatedResult<Todo>, AppError> {
    todo_repo::get_todos_paginated(pool, user_id, &filters).await
}

pub async fn add_todo(
    pool: &SqlitePool,
    user_id: Option<i64>,
    title: &str,
    description: Option<&str>,
    priority: &str,
    due_date: Option<&str>,
    date: &str,
) -> Result<Todo, AppError> {
    let result = todo_repo::add_todo(pool, user_id, title, description, priority, due_date, date, now_ms()).await?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "todos", "create",
        Some(&format!("title: {}", title)),
    ).await;
    Ok(result)
}

pub async fn toggle_todo(pool: &SqlitePool, id: i64, user_id: &str) -> Result<(), AppError> {
    todo_repo::toggle_todo(pool, id, now_ms(), user_id).await?;
    let _ = intelligence_v4_service::instrument(
        pool, "system", "todos", "toggle",
        Some(&format!("id: {}", id)),
    ).await;
    Ok(())
}

pub async fn update_todo(
    pool: &SqlitePool,
    id: i64,
    title: &str,
    description: Option<&str>,
    priority: &str,
    due_date: Option<&str>,
    version: i32,
    user_id: &str,
) -> Result<(), AppError> {
    let todo = todo_repo::find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound)?;

    check_version_conflict(version, todo.version)?;

    let rows = todo_repo::update_todo(pool, id, title, description, priority, due_date, version, now_ms(), user_id).await?;
    if rows == 0 {
        return Err(AppError::Conflict("版本冲突: 数据已被其他操作修改".into()));
    }
    Ok(())
}

pub async fn delete_todo(pool: &SqlitePool, id: i64, deleted_by: Option<i64>) -> Result<(), AppError> {
    let todo = todo_repo::get_todo_for_delete(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound)?;

    let now = now_ms();
    let auto_delete_at = now + 30 * 24 * 3600 * 1000;
    let metadata = serde_json::json!({
        "title": todo.title,
        "description": todo.description,
        "priority": todo.priority,
        "due_date": todo.due_date,
        "date": todo.date,
        "completed": todo.completed,
    });
    let metadata_str = serde_json::to_string(&metadata).ok();

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    let uid = deleted_by.unwrap_or(1);
    recycle_repo::add_item(
        &mut *tx,
        uid,
        &format!("todos/{}", id),
        "todo",
        Some(id),
        Some(&todo.title),
        metadata_str.as_deref(),
        None,
        deleted_by,
        now,
        auto_delete_at,
    )
    .await?;

    sqlx::query("DELETE FROM todos WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(uid)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;
    Ok(())
}