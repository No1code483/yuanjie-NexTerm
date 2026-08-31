//! D1 v3.1 Task 3.2.6: Slack MCP 服务器
//!
//! 通过 Slack Web API 发送/读取消息。
//! API Token（Bot User OAuth Token, xoxb-）通过 configure({"token": "..."}) 注入。

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::mcp::sdk::{error_result, json_result, make_tool, BuiltinMcpServer};
use crate::models::mcp::{Tool, ToolCallResult};

pub struct SlackMcpServer {
    token: RwLock<Option<String>>,
}

impl SlackMcpServer {
    pub fn new() -> Self {
        Self {
            token: RwLock::new(None),
        }
    }

    async fn client(&self) -> Result<reqwest::Client, AppError> {
        let token = self.token.read().await.clone();
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!(
                "Bearer {}",
                token.unwrap_or_default()
            ))
            .map_err(|e| AppError::Internal(format!("无效 token: {}", e)))?,
        );
        reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| AppError::Internal(format!("构建 HTTP 客户端失败: {}", e)))
    }
}

#[async_trait]
impl BuiltinMcpServer for SlackMcpServer {
    fn name(&self) -> &str {
        "slack"
    }

    fn description(&self) -> &str {
        "Slack MCP — 发送/读取 Slack 消息（Slack Web API）"
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            make_tool(
                "post_message",
                "向指定频道发送消息",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "channel": {"type": "string", "description": "频道 ID 或名称"},
                        "text": {"type": "string"}
                    },
                    "required": ["channel", "text"]
                }),
            ),
            make_tool(
                "list_channels",
                "列出可访问的频道（conversations.list）",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {"type": "integer", "default": 50}
                    }
                }),
            ),
            make_tool(
                "get_history",
                "读取指定频道的历史消息（conversations.history）",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "channel": {"type": "string"},
                        "limit": {"type": "integer", "default": 20}
                    },
                    "required": ["channel"]
                }),
            ),
        ]
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        args: Option<serde_json::Value>,
    ) -> Result<ToolCallResult, AppError> {
        let args = args.unwrap_or(serde_json::json!({}));
        if self.token.read().await.is_none() {
            return Ok(error_result("Slack API token 未配置，请先调用 configure 注入 token"));
        }
        let client = self.client().await?;
        match tool_name {
            "post_message" => {
                let channel = args
                    .get("channel")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 channel".into()))?;
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 text".into()))?;
                let resp = client
                    .post("https://slack.com/api/chat.postMessage")
                    .json(&serde_json::json!({"channel": channel, "text": text}))
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("Slack API 请求失败: {}", e)))?;
                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| AppError::Internal(format!("解析响应失败: {}", e)))?;
                if !body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Ok(error_result(&format!(
                        "Slack API 错误: {}",
                        body.get("error")
                            .and_then(|v| v.as_str())
                            .unwrap_or("未知错误")
                    )));
                }
                Ok(json_result(&serde_json::json!({
                    "ok": true,
                    "channel": body.get("channel"),
                    "ts": body.get("ts")
                })))
            }
            "list_channels" => {
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(50);
                let resp = client
                    .get("https://slack.com/api/conversations.list")
                    .query(&[("limit", &limit.to_string())])
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("Slack API 请求失败: {}", e)))?;
                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| AppError::Internal(format!("解析响应失败: {}", e)))?;
                if !body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Ok(error_result(&format!(
                        "Slack API 错误: {}",
                        body.get("error")
                            .and_then(|v| v.as_str())
                            .unwrap_or("未知错误")
                    )));
                }
                let summary: Vec<serde_json::Value> = body
                    .get("channels")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|c| {
                                serde_json::json!({
                                    "id": c.get("id"),
                                    "name": c.get("name"),
                                    "is_channel": c.get("is_channel"),
                                    "num_members": c.get("num_members"),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(json_result(&serde_json::json!({
                    "channels": summary,
                    "count": summary.len()
                })))
            }
            "get_history" => {
                let channel = args
                    .get("channel")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 channel".into()))?;
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(20);
                let resp = client
                    .get("https://slack.com/api/conversations.history")
                    .query(&[
                        ("channel", channel),
                        ("limit", &limit.to_string()),
                    ])
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("Slack API 请求失败: {}", e)))?;
                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| AppError::Internal(format!("解析响应失败: {}", e)))?;
                if !body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Ok(error_result(&format!(
                        "Slack API 错误: {}",
                        body.get("error")
                            .and_then(|v| v.as_str())
                            .unwrap_or("未知错误")
                    )));
                }
                let summary: Vec<serde_json::Value> = body
                    .get("messages")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|m| {
                                serde_json::json!({
                                    "user": m.get("user"),
                                    "text": m.get("text"),
                                    "ts": m.get("ts"),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(json_result(&serde_json::json!({
                    "messages": summary,
                    "count": summary.len()
                })))
            }
            _ => Ok(error_result(&format!("未知工具: {}", tool_name))),
        }
    }

    async fn configure(&self, config: serde_json::Value) -> Result<(), AppError> {
        if let Some(token) = config.get("token").and_then(|v| v.as_str()) {
            *self.token.write().await = Some(token.to_string());
            Ok(())
        } else {
            Err(AppError::Validation("configure 缺少 token 字段".into()))
        }
    }
}
