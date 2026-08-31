//! Agent 状态跟踪 — 对标 Codex agent/status.rs
//!
//! 定义 Agent 生命周期状态，支持从 Engine 事件推导状态变迁。

use serde::{Deserialize, Serialize};

/// Agent 运行时状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// 等待初始化
    PendingInit,
    /// 空闲
    Idle,
    /// 思考中
    Thinking,
    /// 运行中
    Running,
    /// 执行中
    Executing,
    /// 等待用户输入
    WaitingForUser,
    /// 已完成（携带最后一条 Agent 消息）
    Completed(String),
    /// 已完成（无消息）
    CompletedEmpty,
    /// 已中断
    Interrupted,
    /// 错误
    Errored(String),
    /// 已关闭
    Shutdown,
}

impl AgentStatus {
    /// 是否为终态
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            Self::Completed(_) | Self::CompletedEmpty | Self::Errored(_) | Self::Shutdown
        )
    }

    /// 是否活跃
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::PendingInit | Self::Idle | Self::Thinking | Self::Running
                | Self::Executing | Self::WaitingForUser | Self::Interrupted
        )
    }

    /// 字符串表示
    pub fn as_str(&self) -> &str {
        match self {
            Self::PendingInit => "pending",
            Self::Idle => "idle",
            Self::Thinking => "thinking",
            Self::Running => "running",
            Self::Executing => "executing",
            Self::WaitingForUser => "waiting_for_user",
            Self::Completed(_) | Self::CompletedEmpty => "completed",
            Self::Interrupted => "interrupted",
            Self::Errored(_) => "errored",
            Self::Shutdown => "shutdown",
        }
    }

    /// 从字符串解析状态
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" | "pending_init" => Self::PendingInit,
            "idle" => Self::Idle,
            "thinking" => Self::Thinking,
            "running" => Self::Running,
            "executing" => Self::Executing,
            "waiting_for_user" => Self::WaitingForUser,
            "completed" => Self::CompletedEmpty,
            "interrupted" => Self::Interrupted,
            "shutdown" => Self::Shutdown,
            err => Self::Errored(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    //! AgentStatus 单元测试（v1.52 测试体系 Phase 2 - 2.5.1）
    //! 参见：03_测试体系_单元与集成测试.md §2.2.1（agent 优先级 P1）

    use super::*;

    // ===== is_final（终态判断）=====

    #[test]
    fn test_is_final_for_terminal_states() {
        assert!(AgentStatus::CompletedEmpty.is_final());
        assert!(AgentStatus::Completed("done".into()).is_final());
        assert!(AgentStatus::Errored("oops".into()).is_final());
        assert!(AgentStatus::Shutdown.is_final());
    }

    #[test]
    fn test_is_final_for_non_terminal_states() {
        assert!(!AgentStatus::PendingInit.is_final());
        assert!(!AgentStatus::Idle.is_final());
        assert!(!AgentStatus::Thinking.is_final());
        assert!(!AgentStatus::Running.is_final());
        assert!(!AgentStatus::Executing.is_final());
        assert!(!AgentStatus::WaitingForUser.is_final());
        assert!(!AgentStatus::Interrupted.is_final());
    }

    // ===== is_active（活跃判断）=====

    #[test]
    fn test_is_active_for_active_states() {
        assert!(AgentStatus::PendingInit.is_active());
        assert!(AgentStatus::Idle.is_active());
        assert!(AgentStatus::Thinking.is_active());
        assert!(AgentStatus::Running.is_active());
        assert!(AgentStatus::Executing.is_active());
        assert!(AgentStatus::WaitingForUser.is_active());
        assert!(AgentStatus::Interrupted.is_active());
    }

    #[test]
    fn test_is_active_for_inactive_states() {
        assert!(!AgentStatus::CompletedEmpty.is_active());
        assert!(!AgentStatus::Completed("done".into()).is_active());
        assert!(!AgentStatus::Errored("oops".into()).is_active());
        assert!(!AgentStatus::Shutdown.is_active());
    }

    // ===== as_str（字符串表示）=====

    #[test]
    fn test_as_str_matches_expected_strings() {
        assert_eq!(AgentStatus::PendingInit.as_str(), "pending");
        assert_eq!(AgentStatus::Idle.as_str(), "idle");
        assert_eq!(AgentStatus::Thinking.as_str(), "thinking");
        assert_eq!(AgentStatus::Running.as_str(), "running");
        assert_eq!(AgentStatus::Executing.as_str(), "executing");
        assert_eq!(AgentStatus::WaitingForUser.as_str(), "waiting_for_user");
        assert_eq!(AgentStatus::CompletedEmpty.as_str(), "completed");
        assert_eq!(AgentStatus::Completed("msg".into()).as_str(), "completed");
        assert_eq!(AgentStatus::Interrupted.as_str(), "interrupted");
        assert_eq!(AgentStatus::Errored("e".into()).as_str(), "errored");
        assert_eq!(AgentStatus::Shutdown.as_str(), "shutdown");
    }

    // ===== from_str（字符串解析）=====

    #[test]
    fn test_from_str_parses_known_states() {
        assert_eq!(AgentStatus::from_str("pending"), AgentStatus::PendingInit);
        assert_eq!(AgentStatus::from_str("idle"), AgentStatus::Idle);
        assert_eq!(AgentStatus::from_str("running"), AgentStatus::Running);
        assert_eq!(AgentStatus::from_str("completed"), AgentStatus::CompletedEmpty);
        assert_eq!(AgentStatus::from_str("shutdown"), AgentStatus::Shutdown);
    }

    #[test]
    fn test_from_str_is_case_insensitive() {
        assert_eq!(AgentStatus::from_str("IDLE"), AgentStatus::Idle);
        assert_eq!(AgentStatus::from_str("Running"), AgentStatus::Running);
    }

    #[test]
    fn test_from_str_unknown_becomes_errored() {
        let s = "unknown_state_xyz";
        match AgentStatus::from_str(s) {
            AgentStatus::Errored(msg) => assert_eq!(msg, s),
            other => panic!("未知状态应映射为 Errored，实际: {:?}", other),
        }
    }

    // ===== 往返一致性 =====

    #[test]
    fn test_roundtrip_for_states_without_payload() {
        // 无 payload 状态：from_str(as_str()) 应还原
        let states = vec![
            AgentStatus::PendingInit,
            AgentStatus::Idle,
            AgentStatus::Thinking,
            AgentStatus::Running,
            AgentStatus::Executing,
            AgentStatus::WaitingForUser,
            AgentStatus::CompletedEmpty,
            AgentStatus::Interrupted,
            AgentStatus::Shutdown,
        ];
        for s in states {
            let parsed = AgentStatus::from_str(s.as_str());
            assert_eq!(parsed, s, "往返不一致: {}", s.as_str());
        }
    }
}