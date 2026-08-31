// 流式事件系统 — 对标 Codex-rs 的 Event/EventMsg 体系
// 提供会话内的异步事件通知、前后端通信

use tokio::sync::broadcast;
use super::session::TokenUsage;

/// 事件类型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum YuanEvent {
    /// 会话状态变更
    SessionStateChanged {
        session_id: String,
        state: String,
    },
    /// 回合开始
    TurnStarted {
        session_id: String,
        turn_id: String,
        turn_number: usize,
    },
    /// 回合完成
    TurnCompleted {
        session_id: String,
        turn_id: String,
        token_usage: Option<TokenUsage>,
    },
    /// Token 使用统计
    TokenUsage {
        session_id: String,
        turn_id: String,
        usage: TokenUsage,
    },
    /// Agent 状态变更
    AgentStatusChanged {
        session_id: String,
        agent_id: String,
        status: String,
        message: Option<String>,
    },
    /// 工具调用开始
    ToolCallStarted {
        session_id: String,
        turn_id: String,
        tool_name: String,
        tool_input: String,
    },
    /// 工具调用完成
    ToolCallCompleted {
        session_id: String,
        turn_id: String,
        tool_name: String,
        tool_output: Option<String>,
        duration_ms: u64,
    },
    /// 上下文压缩触发
    CompactionTriggered {
        session_id: String,
        reason: String,
        tokens_before: u64,
        tokens_after: u64,
    },
    /// 流式文本增量
    StreamDelta {
        session_id: String,
        turn_id: String,
        delta: String,
        sequence: u64,
    },
    /// 流式文本结束
    StreamEnd {
        session_id: String,
        turn_id: String,
    },
    /// 目标更新
    GoalUpdated {
        session_id: String,
        goal_id: String,
        progress: f64,
        status: String,
    },
    /// 错误事件
    Error {
        session_id: String,
        error: String,
    },
    /// 心跳
    Heartbeat {
        session_id: String,
        timestamp: String,
    },
}

/// 事件总线
/// 基于 tokio::broadcast 实现的事件发布/订阅
pub struct EventBus {
    /// 发送端
    sender: broadcast::Sender<YuanEvent>,
    /// 接收端 (保留用于重连)
    _receiver: broadcast::Receiver<YuanEvent>,
    /// 事件计数器
    event_count: u64,
}

impl EventBus {
    /// 创建新的事件总线
    pub fn new() -> Self {
        let (sender, receiver) = broadcast::channel(256);
        Self {
            sender,
            _receiver: receiver,
            event_count: 0,
        }
    }

    /// 发布事件
    pub fn emit(&mut self, event: YuanEvent) {
        self.event_count += 1;
        let _ = self.sender.send(event);
    }

    /// 订阅事件流
    pub fn subscribe(&self) -> broadcast::Receiver<YuanEvent> {
        self.sender.subscribe()
    }

    /// 获取事件计数
    pub fn event_count(&self) -> u64 {
        self.event_count
    }

    /// 活跃订阅者数
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// 事件历史记录器
/// 保留最近的事件用于回放和调试
pub struct EventHistory {
    events: Vec<YuanEvent>,
    max_size: usize,
}

impl EventHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            events: Vec::with_capacity(max_size),
            max_size,
        }
    }

    pub fn record(&mut self, event: YuanEvent) {
        if self.events.len() >= self.max_size {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    pub fn recent(&self, count: usize) -> &[YuanEvent] {
        let start = self.events.len().saturating_sub(count);
        &self.events[start..]
    }

    pub fn all(&self) -> &[YuanEvent] {
        &self.events
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}