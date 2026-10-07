use std::sync::Arc;
use tokio::sync::RwLock;

use crate::compact::CompactionManager;
use crate::models::compact::{
    CompactionConfig, CompactionRequest, CompactionResult, CompactionSession, ConversationMessage,
    TokenBudget,
};

pub struct CompactService {
    manager: Arc<RwLock<CompactionManager>>,
}

impl CompactService {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(CompactionManager::new(None))),
        }
    }

    pub async fn update_config(&self, config: CompactionConfig) {
        let mut mgr = self.manager.write().await;
        mgr.update_config(config);
    }

    pub async fn get_config(&self) -> CompactionConfig {
        let mgr = self.manager.read().await;
        mgr.get_config().clone()
    }

    pub async fn estimate_tokens(&self, messages: Vec<ConversationMessage>) -> TokenBudget {
        let mgr = self.manager.read().await;
        mgr.estimate_tokens(&messages)
    }

    pub async fn needs_compaction(&self, messages: Vec<ConversationMessage>) -> bool {
        let mgr = self.manager.read().await;
        mgr.needs_compaction(&messages)
    }

    pub async fn compact(&self, request: CompactionRequest) -> CompactionResult {
        let mut mgr = self.manager.write().await;
        mgr.compact(request)
    }

    pub async fn get_session(&self, session_id: &str) -> Option<CompactionSession> {
        let mgr = self.manager.read().await;
        mgr.get_session(session_id).cloned()
    }

    pub async fn reset_session(&self, session_id: &str) {
        let mut mgr = self.manager.write().await;
        mgr.reset_session(session_id);
    }
}