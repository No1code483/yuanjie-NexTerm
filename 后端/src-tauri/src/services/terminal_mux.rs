use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast;

use crate::error::app_error::AppError;
use crate::models::terminal::{TerminalSession, TerminalSessionInput};
use crate::services::terminal_service::TerminalService;

/// MuxNotification — 对标 WezTerm 的 MuxNotification 事件系统
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MuxNotification {
    SessionCreated {
        session_id: String,
        session_type: String,
        tab_id: Option<String>,
        pane_id: Option<String>,
    },
    SessionKilled {
        session_id: String,
        exit_code: i32,
    },
    SessionResized {
        session_id: String,
        cols: u16,
        rows: u16,
    },
    SessionError {
        session_id: String,
        error: String,
    },
}

/// TerminalMux — 对标 WezTerm 的 Mux 层
/// 职责：
/// 1. 包装 TerminalService，管理 PTY 会话生命周期
/// 2. 持久化会话元数据到 terminal_sessions 表
/// 3. 通过 Tauri 事件系统广播会话状态变更
pub struct TerminalMux {
    pub terminal_service: TerminalService,
    pool: SqlitePool,
    event_tx: broadcast::Sender<MuxNotification>,
}

impl TerminalMux {
    pub fn new(pool: SqlitePool) -> Self {
        let (event_tx, _) = broadcast::channel(64);
        Self {
            terminal_service: TerminalService::new(),
            pool,
            event_tx,
        }
    }

    /// 获取事件接收器，供前端订阅
    pub fn subscribe(&self) -> broadcast::Receiver<MuxNotification> {
        self.event_tx.subscribe()
    }

    /// 创建 PTY 会话并持久化元数据
    pub async fn create_session(
        &self,
        user_id: i64,
        session_type: &str,
        tab_id: Option<String>,
        pane_id: Option<String>,
        cols: u16,
        rows: u16,
        app_handle: AppHandle,
    ) -> Result<String, AppError> {
        let session_id = self.terminal_service.create_session(session_type, app_handle.clone())?;

        let now = chrono::Utc::now().timestamp_millis();
        let input = TerminalSessionInput {
            id: session_id.clone(),
            session_type: session_type.to_string(),
            tab_id: tab_id.clone(),
            pane_id: pane_id.clone(),
            cols: cols as i64,
            rows: rows as i64,
        };
        crate::db::repositories::terminal_repo::save_session(&self.pool, user_id, &input, now).await?;

        let notification = MuxNotification::SessionCreated {
            session_id: session_id.clone(),
            session_type: session_type.to_string(),
            tab_id,
            pane_id,
        };
        let _ = self.event_tx.send(notification.clone());
        let _ = app_handle.emit("mux:notification", notification);

        tracing::info!(session_id, session_type, "Mux session created");
        Ok(session_id)
    }

    /// 写入 PTY 输入
    pub fn write_input(
        &self,
        session_id: &str,
        data: &str,
    ) -> Result<(), AppError> {
        self.terminal_service.write_input(session_id, data)
    }

    /// 调整 PTY 尺寸并持久化
    pub async fn resize(
        &self,
        session_id: &str,
        cols: u16,
        rows: u16,
        app_handle: &AppHandle,
    ) -> Result<(), AppError> {
        self.terminal_service.resize(session_id, cols, rows)?;

        let notification = MuxNotification::SessionResized {
            session_id: session_id.to_string(),
            cols,
            rows,
        };
        let _ = self.event_tx.send(notification.clone());
        let _ = app_handle.emit("mux:notification", notification);

        Ok(())
    }

    /// 终止会话，返回退出码
    pub async fn kill_session(
        &self,
        user_id: i64,
        session_id: &str,
        app_handle: &AppHandle,
    ) -> Result<i32, AppError> {
        let exit_code = self.terminal_service.kill_session(session_id)?;

        let now = chrono::Utc::now().timestamp_millis();
        crate::db::repositories::terminal_repo::update_session_killed(&self.pool, user_id, session_id, now).await?;

        let notification = MuxNotification::SessionKilled {
            session_id: session_id.to_string(),
            exit_code,
        };
        let _ = self.event_tx.send(notification.clone());
        let _ = app_handle.emit("mux:notification", notification);

        tracing::info!(session_id, exit_code, "Mux session killed");
        Ok(exit_code)
    }

    /// 获取活跃会话列表
    pub async fn get_active_sessions(&self, user_id: i64) -> Result<Vec<TerminalSession>, AppError> {
        crate::db::repositories::terminal_repo::get_active_sessions(&self.pool, user_id).await
    }

    /// 获取所有活跃会话信息
    pub fn list_sessions(&self) -> Vec<crate::services::terminal_service::SessionInfo> {
        self.terminal_service.list_sessions()
    }

    /// 清理所有持久化会话记录
    pub async fn clear_all_sessions(&self, user_id: i64) -> Result<u64, AppError> {
        crate::db::repositories::terminal_repo::delete_all_sessions(&self.pool, user_id).await
    }
}