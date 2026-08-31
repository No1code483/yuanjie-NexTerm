use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::crud::{PaginatedResult, QueryFilters};
use crate::models::todo::Todo;

pub async fn get_todos_by_date(pool: &SqlitePool, date: &str, user_id: Option<i64>) -> Result<Vec<Todo>, AppError> {
    sqlx::query_as::<_, Todo>(
        "SELECT * FROM todos WHERE (user_id = ? OR user_id IS NULL) AND date = ? ORDER BY 
         CASE priority 
             WHEN 'high' THEN 1 
             WHEN 'medium' THEN 2 
             WHEN 'low' THEN 3 
             ELSE 4 
         END, created_at ASC"
    )
    .bind(user_id)
    .bind(date)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_todos_paginated(
    pool: &SqlitePool,
    user_id: Option<i64>,
    filters: &QueryFilters,
) -> Result<PaginatedResult<Todo>, AppError> {
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM todos WHERE (user_id = ? OR user_id IS NULL)"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    let todos: Vec<Todo> = sqlx::query_as::<_, Todo>(
        "SELECT * FROM todos WHERE (user_id = ? OR user_id IS NULL) ORDER BY created_at DESC LIMIT ? OFFSET ?"
    )
    .bind(user_id)
    .bind(filters.limit())
    .bind(filters.offset())
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let total_count = total.0;
    Ok(PaginatedResult {
        total: total_count,
        page: filters.page,
        page_size: filters.page_size,
        total_pages: ((total_count as f64) / (filters.page_size as f64)).ceil() as u32,
        data: todos,
    })
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Todo>, AppError> {
    sqlx::query_as::<_, Todo>("SELECT * FROM todos WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn add_todo(
    pool: &SqlitePool,
    user_id: Option<i64>,
    title: &str,
    description: Option<&str>,
    priority: &str,
    due_date: Option<&str>,
    date: &str,
    now: i64,
) -> Result<Todo, AppError> {
    sqlx::query_as::<_, Todo>(
        "INSERT INTO todos (user_id, title, description, priority, due_date, date, completed, version, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, 0, 1, ?, ?)
         RETURNING *",
    )
    .bind(user_id)
    .bind(title)
    .bind(description)
    .bind(priority)
    .bind(due_date)
    .bind(date)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn toggle_todo(pool: &SqlitePool, id: i64, now: i64, user_id: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE todos SET completed = NOT completed, updated_at = ? WHERE id = ? AND (user_id = ? OR user_id IS NULL)")
        .bind(now)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn update_todo(
    pool: &SqlitePool,
    id: i64,
    title: &str,
    description: Option<&str>,
    priority: &str,
    due_date: Option<&str>,
    expected_version: i32,
    now: i64,
    user_id: &str,
) -> Result<u64, AppError> {
    let result = sqlx::query(
        "UPDATE todos SET title = ?, description = ?, priority = ?, due_date = ?, version = version + 1, updated_at = ? WHERE id = ? AND version = ? AND (user_id = ? OR user_id IS NULL)"
    )
    .bind(title)
    .bind(description)
    .bind(priority)
    .bind(due_date)
    .bind(now)
    .bind(id)
    .bind(expected_version)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(result.rows_affected())
}

pub async fn get_todo_for_delete(pool: &SqlitePool, id: i64) -> Result<Option<Todo>, AppError> {
    find_by_id(pool, id).await
}