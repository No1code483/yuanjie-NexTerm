//! Yuan Code v3.2 Task 3.4.4 — 协作会话管理（创建/加入/退出/在线状态/实时光标）
//!
//! 设计文档：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.4.4（Phase 6）
//!
//! ## 当前状态：内存态会话管理（PoC）
//!
//! 会话状态保存在内存中（重启丢失），符合 Phase 6 PoC 定位。
//! 后端仅做会话元数据管理（创建/加入/退出/在线状态/光标），
//! 文档内容同步由前端 yjs + y-websocket 直连完成（参见 yjs_doc.rs）。
//!
//! ## 与底层智能的边界
//!
//! - 协作会话是非 AI 功能，不调用云端 API，不依赖底层智能
//! - 底层智能可监测协作行为（通过 yuan_code_monitor::record_event），但不替代协作逻辑

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use crate::error::app_error::AppError;

/// 协作会话 ID（UUID 字符串）
pub type SessionId = String;

/// 用户 ID（与 users 表对齐，但本模块不依赖 DB）
pub type UserId = i64;

/// 协作会话中的用户信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CollabUser {
    pub user_id: UserId,
    /// 显示名（从 users 表读取，前端可缓存）
    pub display_name: String,
    /// 头像 URL（可空）
    pub avatar_url: Option<String>,
    /// 加入时间（Unix 秒）
    pub joined_at: i64,
    /// 最后活跃时间（Unix 秒）
    pub last_active_at: i64,
    /// 当前光标位置（可空，表示不在编辑）
    pub cursor: Option<CursorPosition>,
}

/// 光标位置（与 Monaco Editor Range 对齐）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CursorPosition {
    /// 文件路径（协作作用域内的相对路径）
    pub file_path: String,
    pub start_line: i32,
    pub start_column: i32,
    pub end_line: i32,
    pub end_column: i32,
}

/// 协作会话
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CollabSession {
    pub session_id: SessionId,
    /// 会话名称（如"重构登录模块"）
    pub name: String,
    /// 工作区根路径（协作作用域）
    pub workspace_root: String,
    /// 创建者 user_id
    pub created_by: UserId,
    /// 创建时间（Unix 秒）
    pub created_at: i64,
    /// 参与者列表（含创建者）
    pub participants: Vec<CollabUser>,
    /// 是否已关闭（关闭后无法加入）
    pub is_closed: bool,
}

/// 协作会话摘要（列表展示用，不含 participants 详情）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CollabSessionSummary {
    pub session_id: SessionId,
    pub name: String,
    pub workspace_root: String,
    pub created_by: UserId,
    pub created_at: i64,
    pub participant_count: usize,
    pub is_closed: bool,
}

impl From<&CollabSession> for CollabSessionSummary {
    fn from(s: &CollabSession) -> Self {
        Self {
            session_id: s.session_id.clone(),
            name: s.name.clone(),
            workspace_root: s.workspace_root.clone(),
            created_by: s.created_by,
            created_at: s.created_at,
            participant_count: s.participants.len(),
            is_closed: s.is_closed,
        }
    }
}

/// 协作会话管理器（内存态，进程内单例）
#[derive(Default)]
pub struct CollabSessionManager {
    /// session_id → CollabSession
    sessions: Mutex<HashMap<SessionId, CollabSession>>,
}

impl CollabSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建新会话
    pub async fn create_session(
        &self,
        name: String,
        workspace_root: String,
        creator: UserId,
        creator_display_name: String,
    ) -> Result<CollabSession, AppError> {
        let now = chrono::Utc::now().timestamp();
        let session_id = Uuid::new_v4().to_string();

        let creator_user = CollabUser {
            user_id: creator,
            display_name: creator_display_name,
            avatar_url: None,
            joined_at: now,
            last_active_at: now,
            cursor: None,
        };

        let session = CollabSession {
            session_id: session_id.clone(),
            name,
            workspace_root,
            created_by: creator,
            created_at: now,
            participants: vec![creator_user],
            is_closed: false,
        };

        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id, session.clone());

        tracing::info!(
            "[collab] 会话已创建: session_id={}, name={}, creator={}",
            session.session_id,
            session.name,
            creator
        );

        Ok(session)
    }

    /// 列出所有活跃会话（未关闭）
    pub async fn list_sessions(&self) -> Vec<CollabSessionSummary> {
        let sessions = self.sessions.lock().await;
        sessions
            .values()
            .filter(|s| !s.is_closed)
            .map(CollabSessionSummary::from)
            .collect()
    }

    /// 获取会话详情
    pub async fn get_session(&self, session_id: &str) -> Result<CollabSession, AppError> {
        let sessions = self.sessions.lock().await;
        sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| AppError::NotFound)
    }

    /// 加入会话
    pub async fn join_session(
        &self,
        session_id: &str,
        user_id: UserId,
        display_name: String,
    ) -> Result<CollabSession, AppError> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::NotFound)?;

        if session.is_closed {
            return Err(AppError::Validation("会话已关闭，无法加入".into()));
        }

        let now = chrono::Utc::now().timestamp();

        // 已存在则更新 last_active_at；否则新增
        if let Some(existing) = session.participants.iter_mut().find(|p| p.user_id == user_id) {
            existing.last_active_at = now;
            existing.display_name = display_name;
        } else {
            session.participants.push(CollabUser {
                user_id,
                display_name,
                avatar_url: None,
                joined_at: now,
                last_active_at: now,
                cursor: None,
            });
        }

        let updated = session.clone();
        tracing::info!(
            "[collab] 用户加入会话: session_id={}, user_id={}, participants={}",
            session_id,
            user_id,
            updated.participants.len()
        );
        Ok(updated)
    }

    /// 退出会话
    pub async fn leave_session(
        &self,
        session_id: &str,
        user_id: UserId,
    ) -> Result<(), AppError> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::NotFound)?;

        let before = session.participants.len();
        session.participants.retain(|p| p.user_id != user_id);
        let after = session.participants.len();

        if before == after {
            // 用户不在会话中（幂等返回 Ok）
            return Ok(());
        }

        // 若会话无人则自动关闭
        if session.participants.is_empty() {
            session.is_closed = true;
            tracing::info!(
                "[collab] 会话无参与者，自动关闭: session_id={}",
                session_id
            );
        }

        tracing::info!(
            "[collab] 用户退出会话: session_id={}, user_id={}",
            session_id,
            user_id
        );
        Ok(())
    }

    /// 更新用户光标位置（实时光标）
    pub async fn update_cursor(
        &self,
        session_id: &str,
        user_id: UserId,
        cursor: Option<CursorPosition>,
    ) -> Result<CollabSession, AppError> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::NotFound)?;

        let now = chrono::Utc::now().timestamp();
        let user = session
            .participants
            .iter_mut()
            .find(|p| p.user_id == user_id)
            .ok_or_else(|| {
                AppError::Validation("用户不在会话中，请先 join_session".into())
            })?;

        user.cursor = cursor;
        user.last_active_at = now;

        Ok(session.clone())
    }

    /// 关闭会话（仅创建者可关闭）
    pub async fn close_session(
        &self,
        session_id: &str,
        user_id: UserId,
    ) -> Result<(), AppError> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::NotFound)?;

        if session.created_by != user_id {
            return Err(AppError::Permission {
                resource: "collab_session".into(),
                action: "close".into(),
            });
        }

        session.is_closed = true;
        tracing::info!("[collab] 会话已关闭: session_id={}", session_id);
        Ok(())
    }
}

/// 进程内单例（与 IntelligenceService 模式一致，由 AppState 持有）
pub fn shared_manager() -> Arc<CollabSessionManager> {
    Arc::new(CollabSessionManager::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_join_leave_session() {
        let mgr = CollabSessionManager::new();

        // 创建会话
        let session = mgr
            .create_session(
                "重构登录模块".into(),
                "/workspace/project".into(),
                1,
                "Alice".into(),
            )
            .await
            .unwrap();
        assert_eq!(session.participants.len(), 1);
        assert_eq!(session.created_by, 1);
        assert!(!session.is_closed);

        // 用户 2 加入
        let updated = mgr
            .join_session(&session.session_id, 2, "Bob".into())
            .await
            .unwrap();
        assert_eq!(updated.participants.len(), 2);

        // 用户 2 退出
        mgr.leave_session(&session.session_id, 2).await.unwrap();
        let after = mgr.get_session(&session.session_id).await.unwrap();
        assert_eq!(after.participants.len(), 1);
        assert!(!after.is_closed); // 仍有创建者，不关闭
    }

    #[tokio::test]
    async fn test_session_auto_close_when_empty() {
        let mgr = CollabSessionManager::new();
        let session = mgr
            .create_session("solo".into(), "/ws".into(), 1, "Alice".into())
            .await
            .unwrap();

        mgr.leave_session(&session.session_id, 1).await.unwrap();
        let after = mgr.get_session(&session.session_id).await.unwrap();
        assert!(after.is_closed);
    }

    #[tokio::test]
    async fn test_join_closed_session_fails() {
        let mgr = CollabSessionManager::new();
        let session = mgr
            .create_session("closed".into(), "/ws".into(), 1, "Alice".into())
            .await
            .unwrap();
        mgr.close_session(&session.session_id, 1).await.unwrap();

        let err = mgr
            .join_session(&session.session_id, 2, "Bob".into())
            .await
            .unwrap_err();
        match err {
            AppError::Validation(msg) => assert!(msg.contains("已关闭")),
            _ => panic!("应返回 Validation 错误"),
        }
    }

    #[tokio::test]
    async fn test_close_session_permission_denied_for_non_creator() {
        let mgr = CollabSessionManager::new();
        let session = mgr
            .create_session("perm".into(), "/ws".into(), 1, "Alice".into())
            .await
            .unwrap();
        mgr.join_session(&session.session_id, 2, "Bob".into())
            .await
            .unwrap();

        let err = mgr.close_session(&session.session_id, 2).await.unwrap_err();
        match err {
            AppError::Permission { resource, action } => {
                assert_eq!(resource, "collab_session");
                assert_eq!(action, "close");
            }
            _ => panic!("应返回 Permission 错误"),
        }
    }

    #[tokio::test]
    async fn test_update_cursor_requires_join_first() {
        let mgr = CollabSessionManager::new();
        let session = mgr
            .create_session("cursor".into(), "/ws".into(), 1, "Alice".into())
            .await
            .unwrap();

        // 用户 2 未加入就更新光标 → 应失败
        let err = mgr
            .update_cursor(&session.session_id, 2, None)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // 加入后更新光标 → 应成功
        mgr.join_session(&session.session_id, 2, "Bob".into())
            .await
            .unwrap();
        let updated = mgr
            .update_cursor(
                &session.session_id,
                2,
                Some(CursorPosition {
                    file_path: "src/main.rs".into(),
                    start_line: 10,
                    start_column: 5,
                    end_line: 10,
                    end_column: 15,
                }),
            )
            .await
            .unwrap();
        let bob = updated.participants.iter().find(|p| p.user_id == 2).unwrap();
        assert!(bob.cursor.is_some());
        assert_eq!(bob.cursor.as_ref().unwrap().file_path, "src/main.rs");
    }

    #[tokio::test]
    async fn test_list_sessions_excludes_closed() {
        let mgr = CollabSessionManager::new();
        let s1 = mgr
            .create_session("active".into(), "/ws1".into(), 1, "Alice".into())
            .await
            .unwrap();
        let s2 = mgr
            .create_session("closed".into(), "/ws2".into(), 1, "Alice".into())
            .await
            .unwrap();
        mgr.close_session(&s2.session_id, 1).await.unwrap();

        let list = mgr.list_sessions().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].session_id, s1.session_id);
    }

    #[tokio::test]
    async fn test_rejoin_updates_existing_participant() {
        let mgr = CollabSessionManager::new();
        let session = mgr
            .create_session("rejoin".into(), "/ws".into(), 1, "Alice".into())
            .await
            .unwrap();

        // 用户 2 加入
        mgr.join_session(&session.session_id, 2, "Bob".into())
            .await
            .unwrap();
        // 再次加入（更新 display_name）
        let updated = mgr
            .join_session(&session.session_id, 2, "Bobby".into())
            .await
            .unwrap();
        assert_eq!(updated.participants.len(), 2); // 不重复添加
        let bob = updated.participants.iter().find(|p| p.user_id == 2).unwrap();
        assert_eq!(bob.display_name, "Bobby");
    }
}
