// 事件循环 — 对标 Codex-rs 的 EventLoop
// 驱动会话的生命周期，协调输入队列、turn 流转和事件发送

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::error::app_error::AppError;
use super::session::{YuanSession, TokenUsage};
use super::event::YuanEvent;
use super::input_queue::InputQueue;

/// 事件循环状态
#[derive(Debug, Clone, PartialEq)]
pub enum LoopState {
    /// 运行中
    Running,
    /// 暂停中
    Paused,
    /// 停止中
    Stopping,
    /// 已停止
    Stopped,
}

/// 事件循环
/// 负责驱动会话的主循环：从输入队列取输入 → 创建 Turn → 等待处理完成 → 发送事件
pub struct EventLoop {
    /// 会话实例
    session: Arc<RwLock<YuanSession>>,
    /// 输入队列
    input_queue: Arc<RwLock<InputQueue>>,
    /// 循环状态
    state: LoopState,
    /// 循环句柄
    handle: Option<JoinHandle<()>>,
}

impl EventLoop {
    /// 创建新的事件循环
    pub fn new(session: Arc<RwLock<YuanSession>>) -> Self {
        Self {
            session,
            input_queue: Arc::new(RwLock::new(InputQueue::default())),
            state: LoopState::Stopped,
            handle: None,
        }
    }

    /// 获取输入队列
    pub fn input_queue(&self) -> Arc<RwLock<InputQueue>> {
        self.input_queue.clone()
    }

    /// 提交用户输入
    pub async fn submit_input(&self, content: String) -> Result<String, String> {
        let mut queue = self.input_queue.write().await;
        queue.enqueue(content, super::input_queue::InputType::UserMessage, 0)
    }

    /// 启动事件循环
    pub async fn start(&mut self) -> Result<(), AppError> {
        if self.state == LoopState::Running {
            return Ok(());
        }

        self.state = LoopState::Running;
        let session = self.session.clone();
        let queue = self.input_queue.clone();

        let handle = tokio::spawn(async move {
            loop {
                // 检查队列中是否有待处理输入
                let input = {
                    let mut q = queue.write().await;
                    if q.is_empty() {
                        // 无输入时短暂休眠
                        drop(q);
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        continue;
                    }
                    q.dequeue()
                };

                if let Some(input) = input {
                    let session_id = {
                        let sess = session.read().await;
                        sess.id().to_string()
                    };

                    // 开始新回合
                    let turn_result = {
                        let mut sess = session.write().await;
                        sess.start_turn(input.content).await.map(|t| t.clone())
                    };

                    match turn_result {
                        Ok(turn) => {
                            // 发送流式开始事件
                            {
                                let sess = session.read().await;
                                let event_bus = sess.event_bus();
                                let mut bus = event_bus.write().await;
                                bus.emit(YuanEvent::StreamDelta {
                                    session_id: session_id.clone(),
                                    turn_id: turn.id().to_string(),
                                    delta: String::new(),
                                    sequence: 0,
                                });
                            }

                            // 回合处理完成
                            let completed = {
                                let mut sess = session.write().await;
                                sess.complete_turn(
                                    String::new(),
                                    Some(TokenUsage::new(0, 0)),
                                ).await
                            };

                            if let Err(e) = completed {
                                let sess = session.read().await;
                                let event_bus = sess.event_bus();
                                let mut bus = event_bus.write().await;
                                bus.emit(YuanEvent::Error {
                                    session_id: session_id.clone(),
                                    error: format!("回合处理失败: {}", e),
                                });
                            }

                            // 发送流式结束事件
                            {
                                let sess = session.read().await;
                                let event_bus = sess.event_bus();
                                let mut bus = event_bus.write().await;
                                bus.emit(YuanEvent::StreamEnd {
                                    session_id: session_id.clone(),
                                    turn_id: turn.id().to_string(),
                                });
                            }

                            // 标记输入处理完成
                            {
                                let mut q = queue.write().await;
                                q.complete();
                            }
                        }
                        Err(e) => {
                            let sess = session.read().await;
                            let event_bus = sess.event_bus();
                            let mut bus = event_bus.write().await;
                            bus.emit(YuanEvent::Error {
                                session_id: session_id.clone(),
                                error: format!("开始回合失败: {}", e),
                            });
                        }
                    }

                    // 检查是否应该停止
                    let should_stop = {
                        let sess = session.read().await;
                        !sess.is_active()
                    };

                    if should_stop {
                        break;
                    }
                }
            }
        });

        self.handle = Some(handle);

        Ok(())
    }

    /// 暂停事件循环
    pub async fn pause(&mut self) {
        self.state = LoopState::Paused;
        let mut sess = self.session.write().await;
        sess.pause().await;
    }

    /// 恢复事件循环
    pub async fn resume(&mut self) {
        self.state = LoopState::Running;
        let mut sess = self.session.write().await;
        sess.resume().await;
    }

    /// 停止事件循环
    pub async fn stop(&mut self) {
        self.state = LoopState::Stopping;
        let mut sess = self.session.write().await;
        sess.terminate().await;

        // 取消所有待处理输入
        let mut queue = self.input_queue.write().await;
        queue.cancel_all();

        // 中止当前任务
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }

        self.state = LoopState::Stopped;
    }

    /// 获取循环状态
    pub fn state(&self) -> &LoopState {
        &self.state
    }

    /// 发送事件
    pub async fn emit_event(&self, event: YuanEvent) {
        let sess = self.session.read().await;
        let event_bus = sess.event_bus();
        let mut bus = event_bus.write().await;
        bus.emit(event);
    }

    /// 获取事件订阅者
    pub async fn subscribe(&self) -> tokio::sync::broadcast::Receiver<YuanEvent> {
        let sess = self.session.read().await;
        let event_bus = sess.event_bus();
        let bus = event_bus.read().await;
        bus.subscribe()
    }
}