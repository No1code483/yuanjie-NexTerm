//! Agent 间通信 — 对标 Codex 的 InterAgentCommunication
//!
//! 提供 Agent 之间的消息传递机制，支持：
//! - Task: 任务委派
//! - Result: 结果返回
//! - Question: 提问
//! - Notification: 通知

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::models::agent::AgentMessageRequest;

/// Agent 间消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub from_agent_id: String,
    pub to_agent_id: String,
    pub content: String,
    pub message_type: String,
    pub timestamp: i64,
}

/// Agent 通信管理
#[derive(Debug, Default)]
pub struct AgentCommunication {
    messages: Mutex<HashMap<String, Vec<AgentMessage>>>,
    next_id: AtomicUsize,
}

impl AgentCommunication {
    pub fn new() -> Self {
        Self {
            messages: Mutex::new(HashMap::new()),
            next_id: AtomicUsize::new(0),
        }
    }

    /// 发送消息
    pub async fn send_message(&self, req: AgentMessageRequest) -> AgentMessage {
        let id = format!(
            "msg_{}",
            self.next_id.fetch_add(1, Ordering::Relaxed)
        );

        let to_id = req.to_agent_id.clone();
        let from_id = req.from_agent_id.clone();

        let msg = AgentMessage {
            id,
            from_agent_id: from_id,
            to_agent_id: to_id.clone(),
            content: req.content,
            message_type: req.message_type.as_str().to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        };

        // 存入接收者邮箱
        if let Ok(mut messages) = self.messages.lock() {
            messages
                .entry(to_id)
                .or_default()
                .push(msg.clone());
        }

        msg
    }

    /// 接收消息（从 Agent 邮箱取出）
    pub async fn receive_messages(&self, agent_id: &str) -> Vec<AgentMessage> {
        if let Ok(mut messages) = self.messages.lock() {
            messages.remove(agent_id).unwrap_or_default()
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    //! AgentCommunication 单元测试（v1.52 测试体系 Phase 2 - 2.5.1）
    //! 参见：03_测试体系_单元与集成测试.md §2.2.1（agent 优先级 P1）

    use super::*;
    use crate::models::agent::AgentMessageType;

    fn make_request(from: &str, to: &str, content: &str) -> AgentMessageRequest {
        AgentMessageRequest {
            from_agent_id: from.to_string(),
            to_agent_id: to.to_string(),
            content: content.to_string(),
            message_type: AgentMessageType::Task,
        }
    }

    #[tokio::test]
    async fn test_send_message_returns_message_with_id() {
        let comm = AgentCommunication::new();
        let msg = comm.send_message(make_request("a1", "a2", "hello")).await;
        assert!(msg.id.starts_with("msg_"), "id 应以 msg_ 开头");
        assert_eq!(msg.from_agent_id, "a1");
        assert_eq!(msg.to_agent_id, "a2");
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.message_type, "task");
        assert!(msg.timestamp > 0, "timestamp 应为正数");
    }

    #[tokio::test]
    async fn test_receive_messages_returns_sent_messages() {
        let comm = AgentCommunication::new();
        comm.send_message(make_request("a1", "a2", "msg1")).await;
        comm.send_message(make_request("a1", "a2", "msg2")).await;

        let received = comm.receive_messages("a2").await;
        assert_eq!(received.len(), 2, "应收到 2 条消息");
        assert_eq!(received[0].content, "msg1");
        assert_eq!(received[1].content, "msg2");
    }

    #[tokio::test]
    async fn test_receive_messages_empties_mailbox() {
        let comm = AgentCommunication::new();
        comm.send_message(make_request("a1", "a2", "once")).await;

        let first = comm.receive_messages("a2").await;
        assert_eq!(first.len(), 1);

        let second = comm.receive_messages("a2").await;
        assert_eq!(second.len(), 0, "邮箱取出后应清空");
    }

    #[tokio::test]
    async fn test_receive_messages_for_empty_mailbox_returns_empty() {
        let comm = AgentCommunication::new();
        let received = comm.receive_messages("nobody").await;
        assert!(received.is_empty(), "无邮件的 Agent 应返回空");
    }

    #[tokio::test]
    async fn test_message_ids_are_unique_and_monotonic() {
        let comm = AgentCommunication::new();
        let m1 = comm.send_message(make_request("a", "b", "1")).await;
        let m2 = comm.send_message(make_request("a", "b", "2")).await;
        let m3 = comm.send_message(make_request("a", "b", "3")).await;
        assert_ne!(m1.id, m2.id);
        assert_ne!(m2.id, m3.id);
        assert_ne!(m1.id, m3.id);
    }

    #[tokio::test]
    async fn test_messages_isolated_per_recipient() {
        let comm = AgentCommunication::new();
        comm.send_message(make_request("a1", "a2", "for_a2")).await;
        comm.send_message(make_request("a1", "a3", "for_a3")).await;

        let for_a2 = comm.receive_messages("a2").await;
        let for_a3 = comm.receive_messages("a3").await;
        assert_eq!(for_a2.len(), 1);
        assert_eq!(for_a2[0].content, "for_a2");
        assert_eq!(for_a3.len(), 1);
        assert_eq!(for_a3[0].content, "for_a3");
    }

    #[tokio::test]
    async fn test_message_type_preserved() {
        let comm = AgentCommunication::new();
        let req = AgentMessageRequest {
            from_agent_id: "a".into(),
            to_agent_id: "b".into(),
            content: "question".into(),
            message_type: AgentMessageType::Question,
        };
        let msg = comm.send_message(req).await;
        assert_eq!(msg.message_type, "question");
    }
}