use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::user::{CreateUserDto, User};

pub async fn find_by_username(pool: &SqlitePool, username: &str) -> Result<Option<User>, AppError> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> Result<Option<User>, AppError> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn create_user(pool: &SqlitePool, dto: &CreateUserDto) -> Result<User, AppError> {
    sqlx::query_as::<_, User>(
        "INSERT INTO users (username, password_hash, salt, encrypted_mek, mek_nonce, recovery_phrase_hash, is_permanent, expires_at, role, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(&dto.username)
    .bind(&dto.password_hash)
    .bind(&dto.salt)
    .bind(&dto.encrypted_mek)
    .bind(&dto.mek_nonce)
    .bind(&dto.recovery_phrase_hash)
    .bind(dto.is_permanent)
    .bind(dto.expires_at)
    .bind(&dto.role)
    .bind(dto.created_at)
    .bind(dto.updated_at)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn update_mek(
    pool: &SqlitePool,
    user_id: i64,
    encrypted_mek: &[u8],
    mek_nonce: &[u8],
    updated_at: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE users SET encrypted_mek = ?, mek_nonce = ?, updated_at = ? WHERE id = ?",
    )
    .bind(encrypted_mek)
    .bind(mek_nonce)
    .bind(updated_at)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn delete_expired_temp_users(pool: &SqlitePool, now: i64) -> Result<u64, AppError> {
    sqlx::query("DELETE FROM users WHERE is_permanent = 0 AND expires_at IS NOT NULL AND expires_at < ?")
        .bind(now)
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
        .map_err(AppError::Database)
}

pub async fn update_password_and_mek(
    pool: &SqlitePool,
    user_id: i64,
    password_hash: &str,
    salt: &[u8],
    encrypted_mek: &[u8],
    mek_nonce: &[u8],
    updated_at: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE users SET password_hash = ?, salt = ?, encrypted_mek = ?, mek_nonce = ?, updated_at = ? WHERE id = ?",
    )
    .bind(password_hash)
    .bind(salt)
    .bind(encrypted_mek)
    .bind(mek_nonce)
    .bind(updated_at)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn update_username(
    pool: &SqlitePool,
    user_id: i64,
    new_username: &str,
    updated_at: i64,
) -> Result<(), AppError> {
    let existing = find_by_username(pool, new_username).await?;
    if existing.is_some() {
        return Err(AppError::Auth("用户名已存在".into()));
    }
    sqlx::query("UPDATE users SET username = ?, updated_at = ? WHERE id = ?")
        .bind(new_username)
        .bind(updated_at)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

// 更新用户扩展资料（头像 / 签名 / 显示名），None 值保持原状
pub async fn update_profile(
    pool: &SqlitePool,
    user_id: i64,
    avatar_url: Option<&str>,
    bio: Option<&str>,
    display_name: Option<&str>,
    updated_at: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE users SET avatar_url = COALESCE(?, avatar_url), bio = COALESCE(?, bio), display_name = COALESCE(?, display_name), updated_at = ? WHERE id = ?",
    )
    .bind(avatar_url)
    .bind(bio)
    .bind(display_name)
    .bind(updated_at)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}