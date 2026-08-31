// SSE 连接管理 — 对标 Codex-rs 的 SSE 流式连接
// 管理与 AI 模型提供商之间的 Server-Sent Events 流式连接

use std::time::Duration;
use reqwest::Client;
use tokio::sync::mpsc;

use super::model::ModelRequest;
use super::model_retry::RetryPolicy;

/// SSE 连接状态
#[derive(Debug, Clone, PartialEq)]
pub enum SseConnectionState {
    /// 未连接
    Disconnected,
    /// 连接中
    Connecting,
    /// 已连接
    Connected,
    /// 出错
    Error(String),
    /// 已关闭
    Closed,
}

/// SSE 流式事件
#[derive(Debug, Clone)]
pub enum SseStreamEvent {
    /// 文本增量
    Delta {
        content: String,
        sequence: u64,
    },
    /// 工具调用增量
    ToolCallDelta {
        tool_index: usize,
        tool_id: Option<String>,
        function_name: Option<String>,
        function_arguments: Option<String>,
    },
    /// 流结束
    Done {
        finish_reason: String,
        usage: Option<SseTokenUsage>,
    },
    /// 连接错误
    Error(String),
}

/// SSE Token 用量
#[derive(Debug, Clone)]
pub struct SseTokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

/// SSE 连接管理器
pub struct SseConnectionManager {
    /// HTTP 客户端
    client: Client,
    /// 当前连接状态
    state: SseConnectionState,
    /// 重试策略
    _retry_policy: RetryPolicy,
    /// 连接超时
    connect_timeout: Duration,
    /// 读取超时
    read_timeout: Duration,
    /// 持活间隔
    _keepalive_interval: Duration,
}

impl SseConnectionManager {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            state: SseConnectionState::Disconnected,
            _retry_policy: RetryPolicy::default(),
            connect_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(120),
            _keepalive_interval: Duration::from_secs(30),
        }
    }

    /// 设置连接超时
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// 设置读取超时
    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = timeout;
        self
    }

    /// 获取连接状态
    pub fn state(&self) -> &SseConnectionState {
        &self.state
    }

    /// 发送流式请求到 OpenAI 兼容 API
    /// 返回一个 mpsc 接收端用于接收流式事件
    pub async fn stream_openai(
        &mut self,
        api_base: &str,
        api_key: &str,
        request: &ModelRequest,
    ) -> Result<mpsc::Receiver<SseStreamEvent>, String> {
        self.state = SseConnectionState::Connecting;

        let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));
        let (tx, rx) = mpsc::channel::<SseStreamEvent>(256);

        let request_body = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "top_p": request.top_p,
            "stream": true,
            "stream_options": {
                "include_usage": true
            }
        });

        let client = self.client.clone();
        let api_key = api_key.to_string();
        let connect_timeout = self.connect_timeout;
        let read_timeout = self.read_timeout;

        // 后台任务处理流式响应
        tokio::spawn(async move {
            let response = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .header("Accept", "text/event-stream")
                .timeout(connect_timeout)
                .json(&request_body)
                .send()
                .await;

            let response = match response {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let status = resp.status().as_u16();
                        let body = resp.text().await.unwrap_or_default();
                        let _ = tx
                            .send(SseStreamEvent::Error(format!(
                                "HTTP {}: {}",
                                status, body
                            )))
                            .await;
                        return;
                    }
                    resp
                }
                Err(e) => {
                    let _ = tx
                        .send(SseStreamEvent::Error(format!("连接失败: {}", e)))
                        .await;
                    return;
                }
            };

            // 使用 tokio::time::timeout 包装整个读取过程
            let read_result = tokio::time::timeout(read_timeout, async {
                use futures::StreamExt;
                let mut stream = response.bytes_stream();
                let mut buffer = String::new();
                let mut sequence: u64 = 0;

                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(bytes) => {
                            let text = String::from_utf8_lossy(&bytes);
                            buffer.push_str(&text);

                            // 按行解析 SSE 数据
                            while let Some(line_end) = buffer.find('\n') {
                                let line = buffer[..line_end].trim().to_string();
                                buffer = buffer[line_end + 1..].to_string();

                                if line.is_empty() {
                                    continue;
                                }

                                if line == "data: [DONE]" {
                                    let _ = tx
                                        .send(SseStreamEvent::Done {
                                            finish_reason: "stop".into(),
                                            usage: None,
                                        })
                                        .await;
                                    return;
                                }

                                if let Some(data) = line.strip_prefix("data: ") {
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                        // 处理 usage 信息
                                        if let Some(usage) = json.get("usage") {
                                            let _ = tx
                                                .send(SseStreamEvent::Done {
                                                    finish_reason: "stop".into(),
                                                    usage: Some(SseTokenUsage {
                                                        input_tokens: usage["input_tokens"]
                                                            .as_u64()
                                                            .unwrap_or(0),
                                                        output_tokens: usage["output_tokens"]
                                                            .as_u64()
                                                            .unwrap_or(0),
                                                        total_tokens: usage["total_tokens"]
                                                            .as_u64()
                                                            .unwrap_or(0),
                                                    }),
                                                })
                                                .await;
                                            return;
                                        }

                                        // 处理 delta 内容
                                        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                                            if let Some(choice) = choices.first() {
                                                if let Some(delta) = choice.get("delta") {
                                                    // 文本增量
                                                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                                        if !content.is_empty() {
                                                            sequence += 1;
                                                            let _ = tx
                                                                .send(SseStreamEvent::Delta {
                                                                    content: content.to_string(),
                                                                    sequence,
                                                                })
                                                                .await;
                                                        }
                                                    }

                                                    // 工具调用增量
                                                    if let Some(tool_calls) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                                                        for tc in tool_calls {
                                                            let index = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                                                            let id = tc.get("id").and_then(|i| i.as_str()).map(|s| s.to_string());
                                                            let func = tc.get("function");
                                                            let func_name = func.and_then(|f| f.get("name")).and_then(|n| n.as_str()).map(|s| s.to_string());
                                                            let func_args = func.and_then(|f| f.get("arguments")).and_then(|a| a.as_str()).map(|s| s.to_string());

                                                            let _ = tx
                                                                .send(SseStreamEvent::ToolCallDelta {
                                                                    tool_index: index,
                                                                    tool_id: id,
                                                                    function_name: func_name,
                                                                    function_arguments: func_args,
                                                                })
                                                                .await;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let _ = tx
                                .send(SseStreamEvent::Error(format!("流读取错误: {}", e)))
                                .await;
                            return;
                        }
                    }
                }
            })
            .await;

            if read_result.is_err() {
                let _ = tx
                    .send(SseStreamEvent::Error("读取超时".into()))
                    .await;
            }
        });

        self.state = SseConnectionState::Connected;
        Ok(rx)
    }

    /// 发送流式请求到 Anthropic 兼容 API
    pub async fn stream_anthropic(
        &mut self,
        api_base: &str,
        api_key: &str,
        request: &ModelRequest,
    ) -> Result<mpsc::Receiver<SseStreamEvent>, String> {
        self.state = SseConnectionState::Connecting;

        let url = format!("{}/messages", api_base.trim_end_matches('/'));
        let (tx, rx) = mpsc::channel::<SseStreamEvent>(256);

        let request_body = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "top_p": request.top_p,
            "stream": true,
        });

        let client = self.client.clone();
        let api_key = api_key.to_string();
        let connect_timeout = self.connect_timeout;
        let read_timeout = self.read_timeout;

        tokio::spawn(async move {
            let response = client
                .post(&url)
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .timeout(connect_timeout)
                .json(&request_body)
                .send()
                .await;

            let response = match response {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let status = resp.status().as_u16();
                        let body = resp.text().await.unwrap_or_default();
                        let _ = tx
                            .send(SseStreamEvent::Error(format!(
                                "HTTP {}: {}",
                                status, body
                            )))
                            .await;
                        return;
                    }
                    resp
                }
                Err(e) => {
                    let _ = tx
                        .send(SseStreamEvent::Error(format!("连接失败: {}", e)))
                        .await;
                    return;
                }
            };

            let read_result = tokio::time::timeout(read_timeout, async {
                use futures::StreamExt;
                let mut stream = response.bytes_stream();
                let mut buffer = String::new();
                let mut sequence: u64 = 0;

                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(bytes) => {
                            let text = String::from_utf8_lossy(&bytes);
                            buffer.push_str(&text);

                            while let Some(line_end) = buffer.find('\n') {
                                let line = buffer[..line_end].trim().to_string();
                                buffer = buffer[line_end + 1..].to_string();

                                if line.is_empty() {
                                    continue;
                                }

                                if let Some(data) = line.strip_prefix("data: ") {
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                        let event_type = json["type"].as_str().unwrap_or("");

                                        match event_type {
                                            "content_block_start" => {}
                                            "content_block_delta" => {
                                                if let Some(delta) = json["delta"].as_object() {
                                                    // 文本增量
                                                    if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                                        sequence += 1;
                                                        let _ = tx
                                                            .send(SseStreamEvent::Delta {
                                                                content: text.to_string(),
                                                                sequence,
                                                            })
                                                            .await;
                                                    }
                                                }
                                            }
                                            "message_stop" => {
                                                let _ = tx
                                                    .send(SseStreamEvent::Done {
                                                        finish_reason: "stop".into(),
                                                        usage: None,
                                                    })
                                                    .await;
                                                return;
                                            }
                                            "error" => {
                                                let _ = tx
                                                    .send(SseStreamEvent::Error(
                                                        json["error"]["message"]
                                                            .as_str()
                                                            .unwrap_or("未知错误")
                                                            .to_string(),
                                                    ))
                                                    .await;
                                                return;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let _ = tx
                                .send(SseStreamEvent::Error(format!("流读取错误: {}", e)))
                                .await;
                            return;
                        }
                    }
                }
            })
            .await;

            if read_result.is_err() {
                let _ = tx
                    .send(SseStreamEvent::Error("读取超时".into()))
                    .await;
            }
        });

        self.state = SseConnectionState::Connected;
        Ok(rx)
    }

    /// 关闭连接
    pub fn close(&mut self) {
        self.state = SseConnectionState::Closed;
    }
}

impl Default for SseConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}