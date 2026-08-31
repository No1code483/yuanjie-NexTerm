use tauri::State;

use crate::db::connection::AppState;
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;
use crate::models::permission::Permission;
use crate::models::user::{LoginRequest, LoginResponse, RegisterResponse, ResetPasswordRequest, TempAccountResponse, UserInfoResponse};
use crate::services::auth_service;
use sha2::{Sha256, Digest};
use rand::Rng;

#[tauri::command]
pub async fn register(
    state: State<'_, AppState>,
    username: String,
    password: String,
    is_permanent: bool,
) -> Result<ApiResponse<RegisterResponse>, String> {
    let role = if is_permanent { "admin" } else { "guest" };
    match auth_service::register(
        &state.pool,
        &state.mek_manager,
        &username,
        &password,
        is_permanent,
        None,
        role,
    )
    .await
    {
        Ok(resp) => Ok(ApiResponse::success(resp)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    request: LoginRequest,
) -> Result<ApiResponse<LoginResponse>, String> {
    match auth_service::login(&state.pool, &state.mek_manager, &request.username, &request.password).await {
        Ok(resp) => {
            let mut user = state.current_user.write().await;
            *user = Some(resp.user.id);
            let mut token = state.current_token.write().await;
            *token = Some(resp.token.clone());
            Ok(ApiResponse::success(resp))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn create_temp_account(
    state: State<'_, AppState>,
    duration_hours: i64,
    username: Option<String>,
) -> Result<ApiResponse<TempAccountResponse>, String> {
    match auth_service::create_temp_account(&state.pool, &state.mek_manager, duration_hours, username.as_deref()).await {
        Ok(resp) => Ok(ApiResponse::success(resp)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn recover_by_phrase(
    state: State<'_, AppState>,
    username: String,
    recovery_phrase: String,
    new_password: String,
) -> Result<ApiResponse<LoginResponse>, String> {
    match auth_service::recover_by_phrase(
        &state.pool,
        &state.mek_manager,
        &username,
        &recovery_phrase,
        &new_password,
    )
    .await
    {
        Ok(resp) => {
            let mut user = state.current_user.write().await;
            *user = Some(resp.user.id);
            let mut token = state.current_token.write().await;
            *token = Some(resp.token.clone());
            Ok(ApiResponse::success(resp))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<ApiResponse<()>, String> {
    let user_id = {
        let user = state.current_user.read().await;
        user.ok_or_else(|| AppError::Auth("未登录".into()))?
    };
    match auth_service::logout(&state.pool, &state.mek_manager, user_id).await {
        Ok(_) => {
            let mut user = state.current_user.write().await;
            *user = None;
            let mut token = state.current_token.write().await;
            *token = None;
            Ok(ApiResponse::success(()))
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn auth_verify_token(
    state: State<'_, AppState>,
) -> Result<ApiResponse<UserInfoResponse>, String> {
    let user_id = {
        let user = state.current_user.read().await;
        user.ok_or_else(|| AppError::Auth("未登录".into()))?
    };
    let token = {
        let t = state.current_token.read().await;
        t.clone().ok_or_else(|| AppError::Auth("未登录".into()))?
    };
    match auth_service::verify_token(&state.pool, user_id, &token).await {
        Ok(user_info) => Ok(ApiResponse::success(user_info)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn auth_get_permissions(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<Permission>>, String> {
    let user_id = {
        let user = state.current_user.read().await;
        user.ok_or_else(|| AppError::Auth("未登录".into()))?
    };
    let user = crate::db::repositories::user_repo::find_by_id(&state.pool, user_id)
        .await
        .map_err(|e| String::from(e))?
        .ok_or_else(|| String::from(AppError::Auth("用户不存在".into())))?;
    match auth_service::get_permissions(&state.pool, &user.role).await {
        Ok(perms) => Ok(ApiResponse::success(perms)),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn auth_reset_password(
    state: State<'_, AppState>,
    request: ResetPasswordRequest,
) -> Result<ApiResponse<()>, String> {
    let user_id = {
        let user = state.current_user.read().await;
        user.ok_or_else(|| AppError::Auth("未登录".into()))?
    };
    match auth_service::reset_password(
        &state.pool,
        &state.mek_manager,
        user_id,
        &request.old_password,
        &request.new_password,
    )
    .await
    {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn auth_restore_session(
    state: State<'_, AppState>,
    token: String,
) -> Result<ApiResponse<LoginResponse>, String> {
    match auth_service::restore_session(&state.pool, &token).await {
        Ok(resp) => {
            let mut user = state.current_user.write().await;
            *user = Some(resp.user.id);
            let mut token_state = state.current_token.write().await;
            *token_state = Some(resp.token.clone());
            Ok(ApiResponse::success(resp))
        }
        Err(e) => Err(e.into()),
    }
}

// ========== Task 14.1: Session Management ==========

#[tauri::command]
pub async fn session_list(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let sessions = auth_service::list_sessions(&state.pool, user_id)
        .await
        .map_err(|e| String::from(e))?;
    Ok(ApiResponse::success(sessions))
}

#[tauri::command]
pub async fn session_revoke(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<ApiResponse<()>, String> {
    // IDOR 修复：使用 user_id 确保只能撤销自己的会话
    let user_id = crate::commands::common::require_auth(&state).await?;
    auth_service::revoke_session(&state.pool, &session_id, user_id)
        .await
        .map_err(|e| String::from(e))?;
    Ok(ApiResponse::success(()))
}

// ========== Task 14.3: TOTP 2FA ==========

// Simple TOTP implementation using SHA-256 (since we have sha2)
// Standard TOTP: HMAC-SHA1, 6-digit, 30-second window
// We use HMAC-SHA256 instead for simplicity

fn generate_totp_secret() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..20).map(|_| rng.gen::<u8>()).collect();
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes)
}

fn compute_totp(secret: &str, time_step: u64) -> String {
    let secret_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, secret)
        .unwrap_or_default();

    let time_bytes = time_step.to_be_bytes();
    let mut padded = [0u8; 64];
    padded[0..32].copy_from_slice(&[0x36u8; 32]);
    for i in 0..32.min(secret_bytes.len()) {
        padded[i] ^= secret_bytes[i];
    }
    padded[32..40].copy_from_slice(&time_bytes);
    let inner_hash = Sha256::digest(&padded[0..40]);

    let mut outer = [0u8; 64];
    outer[0..32].copy_from_slice(&[0x5cu8; 32]);
    for i in 0..32.min(secret_bytes.len()) {
        outer[i] ^= secret_bytes[i];
    }
    outer[32..64].copy_from_slice(&inner_hash);
    let hmac = Sha256::digest(&outer[0..64]);

    let offset = (hmac[31] & 0x0f) as usize;
    let code = ((hmac[offset] as u32 & 0x7f) << 24)
        | ((hmac[offset + 1] as u32) << 16)
        | ((hmac[offset + 2] as u32) << 8)
        | (hmac[offset + 3] as u32);
    let code = code % 1_000_000;
    format!("{:06}", code)
}

fn verify_totp(secret: &str, code: &str) -> bool {
    let current_step = chrono::Utc::now().timestamp() as u64 / 30;
    // Check current and adjacent windows
    for step in current_step.saturating_sub(1)..=current_step + 1 {
        if compute_totp(secret, step) == code {
            return true;
        }
    }
    false
}

#[tauri::command]
pub async fn auth_2fa_setup(
    state: State<'_, AppState>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let secret = generate_totp_secret();
    let user = crate::db::repositories::user_repo::find_by_id(&state.pool, user_id)
        .await
        .map_err(|e| String::from(e))?
        .ok_or_else(|| String::from("用户不存在"))?;

    let qr_code_url = format!(
        "otpauth://totp/NexTerm:{}?secret={}&issuer=NexTerm",
        user.username, secret
    );

    // Store secret temporarily
    sqlx::query("INSERT OR REPLACE INTO system_config (key, value) VALUES (?, ?)")
        .bind(format!("2fa_secret_{}", user_id))
        .bind(&secret)
        .execute(&state.pool)
        .await
        .map_err(|e| format!("保存密钥失败: {}", e))?;

    Ok(ApiResponse::success(serde_json::json!({
        "secret": secret,
        "qr_code_url": qr_code_url,
    })))
}

#[tauri::command]
pub async fn auth_2fa_verify(
    state: State<'_, AppState>,
    code: String,
) -> Result<ApiResponse<bool>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let secret_row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM system_config WHERE key = ?"
    )
    .bind(format!("2fa_secret_{}", user_id))
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| format!("查询失败: {}", e))?;

    let secret = match secret_row {
        Some((s,)) => s,
        None => return Ok(ApiResponse::error(4001, "2FA 未设置")),
    };

    let valid = verify_totp(&secret, &code);
    Ok(ApiResponse::success(valid))
}

#[tauri::command]
pub async fn auth_2fa_enable(
    state: State<'_, AppState>,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    // Mark 2FA as enabled
    sqlx::query("INSERT OR REPLACE INTO system_config (key, value) VALUES (?, ?)")
        .bind(format!("2fa_enabled_{}", user_id))
        .bind("1")
        .execute(&state.pool)
        .await
        .map_err(|e| format!("启用2FA失败: {}", e))?;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn auth_2fa_disable(
    state: State<'_, AppState>,
    code: String,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let secret_row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM system_config WHERE key = ?"
    )
    .bind(format!("2fa_secret_{}", user_id))
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| format!("查询失败: {}", e))?;

    let secret = match secret_row {
        Some((s,)) => s,
        None => return Ok(ApiResponse::error(4001, "2FA 未设置")),
    };

    if !verify_totp(&secret, &code) {
        return Ok(ApiResponse::error(4002, "验证码错误"));
    }

    sqlx::query("DELETE FROM system_config WHERE key = ?")
        .bind(format!("2fa_enabled_{}", user_id))
        .execute(&state.pool)
        .await
        .map_err(|e| format!("禁用2FA失败: {}", e))?;

    sqlx::query("DELETE FROM system_config WHERE key = ?")
        .bind(format!("2fa_secret_{}", user_id))
        .execute(&state.pool)
        .await
        .map_err(|e| format!("清理密钥失败: {}", e))?;

    Ok(ApiResponse::success(()))
}

// Check if 2FA is enabled for a user
#[tauri::command]
pub async fn auth_2fa_status(
    state: State<'_, AppState>,
) -> Result<ApiResponse<bool>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM system_config WHERE key = ?"
    )
    .bind(format!("2fa_enabled_{}", user_id))
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| format!("查询失败: {}", e))?;
    Ok(ApiResponse::success(row.is_some()))
}

// Login with 2FA check
#[tauri::command]
pub async fn login_2fa(
    state: State<'_, AppState>,
    request: LoginRequest,
) -> Result<ApiResponse<serde_json::Value>, String> {
    // First, verify password
    match auth_service::login(&state.pool, &state.mek_manager, &request.username, &request.password).await {
        Ok(resp) => {
            let user_id = resp.user.id;
            // Check if 2FA is enabled
            let row: Option<(String,)> = sqlx::query_as(
                "SELECT value FROM system_config WHERE key = ?"
            )
            .bind(format!("2fa_enabled_{}", user_id))
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| format!("查询失败: {}", e))?;

            if row.is_some() {
                // 2FA is enabled, return requires_2fa flag
                Ok(ApiResponse::success(serde_json::json!({
                    "requires_2fa": true,
                    "user_id": user_id,
                    "token": resp.token,
                })))
            } else {
                // No 2FA, complete login
                let mut user = state.current_user.write().await;
                *user = Some(resp.user.id);
                let mut token = state.current_token.write().await;
                *token = Some(resp.token.clone());
                Ok(ApiResponse::success(serde_json::json!({
                    "requires_2fa": false,
                    "user": resp.user,
                    "token": resp.token,
                })))
            }
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn auth_2fa_login_verify(
    state: State<'_, AppState>,
    user_id: i64,
    code: String,
    token: String,
) -> Result<ApiResponse<LoginResponse>, String> {
    let secret_row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM system_config WHERE key = ?"
    )
    .bind(format!("2fa_secret_{}", user_id))
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| format!("查询失败: {}", e))?;

    let secret = match secret_row {
        Some((s,)) => s,
        None => return Ok(ApiResponse::error(4001, "2FA 未设置")),
    };

    if !verify_totp(&secret, &code) {
        return Ok(ApiResponse::error(4002, "验证码错误"));
    }

    let user_info = auth_service::verify_token(&state.pool, user_id, &token)
        .await
        .map_err(|e| String::from(e))?;

    let mut user = state.current_user.write().await;
    *user = Some(user_info.id);
    let mut token_state = state.current_token.write().await;
    *token_state = Some(token.clone());

    Ok(ApiResponse::success(LoginResponse {
        token,
        user: user_info,
    }))
}