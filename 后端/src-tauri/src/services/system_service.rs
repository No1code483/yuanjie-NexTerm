use sqlx::SqlitePool;

use crate::db::repositories::system_repo;
use crate::error::app_error::AppError;
use crate::models::system::SystemConfig;

pub async fn get_config(pool: &SqlitePool, key: &str) -> Result<Option<SystemConfig>, AppError> {
    system_repo::get_config(pool, key).await
}

pub async fn set_config(pool: &SqlitePool, key: &str, value: &str) -> Result<SystemConfig, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    system_repo::set_config(pool, key, value, now).await
}

pub async fn get_all_configs(pool: &SqlitePool) -> Result<Vec<SystemConfig>, AppError> {
    system_repo::get_all_configs(pool).await
}

pub fn open_file(path: &str) -> Result<(), AppError> {
    opener::open(path).map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
}

pub fn open_url(url: &str) -> Result<(), AppError> {
    opener::open(url).map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
}