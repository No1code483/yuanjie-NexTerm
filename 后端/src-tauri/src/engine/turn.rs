// YuanTurn — 单次对话回合
// 对标 Codex-rs 的 Turn，代表一次 "用户输入 → AI 响应" 的完整回合

use uuid::Uuid;
use chrono::Utc;
use super::config::SessionConfig;
use super::session::TokenUsage;

/// 回合状态
#[derive(Debug, Clone, PartialEq)]
pub enum TurnStatus {
    /// 初始化
    Initialized,
    /// 进行中
    InProgress,
    /// 已完成
    Completed,
    /// 已中止
    Aborted,
    /// 出错
    Failed,
}

/// 上下文快照 — 回合开始时不可变的上下文
#[derive(Debug, Clone)]
pub struct TurnContext {
    /// 压缩后的历史消息
    pub compacted_history: Vec<HistoryMessage>,
    /// 当前 Token 预算
    pub token_budget: u64,
    /// 已使用 Token 数
    pub tokens_used: u64,
    /// 系统提示
    pub system_prompt: String,
    /// 活跃工具列表
    pub active_tools: Vec<String>,
    /// 快照时间
    pub snapshot_at: String,
}

/// 历史消息
#[derive(Debug, Clone)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

impl TurnContext {
    /// 创建上下文快照
    pub fn snapshot(turns: &[YuanTurn], config: &SessionConfig) -> Self {
        let mut history = Vec::new();
        for turn in turns {
            history.push(HistoryMessage {
                role: "user".into(),
                content: turn.user_input.clone(),
                timestamp: turn.started_at.clone(),
            });
            if let Some(ref response) = turn.response {
                history.push(HistoryMessage {
                    role: "assistant".into(),
                    content: response.clone(),
                    timestamp: turn.completed_at.clone().unwrap_or_default(),
                });
            }
        }

        Self {
            compacted_history: history,
            token_budget: config.token_budget,
            tokens_used: 0,
            system_prompt: config.system_prompt.clone(),
            active_tools: config.active_tools.clone(),
            snapshot_at: Utc::now().to_rfc3339(),
        }
    }
}

/// 单次对话回合
#[derive(Debug, Clone)]
pub struct YuanTurn {
    /// 回合 ID
    id: String,
    /// 回合编号
    number: usize,
    /// 用户输入
    pub user_input: String,
    /// AI 响应
    pub response: Option<String>,
    /// 回合状态
    status: TurnStatus,
    /// Token 用量
    token_usage: Option<TokenUsage>,
    /// 开始时间
    started_at: String,
    /// 完成时间
    completed_at: Option<String>,
    /// 上下文快照
    context: Option<TurnContext>,
    /// 流式响应块
    stream_chunks: Vec<String>,
}

impl YuanTurn {
    /// 创建新回合
    pub fn new(number: usize, user_input: String, context: Option<TurnContext>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            number,
            user_input,
            response: None,
            status: TurnStatus::Initialized,
            token_usage: None,
            started_at: Utc::now().to_rfc3339(),
            completed_at: None,
            context,
            stream_chunks: Vec::new(),
        }
    }

    /// 回合 ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 回合编号
    pub fn number(&self) -> usize {
        self.number
    }

    /// 开始回合
    pub fn start(&mut self) {
        self.status = TurnStatus::InProgress;
    }

    /// 完成回合
    pub fn complete(&mut self, response: String, token_usage: Option<TokenUsage>) {
        self.response = Some(response);
        self.status = TurnStatus::Completed;
        self.token_usage = token_usage;
        self.completed_at = Some(Utc::now().to_rfc3339());
    }

    /// 中止回合
    pub fn abort(&mut self) {
        self.status = TurnStatus::Aborted;
        self.completed_at = Some(Utc::now().to_rfc3339());
    }

    /// 失败
    pub fn fail(&mut self, error: &str) {
        self.status = TurnStatus::Failed;
        self.response = Some(format!("[错误] {}", error));
        self.completed_at = Some(Utc::now().to_rfc3339());
    }

    /// 添加流式响应块
    pub fn add_stream_chunk(&mut self, chunk: String) {
        self.stream_chunks.push(chunk);
    }

    /// 获取完整流式响应
    pub fn streamed_response(&self) -> String {
        self.stream_chunks.join("")
    }

    /// Token 用量
    pub fn token_usage(&self) -> Option<&TokenUsage> {
        self.token_usage.as_ref()
    }

    /// 回合状态
    pub fn status(&self) -> &TurnStatus {
        &self.status
    }

    /// 是否已完成
    pub fn is_completed(&self) -> bool {
        matches!(self.status, TurnStatus::Completed | TurnStatus::Aborted | TurnStatus::Failed)
    }

    /// 上下文
    pub fn context(&self) -> Option<&TurnContext> {
        self.context.as_ref()
    }

    /// 耗时 (秒)
    pub fn duration_secs(&self) -> Option<f64> {
        if let (Some(start), Some(end)) = (
            chrono::DateTime::parse_from_rfc3339(&self.started_at).ok(),
            self.completed_at.as_ref().and_then(|c| chrono::DateTime::parse_from_rfc3339(c).ok()),
        ) {
            Some((end - start).num_milliseconds() as f64 / 1000.0)
        } else {
            None
        }
    }
}