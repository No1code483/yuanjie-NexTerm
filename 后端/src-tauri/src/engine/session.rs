// YuanSession — 会话生命周期管理器
// 对标 Codex-rs 的 Session，管理单个对话会话的完整生命周期

use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

use crate::error::app_error::AppError;
use super::config::SessionConfig;
use super::turn::{YuanTurn, TurnContext};
use super::event::{EventBus, YuanEvent};

/// 会话状态
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    /// 初始化中
    Initializing,
    /// 等待用户输入
    WaitingForInput,
    /// 处理中
    Processing,
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 已终止
    Terminated,
}

/// 会话生命周期管理器
pub struct YuanSession {
    /// 会话 ID
    id: String,
    /// 会话状态
    state: SessionState,
    /// 创建时间
    created_at: String,
    /// 会话配置
    config: SessionConfig,
    /// 所有回合
    turns: Vec<YuanTurn>,
    /// 当前活跃回合
    active_turn: Option<YuanTurn>,
    /// 上下文快照
    context: Option<TurnContext>,
    /// 事件总线
    event_bus: Arc<RwLock<EventBus>>,
    /// 输入队列
    #[allow(dead_code)]
    input_queue: Vec<String>,
    /// 是否活跃
    active: bool,
}

impl YuanSession {
    /// 创建新会话
    pub async fn new(config: SessionConfig) -> Result<Self, AppError> {
        let session_id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();

        Ok(Self {
            id: session_id,
            state: SessionState::Initializing,
            created_at,
            config,
            turns: Vec::new(),
            active_turn: None,
            context: None,
            event_bus: Arc::new(RwLock::new(EventBus::new())),
            input_queue: Vec::new(),
            active: true,
        })
    }

    /// 会话 ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 创建时间
    pub fn created_at(&self) -> &str {
        &self.created_at
    }

    /// 回合数
    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// 是否活跃
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// 当前状态
    pub fn state(&self) -> &SessionState {
        &self.state
    }

    /// 开始新的回合
    pub async fn start_turn(&mut self, user_input: String) -> Result<&YuanTurn, AppError> {
        if !self.active {
            return Err(AppError::Internal("会话已结束".into()));
        }

        // 保存当前上下文快照
        self.context = Some(TurnContext::snapshot(&self.turns, &self.config));

        // 创建新回合
        let turn_number = self.turns.len() + 1;
        let mut turn = YuanTurn::new(turn_number, user_input, self.context.clone());
        turn.start();

        self.state = SessionState::Processing;
        self.active_turn = Some(turn.clone());

        // 发送事件
        let event = YuanEvent::TurnStarted {
            session_id: self.id.clone(),
            turn_id: turn.id().to_string(),
            turn_number,
        };
        self.event_bus.write().await.emit(event);

        self.turns.push(turn);
        Ok(self.turns.last().unwrap())
    }

    /// 完成当前回合
    pub async fn complete_turn(
        &mut self,
        response: String,
        token_usage: Option<TokenUsage>,
    ) -> Result<(), AppError> {
        if let Some(turn) = self.active_turn.as_mut() {
            turn.complete(response, token_usage);

            // 发送事件
            let event = YuanEvent::TurnCompleted {
                session_id: self.id.clone(),
                turn_id: turn.id().to_string(),
                token_usage: turn.token_usage().cloned(),
            };
            self.event_bus.write().await.emit(event);
        }

        self.active_turn = None;
        self.state = SessionState::WaitingForInput;

        Ok(())
    }

    /// 中止当前回合
    pub async fn abort_turn(&mut self) -> Result<(), AppError> {
        if let Some(turn) = self.active_turn.as_mut() {
            turn.abort();

            let event = YuanEvent::Error {
                session_id: self.id.clone(),
                error: "回合被中止".into(),
            };
            self.event_bus.write().await.emit(event);
        }

        self.active_turn = None;
        self.state = SessionState::WaitingForInput;

        Ok(())
    }

    /// 暂停会话
    pub async fn pause(&mut self) {
        self.state = SessionState::Paused;
        let event = YuanEvent::SessionStateChanged {
            session_id: self.id.clone(),
            state: "paused".into(),
        };
        self.event_bus.write().await.emit(event);
    }

    /// 继续会话
    pub async fn resume(&mut self) {
        self.state = SessionState::WaitingForInput;
        let event = YuanEvent::SessionStateChanged {
            session_id: self.id.clone(),
            state: "active".into(),
        };
        self.event_bus.write().await.emit(event);
    }

    /// 终止会话
    pub async fn terminate(&mut self) {
        self.active = false;
        self.state = SessionState::Terminated;
        let event = YuanEvent::SessionStateChanged {
            session_id: self.id.clone(),
            state: "terminated".into(),
        };
        self.event_bus.write().await.emit(event);
    }

    /// 获取事件总线
    pub fn event_bus(&self) -> Arc<RwLock<EventBus>> {
        self.event_bus.clone()
    }

    /// 获取当前上下文
    pub fn context(&self) -> Option<&TurnContext> {
        self.context.as_ref()
    }

    /// 获取所有回合
    pub fn turns(&self) -> &[YuanTurn] {
        &self.turns
    }

    /// 获取活跃回合
    pub fn active_turn(&self) -> Option<&YuanTurn> {
        self.active_turn.as_ref()
    }
}

/// Token 用量统计
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenUsage {
    /// 输入 Token 数
    pub input_tokens: u64,
    /// 输出 Token 数
    pub output_tokens: u64,
    /// 总 Token 数
    pub total_tokens: u64,
    /// 估算成本 (USD)
    pub estimated_cost: f64,
}

impl TokenUsage {
    pub fn new(input_tokens: u64, output_tokens: u64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            estimated_cost: 0.0,
        }
    }
}