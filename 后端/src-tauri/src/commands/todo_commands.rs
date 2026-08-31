use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::crud::QueryFilters;
use crate::models::todo::{CreateTodoRequest, Todo, TodoListResponse, UpdateTodoRequest};
use crate::services::intelligence_v4_service;
use crate::services::todo_service;
use crate::commands::common::require_auth;

#[tauri::command]
pub async fn get_todos(
    state: State<'_, AppState>,
    date: String,
) -> Result<ApiResponse<Vec<Todo>>, String> {
    let current_user = require_auth(&state).await?;
    match todo_service::get_todos(&state.pool, &date, Some(current_user)).await {
        Ok(todos) => Ok(ApiResponse::success(todos)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn get_todos_paginated(
    state: State<'_, AppState>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<ApiResponse<TodoListResponse>, String> {
    let current_user = require_auth(&state).await?;
    let filters = QueryFilters {
        page: page.unwrap_or(1),
        page_size: page_size.unwrap_or(20),
        ..Default::default()
    };
    match todo_service::get_todos_paginated(&state.pool, Some(current_user), filters).await {
        Ok(result) => {
            Ok(ApiResponse::success(TodoListResponse {
                todos: result.data,
                total: result.total,
                page: result.page,
                page_size: result.page_size,
            }))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn add_todo(
    state: State<'_, AppState>,
    request: CreateTodoRequest,
) -> Result<ApiResponse<Todo>, String> {
    let current_user = require_auth(&state).await?;
    let priority = request.priority.unwrap_or_else(|| "medium".to_string());
    match todo_service::add_todo(
        &state.pool,
        Some(current_user),
        &request.title,
        request.description.as_deref(),
        &priority,
        request.due_date.as_deref(),
        &request.date,
    )
    .await
    {
        Ok(todo) => {
            intelligence_v4_service::instrument_cmd(&state, "todo", &format!("创建待办/{}", todo.title), None).await;
            Ok(ApiResponse::success(todo))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn toggle_todo(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    // IDOR 修复：使用 user_id 进行资源所有权验证
    let user_id = require_auth(&state).await?;
    match todo_service::toggle_todo(&state.pool, id, &user_id.to_string()).await {
        Ok(_) => {
            intelligence_v4_service::instrument_cmd(&state, "todo", "切换待办状态", None).await;
            Ok(ApiResponse::success(()))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn update_todo(
    state: State<'_, AppState>,
    request: UpdateTodoRequest,
) -> Result<ApiResponse<()>, String> {
    // IDOR 修复：使用 user_id 进行资源所有权验证
    let user_id = require_auth(&state).await?;
    match todo_service::update_todo(
        &state.pool,
        request.id,
        &request.title,
        request.description.as_deref(),
        &request.priority,
        request.due_date.as_deref(),
        request.version,
        &user_id.to_string(),
    )
    .await
    {
        Ok(_) => {
            intelligence_v4_service::instrument_cmd(&state, "todo", "修改待办", None).await;
            Ok(ApiResponse::success(()))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn delete_todo(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    let current_user = require_auth(&state).await?;
    match todo_service::delete_todo(&state.pool, id, Some(current_user)).await {
        Ok(_) => {
            intelligence_v4_service::instrument_cmd(&state, "todo", "删除待办", None).await;
            Ok(ApiResponse::success(()))
        }
        Err(e) => Err(e.into()),
    }
}