use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::models::safety::{SafetyCheckRequest, SafetyCheckResult, SafetyProfile};
use crate::safety::SafetyManager;

pub struct SafetyService {
    manager: Arc<RwLock<SafetyManager>>,
}

impl SafetyService {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(SafetyManager::new())),
        }
    }

    pub async fn check(&self, req: SafetyCheckRequest) -> Result<SafetyCheckResult, AppError> {
        let mgr = self.manager.read().await;
        Ok(mgr.check(&req))
    }

    pub async fn quick_check(&self, content: &str) -> Result<bool, AppError> {
        let mgr = self.manager.read().await;
        Ok(mgr.quick_check(content))
    }

    pub async fn set_profile(&self, profile: SafetyProfile) -> Result<(), AppError> {
        let mut mgr = self.manager.write().await;
        mgr.set_profile(profile);
        Ok(())
    }

    pub async fn get_profile(&self) -> Result<SafetyProfile, AppError> {
        let mgr = self.manager.read().await;
        Ok(mgr.profile().clone())
    }
}