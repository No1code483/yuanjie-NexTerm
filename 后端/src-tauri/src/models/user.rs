use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub salt: Vec<u8>,
    pub encrypted_mek: Vec<u8>,
    pub mek_nonce: Vec<u8>,
    pub recovery_phrase_hash: Option<String>,
    pub is_permanent: bool,
    pub expires_at: Option<i64>,
    pub role: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfoResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub token: String,
    pub user: UserInfoResponse,
    pub recovery_phrase: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TempAccountResponse {
    pub username: String,
    pub password: String,
    pub expires_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserDto {
    pub username: String,
    pub password_hash: String,
    pub salt: Vec<u8>,
    pub encrypted_mek: Vec<u8>,
    pub mek_nonce: Vec<u8>,
    pub recovery_phrase_hash: Option<String>,
    pub is_permanent: bool,
    pub expires_at: Option<i64>,
    pub role: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfoResponse {
    pub id: i64,
    pub username: String,
    pub role: String,
    pub is_permanent: bool,
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub display_name: Option<String>,
}

impl From<User> for UserInfoResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            username: u.username,
            role: u.role,
            is_permanent: u.is_permanent,
            expires_at: u.expires_at,
            created_at: u.created_at,
            avatar_url: u.avatar_url,
            bio: u.bio,
            display_name: u.display_name,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResetPasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeUsernameRequest {
    pub new_username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub display_name: Option<String>,
}