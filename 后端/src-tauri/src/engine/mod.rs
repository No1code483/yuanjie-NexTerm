// Yuan Code 核心引擎 — 会话生命周期管理
// 对标 Codex-rs 的 Session/Turn/Thread 三层架构
//
// 架构层级:
//   YuanEngine (引擎总控)
//     └── YuanSession (会话管理)
//         ├── YuanTurn (对话回合)
//         │   └── TurnContext (上下文快照)
//         ├── EventBus (事件总线)
//         ├── InputQueue (输入队列)
//         └── SessionConfig (会话配置)

pub mod session;
pub mod turn;
pub mod event;
pub mod event_loop;
pub mod input_queue;
pub mod model;
pub mod model_retry;
pub mod model_sse;
pub mod config;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use self::session::YuanSession;
use self::config::SessionConfig;

/// 引擎总控
/// 管理所有活跃会话，负责会话的创建、销毁、状态查询
pub struct YuanEngine {
    /// 活跃会话映射 (session_id -> session)
    sessions: HashMap<String, Arc<RwLock<YuanSession>>>,
    /// 引擎配置
    config: EngineConfig,
}

/// 引擎配置
#[derive(Clone)]
pub struct EngineConfig {
    /// 最大并发会话数
    pub max_sessions: usize,
    /// 默认会话配置
    pub default_session: SessionConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_sessions: 10,
            default_session: SessionConfig::default(),
        }
    }
}

impl YuanEngine {
    /// 创建新的引擎实例
    pub fn new(config: EngineConfig) -> Self {
        Self {
            sessions: HashMap::new(),
            config,
        }
    }

    /// 创建新会话
    pub async fn create_session(
        &mut self,
        session_config: Option<SessionConfig>,
    ) -> Result<String, AppError> {
        if self.sessions.len() >= self.config.max_sessions {
            return Err(AppError::Internal(
                format!("已达到最大会话数限制 ({})", self.config.max_sessions)
            ));
        }

        let config = session_config.unwrap_or_else(|| self.config.default_session.clone());
        let session = YuanSession::new(config).await?;
        let session_id = session.id().to_string();

        self.sessions
            .insert(session_id.clone(), Arc::new(RwLock::new(session)));

        Ok(session_id)
    }

    /// 获取会话
    pub async fn get_session(
        &self,
        session_id: &str,
    ) -> Result<Arc<RwLock<YuanSession>>, AppError> {
        self.sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| AppError::Internal(format!("会话不存在: {}", session_id)))
    }

    /// 销毁会话
    pub async fn destroy_session(&mut self, session_id: &str) -> Result<(), AppError> {
        if let Some(_session) = self.sessions.remove(session_id) {
            // 会话清理逻辑
            tracing::info!("会话 {} 已销毁", session_id);
        }
        Ok(())
    }

    /// 列出所有活跃会话
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let mut infos = Vec::new();
        for (id, session) in &self.sessions {
            let session = session.read().await;
            infos.push(SessionInfo {
                id: id.clone(),
                created_at: session.created_at().to_string(),
                turn_count: session.turn_count(),
                is_active: session.is_active(),
            });
        }
        infos
    }

    /// 获取活跃会话数
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

/// 会话信息摘要
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub created_at: String,
    pub turn_count: usize,
    pub is_active: bool,
}