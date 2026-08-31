// 输入队列 — 对标 Codex-rs 的 InputQueue
// 管理用户输入的排队、优先级和处理

use std::collections::VecDeque;
use std::time::Instant;

/// 输入队列项
#[derive(Debug, Clone)]
pub struct InputItem {
    /// 输入 ID
    pub id: String,
    /// 输入内容
    pub content: String,
    /// 优先级 (0 = 最高)
    pub priority: u8,
    /// 入队时间
    pub enqueued_at: Instant,
    /// 输入类型
    pub input_type: InputType,
}

/// 输入类型
#[derive(Debug, Clone, PartialEq)]
pub enum InputType {
    /// 用户消息
    UserMessage,
    /// 系统指令
    SystemCommand,
    /// 工具调用结果
    ToolResult,
    /// 后续追问
    FollowUp,
}

/// 输入队列
pub struct InputQueue {
    /// 待处理队列
    queue: VecDeque<InputItem>,
    /// 最大队列长度
    max_size: usize,
    /// 是否正在处理
    processing: bool,
    /// 当前处理的输入
    current: Option<InputItem>,
    /// 总数计数器
    total_count: u64,
    /// 是否已取消
    cancelled: bool,
}

impl InputQueue {
    /// 创建新的输入队列
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            max_size,
            processing: false,
            current: None,
            total_count: 0,
            cancelled: false,
        }
    }

    /// 入队一个输入
    pub fn enqueue(&mut self, content: String, input_type: InputType, priority: u8) -> Result<String, String> {
        if self.queue.len() >= self.max_size {
            return Err("输入队列已满".into());
        }

        self.total_count += 1;
        let id = format!("input_{}", self.total_count);
        let item = InputItem {
            id: id.clone(),
            content,
            priority,
            enqueued_at: Instant::now(),
            input_type,
        };

        // 按优先级插入（值越小优先级越高）
        let pos = self.queue.iter().position(|i| i.priority > priority);
        match pos {
            Some(idx) => self.queue.insert(idx, item),
            None => self.queue.push_back(item),
        }

        Ok(id)
    }

    /// 从队列取出下一个输入
    pub fn dequeue(&mut self) -> Option<InputItem> {
        if self.cancelled || self.queue.is_empty() {
            return None;
        }

        let item = self.queue.pop_front()?;
        self.processing = true;
        self.current = Some(item.clone());
        Some(item)
    }

    /// 标记当前输入处理完成
    pub fn complete(&mut self) {
        self.processing = false;
        self.current = None;
    }

    /// 取消所有待处理输入
    pub fn cancel_all(&mut self) {
        self.cancelled = true;
        self.queue.clear();
        self.processing = false;
        self.current = None;
    }

    /// 重置取消状态
    pub fn reset(&mut self) {
        self.cancelled = false;
    }

    /// 队列长度
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// 是否正在处理
    pub fn is_processing(&self) -> bool {
        self.processing
    }

    /// 是否有待处理输入
    pub fn has_pending(&self) -> bool {
        !self.queue.is_empty() || self.processing
    }

    /// 当前处理的输入
    pub fn current(&self) -> Option<&InputItem> {
        self.current.as_ref()
    }

    /// 清空队列
    pub fn clear(&mut self) {
        self.queue.clear();
        self.processing = false;
        self.current = None;
    }

    /// 检查输入是否已超时
    pub fn cleanup_stale(&mut self, max_age_secs: u64) {
        self.queue.retain(|item| {
            item.enqueued_at.elapsed().as_secs() < max_age_secs
        });
    }
}

impl Default for InputQueue {
    fn default() -> Self {
        Self::new(100)
    }
}