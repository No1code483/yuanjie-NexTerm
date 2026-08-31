use std::sync::Arc;

use rand::RngCore;
use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::crypto::aes_gcm;
use crate::crypto::key_derivation;
use crate::crypto::mek_manager::MekManager;
use crate::db::repositories::permission_repo;
use crate::db::repositories::user_repo;
use crate::error::app_error::AppError;
use crate::models::permission::Permission;
use crate::models::user::{CreateUserDto, LoginResponse, RegisterResponse, TempAccountResponse, UserInfoResponse};

const RECOVERY_PHRASE_WORD_COUNT: usize = 12;

pub async fn register(
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    username: &str,
    password: &str,
    is_permanent: bool,
    expires_at: Option<i64>,
    role: &str,
) -> Result<RegisterResponse, AppError> {
    if user_repo::find_by_username(pool, username).await?.is_some() {
        return Err(AppError::Auth("用户名已存在".into()));
    }

    let salt = key_derivation::generate_salt();
    let kek = key_derivation::derive_kek(password, &salt)?;
    let password_hash = key_derivation::hash_password(password, &salt)?;

    let mut mek = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut mek);

    let (encrypted_mek, mek_nonce) = aes_gcm::encrypt_bytes(&mek, &kek)?;

    let recovery_phrase = key_derivation::generate_recovery_phrase(RECOVERY_PHRASE_WORD_COUNT);
    let recovery_phrase_hash = Some(key_derivation::hash_recovery_phrase(&recovery_phrase)?);

    let now = chrono::Utc::now().timestamp_millis();

    let dto = CreateUserDto {
        username: username.to_string(),
        password_hash,
        salt,
        encrypted_mek,
        mek_nonce: mek_nonce.to_vec(),
        recovery_phrase_hash,
        is_permanent,
        expires_at,
        role: role.to_string(),
        created_at: now,
        updated_at: now,
    };

    let user = user_repo::create_user(pool, &dto).await?;

    {
        let mut mgr = mek_manager.write().await;
        mgr.cache_mek(user.id, mek);
    }

    Ok(RegisterResponse {
        token: format!("nt_{}", uuid::Uuid::new_v4()),
        user: UserInfoResponse::from(user),
        recovery_phrase,
    })
}

pub async fn login(
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    username: &str,
    password: &str,
) -> Result<LoginResponse, AppError> {
    let mut user = user_repo::find_by_username(pool, username)
        .await?
        .ok_or_else(|| AppError::Auth("用户名或密码错误".into()))?;

    if user.is_permanent && user.role == "guest" {
        let _ = sqlx::query("UPDATE users SET role = 'admin' WHERE id = ? AND role = 'guest'")
            .bind(user.id)
            .execute(pool)
            .await;
        user.role = "admin".to_string();
    }

    if !user.is_permanent {
        if let Some(expires_at) = user.expires_at {
            let now = chrono::Utc::now().timestamp_millis();
            if now > expires_at {
                return Err(AppError::TempAccountExpired);
            }
        }
    }

    let is_valid = key_derivation::verify_password(password, &user.password_hash)?;
    if !is_valid {
        return Err(AppError::Auth("用户名或密码错误".into()));
    }

    let kek = key_derivation::derive_kek(password, &user.salt)?;

    let mek_nonce: [u8; 12] = user
        .mek_nonce
        .as_slice()
        .try_into()
        .map_err(|_| AppError::MekDecryption("MEK nonce 长度错误".into()))?;

    let mek_bytes = aes_gcm::decrypt_bytes(&user.encrypted_mek, &kek, &mek_nonce)?;

    let mut mek = [0u8; 32];
    mek.copy_from_slice(&mek_bytes);

    {
        let mut mgr = mek_manager.write().await;
        mgr.cache_mek(user.id, mek);
    }

    let token = format!("nt_{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().timestamp_millis();
    let expires_at = if user.is_permanent {
        None
    } else {
        user.expires_at
    };

    sqlx::query(
        "INSERT INTO auth_sessions (user_id, token, is_active, created_at, expires_at) VALUES (?, ?, 1, ?, ?)",
    )
    .bind(user.id)
    .bind(&token)
    .bind(now)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(LoginResponse {
        token,
        user: UserInfoResponse::from(user),
    })
}

pub async fn create_temp_account(
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    duration_hours: i64,
    preferred_username: Option<&str>,
) -> Result<TempAccountResponse, AppError> {
    let username = if let Some(name) = preferred_username {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.len() > 50 {
            format!("guest_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("0000"))
        } else {
            trimmed.to_string()
        }
    } else {
        format!("guest_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("0000"))
    };
    
    let password = generate_temp_password();

    let now = chrono::Utc::now().timestamp_millis();
    let expires_at = now + duration_hours * 3600 * 1000;

    let _ = register(
        pool,
        mek_manager,
        &username,
        &password,
        false,
        Some(expires_at),
        "guest",
    )
    .await?;

    Ok(TempAccountResponse {
        username,
        password,
        expires_at,
    })
}

pub async fn recover_by_phrase(
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    username: &str,
    recovery_phrase: &str,
    new_password: &str,
) -> Result<LoginResponse, AppError> {
    let user = user_repo::find_by_username(pool, username)
        .await?
        .ok_or_else(|| AppError::Auth("用户不存在".into()))?;

    let stored_hash = user
        .recovery_phrase_hash
        .as_ref()
        .ok_or_else(|| AppError::RecoveryPhraseInvalid)?;

    if !key_derivation::verify_recovery_phrase(recovery_phrase, stored_hash)? {
        return Err(AppError::RecoveryPhraseInvalid);
    }

    let new_salt = key_derivation::generate_salt();
    let new_kek = key_derivation::derive_kek(new_password, &new_salt)?;
    let new_password_hash = key_derivation::hash_password(new_password, &new_salt)?;

    let mek_nonce: [u8; 12] = user
        .mek_nonce
        .as_slice()
        .try_into()
        .map_err(|_| AppError::MekDecryption("MEK nonce 长度错误".into()))?;

    let old_kek = key_derivation::derive_kek_from_stored(&user.password_hash)?;
    let mek_bytes = aes_gcm::decrypt_bytes(&user.encrypted_mek, &old_kek, &mek_nonce)?;
    let mek: [u8; 32] = mek_bytes
        .try_into()
        .map_err(|_| AppError::MekDecryption("MEK 长度错误".into()))?;

    let (encrypted_mek, new_nonce) = aes_gcm::encrypt_bytes(&mek, &new_kek)?;
    let now = chrono::Utc::now().timestamp_millis();
    user_repo::update_password_and_mek(
        pool,
        user.id,
        &new_password_hash,
        &new_salt,
        &encrypted_mek,
        &new_nonce,
        now,
    )
    .await?;

    {
        let mut mgr = mek_manager.write().await;
        mgr.cache_mek(user.id, mek);
    }

    let token = format!("nt_{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().timestamp_millis();

    sqlx::query(
        "INSERT INTO auth_sessions (user_id, token, is_active, created_at, expires_at) VALUES (?, ?, 1, ?, NULL)",
    )
    .bind(user.id)
    .bind(&token)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(LoginResponse {
        token,
        user: UserInfoResponse::from(user),
    })
}

pub async fn logout(
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    user_id: i64,
) -> Result<(), AppError> {
    sqlx::query("UPDATE auth_sessions SET is_active = 0 WHERE user_id = ? AND is_active = 1")
        .bind(user_id)
        .execute(pool)
        .await?;

    let mut mgr = mek_manager.write().await;
    mgr.clear_mek(user_id);
    Ok(())
}

pub async fn restore_session(
    pool: &SqlitePool,
    token: &str,
) -> Result<LoginResponse, AppError> {
    let (user_id,): (i64,) = sqlx::query_as(
        "SELECT user_id FROM auth_sessions WHERE token = ? AND is_active = 1",
    )
    .bind(token)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Auth("会话已失效，请重新登录".into()))?;

    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        "UPDATE auth_sessions SET is_active = 0 WHERE token = ? AND is_active = 1 AND expires_at IS NOT NULL AND expires_at < ?",
    )
    .bind(token)
    .bind(now)
    .execute(pool)
    .await?;

    let (session_id,): (i64,) = sqlx::query_as(
        "SELECT id FROM auth_sessions WHERE token = ? AND is_active = 1",
    )
    .bind(token)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Auth("会话已过期，请重新登录".into()))?;

    let _ = session_id;

    let user = user_repo::find_by_id(pool, user_id)
        .await?
        .ok_or_else(|| AppError::Auth("用户不存在".into()))?;

    Ok(LoginResponse {
        token: token.to_string(),
        user: UserInfoResponse::from(user),
    })
}

fn generate_temp_password() -> String {
    use rand::Rng;
    let charset: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789";
    let mut rng = rand::thread_rng();
    (0..12)
        .map(|_| {
            let idx = rng.gen_range(0..charset.len());
            charset[idx] as char
        })
        .collect()
}

pub async fn verify_token(pool: &SqlitePool, user_id: i64, token: &str) -> Result<UserInfoResponse, AppError> {
    let session = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM auth_sessions WHERE user_id = ? AND token = ? AND is_active = 1",
    )
    .bind(user_id)
    .bind(token)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Auth("会话已失效，请重新登录".into()))?;

    let _ = session;

    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        "UPDATE auth_sessions SET is_active = 0 WHERE user_id = ? AND token = ? AND is_active = 1 AND expires_at IS NOT NULL AND expires_at < ?",
    )
    .bind(user_id)
    .bind(token)
    .bind(now)
    .execute(pool)
    .await?;

    let session = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM auth_sessions WHERE user_id = ? AND token = ? AND is_active = 1",
    )
    .bind(user_id)
    .bind(token)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Auth("会话已过期，请重新登录".into()))?;

    let _ = session;

    let user = user_repo::find_by_id(pool, user_id)
        .await?
        .ok_or_else(|| AppError::Auth("用户不存在".into()))?;
    Ok(UserInfoResponse::from(user))
}

pub async fn get_permissions(pool: &SqlitePool, role: &str) -> Result<Vec<Permission>, AppError> {
    permission_repo::get_permissions_by_role(pool, role).await
}

pub async fn reset_password(
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    user_id: i64,
    old_password: &str,
    new_password: &str,
) -> Result<(), AppError> {
    let user = user_repo::find_by_id(pool, user_id)
        .await?
        .ok_or_else(|| AppError::Auth("用户不存在".into()))?;

    let old_password_hash = key_derivation::hash_password(old_password, &user.salt)?;
    if old_password_hash != user.password_hash {
        return Err(AppError::Auth("原密码错误".into()));
    }

    let old_kek = key_derivation::derive_kek(old_password, &user.salt)?;
    let mek_nonce: [u8; 12] = user
        .mek_nonce
        .as_slice()
        .try_into()
        .map_err(|_| AppError::MekDecryption("MEK nonce 长度错误".into()))?;
    let mek_bytes = aes_gcm::decrypt_bytes(&user.encrypted_mek, &old_kek, &mek_nonce)?;
    let mek: [u8; 32] = mek_bytes
        .try_into()
        .map_err(|_| AppError::MekDecryption("MEK 长度错误".into()))?;

    let new_salt = key_derivation::generate_salt();
    let new_kek = key_derivation::derive_kek(new_password, &new_salt)?;
    let new_password_hash = key_derivation::hash_password(new_password, &new_salt)?;
    let (encrypted_mek, new_nonce) = aes_gcm::encrypt_bytes(&mek, &new_kek)?;
    let now = chrono::Utc::now().timestamp_millis();

    user_repo::update_password_and_mek(
        pool,
        user_id,
        &new_password_hash,
        &new_salt,
        &encrypted_mek,
        &new_nonce,
        now,
    )
    .await?;

    {
        let mut mgr = mek_manager.write().await;
        mgr.cache_mek(user_id, mek);
    }

    Ok(())
}

pub async fn change_username(
    pool: &SqlitePool,
    user_id: i64,
    new_username: &str,
) -> Result<(), AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    user_repo::update_username(pool, user_id, new_username, now).await
}

// ========== Session Management ==========

pub async fn list_sessions(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<serde_json::Value>, AppError> {
    let rows: Vec<(i64, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT id, token, created_at, expires_at FROM auth_sessions WHERE user_id = ? AND is_active = 1 ORDER BY created_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let sessions = rows.into_iter().map(|(id, token, created, expires)| {
        let _short_token = if token.len() > 8 {
            format!("{}...", &token[..8])
        } else {
            token.clone()
        };
        serde_json::json!({
            "id": id,
            "session_id": id.to_string(),
            "device": "NexTerm 客户端",
            "ip": "127.0.0.1",
            "created_at": created,
            "last_active": created,
            "expires_at": expires,
        })
    }).collect();

    Ok(sessions)
}

pub async fn revoke_session(
    pool: &SqlitePool,
    session_id: &str,
    user_id: i64,
) -> Result<(), AppError> {
    let id: i64 = session_id.parse().map_err(|_| AppError::Validation("无效的会话ID".into()))?;
    // IDOR 修复：添加 user_id 过滤，防止用户撤销他人会话
    sqlx::query("UPDATE auth_sessions SET is_active = 0 WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}