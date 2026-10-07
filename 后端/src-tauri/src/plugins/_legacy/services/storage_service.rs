use chrono::Utc;

use crate::error::app_error::AppError;

const DEBOUNCE_MS: u64 = 300;
const MAX_RETRY: u32 = 3;

pub async fn debounce_save() {
    tokio::time::sleep(tokio::time::Duration::from_millis(DEBOUNCE_MS)).await;
}

pub fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

pub fn check_version_conflict(expected_version: i32, current_version: i32) -> Result<(), AppError> {
    if expected_version != current_version {
        return Err(AppError::Conflict(format!(
            "版本冲突: 期望版本 {}, 当前版本 {}",
            expected_version, current_version
        )));
    }
    Ok(())
}

pub fn build_versioned_update(
    expected_version: i32,
) -> (i32, i64) {
    let now = now_ms();
    let new_version = expected_version + 1;
    (new_version, now)
}

pub async fn retry_on_conflict<F, Fut, T>(mut operation: F) -> Result<T, AppError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, AppError>>,
{
    let mut last_error: Option<AppError> = None;
    for attempt in 0..MAX_RETRY {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(AppError::Conflict(_)) => {
                last_error = Some(AppError::Conflict(format!(
                    "版本冲突，已重试 {} 次",
                    attempt + 1
                )));
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            Err(e) => return Err(e),
        }
    }
    Err(last_error.unwrap_or_else(|| AppError::Conflict("版本冲突重试耗尽".into())))
}

pub struct StorageService;

impl StorageService {
    pub async fn auto_save<T, F, Fut>(save_fn: F) -> Result<T, AppError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, AppError>>,
    {
        debounce_save().await;
        retry_on_conflict(save_fn).await
    }
}