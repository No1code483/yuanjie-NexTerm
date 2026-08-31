use crate::db::connection::AppState;
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;
use crate::services::auth_service;

pub fn map_result<T: serde::Serialize>(result: Result<T, AppError>) -> Result<ApiResponse<T>, String> {
    match result {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

/// 验证当前用户已登录且 Token 有效
/// 返回 user_id
pub async fn verify_token(state: &AppState) -> Result<i64, AppError> {
    let token_lock = state.current_token.read().await;
    let token = token_lock
        .as_ref()
        .ok_or_else(|| AppError::Auth("未登录".into()))?;

    let user_lock = state.current_user.read().await;
    let user_id = user_lock
        .ok_or_else(|| AppError::Auth("未登录".into()))?;

    let user = auth_service::verify_token(&state.pool, user_id, token).await?;
    Ok(user.id)
}

/// Tauri Command 中使用：获取当前登录用户 ID，未登录则返回错误响应
pub async fn require_auth(state: &AppState) -> Result<i64, String> {
    match verify_token(state).await {
        Ok(id) => Ok(id),
        Err(e) => Err(ApiResponse::<()>::error(e.error_code(), &e.to_string()).to_json()),
    }
}

impl<T: serde::Serialize> ApiResponse<T> {
    pub fn to_json(self) -> String {
        serde_json::to_string(&self).unwrap_or_else(|_| r#"{"code":9001,"message":"序列化失败","data":null}"#.to_string())
    }
}