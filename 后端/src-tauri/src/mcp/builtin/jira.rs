//! D1 v3.1 Task 3.2.7: Jira MCP 服务器
//!
//! 通过 Jira REST API v3 操作 Issue。
//! 通过 configure({"base_url": "...", "email": "...", "api_token": "..."}) 注入，
//! 使用 Basic Auth（email:api_token）。

use async_trait::async_trait;
use base64::Engine;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::mcp::sdk::{error_result, json_result, make_tool, BuiltinMcpServer};
use crate::models::mcp::{Tool, ToolCallResult};

pub struct JiraMcpServer {
    base_url: RwLock<Option<String>>,
    basic_auth: RwLock<Option<String>>,
}

impl JiraMcpServer {
    pub fn new() -> Self {
        Self {
            base_url: RwLock::new(None),
            basic_auth: RwLock::new(None),
        }
    }

    async fn client(&self) -> Result<(reqwest::Client, String), AppError> {
        let auth = self.basic_auth.read().await.clone();
        let base_url = self
            .base_url
            .read()
            .await
            .clone()
            .ok_or_else(|| AppError::Validation("Jira base_url 未配置".into()))?;
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        if let Some(a) = auth {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Basic {}", a))
                    .map_err(|e| AppError::Internal(format!("无效 auth: {}", e)))?,
            );
        }
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| AppError::Internal(format!("构建 HTTP 客户端失败: {}", e)))?;
        Ok((client, base_url))
    }
}

#[async_trait]
impl BuiltinMcpServer for JiraMcpServer {
    fn name(&self) -> &str {
        "jira"
    }

    fn description(&self) -> &str {
        "Jira MCP — 操作 Jira Issue（get_issue/create_issue/search_issues）"
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            make_tool(
                "get_issue",
                "获取指定 Issue 详情",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "issue_key": {"type": "string", "description": "如 PROJ-123"}
                    },
                    "required": ["issue_key"]
                }),
            ),
            make_tool(
                "create_issue",
                "创建 Issue",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project": {"type": "string"},
                        "summary": {"type": "string"},
                        "description": {"type": "string"},
                        "issue_type": {"type": "string", "default": "Task"}
                    },
                    "required": ["project", "summary"]
                }),
            ),
            make_tool(
                "search_issues",
                "通过 JQL 搜索 Issue",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "jql": {"type": "string", "description": "JQL 查询语句"},
                        "max_results": {"type": "integer", "default": 20}
                    },
                    "required": ["jql"]
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
        if self.basic_auth.read().await.is_none() {
            return Ok(error_result(
                "Jira 凭据未配置，请先调用 configure({base_url, email, api_token})",
            ));
        }
        let (client, base_url) = self.client().await?;
        match tool_name {
            "get_issue" => {
                let issue_key = args
                    .get("issue_key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 issue_key".into()))?;
                let url = format!("{}/rest/api/3/issue/{}", base_url, issue_key);
                let resp = client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("Jira API 请求失败: {}", e)))?;
                let status = resp.status();
                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| AppError::Internal(format!("解析响应失败: {}", e)))?;
                if !status.is_success() {
                    return Ok(error_result(&format!("Jira API 返回 {}", status)));
                }
                Ok(json_result(&serde_json::json!({
                    "key": body.get("key"),
                    "summary": body
                        .get("fields")
                        .and_then(|f| f.get("summary")),
                    "status": body
                        .get("fields")
                        .and_then(|f| f.get("status"))
                        .and_then(|s| s.get("name")),
                })))
            }
            "create_issue" => {
                let project = args
                    .get("project")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 project".into()))?;
                let summary = args
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 summary".into()))?;
                let description = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let issue_type = args
                    .get("issue_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Task");
                let url = format!("{}/rest/api/3/issue", base_url);
                let payload = serde_json::json!({
                    "fields": {
                        "project": {"key": project},
                        "summary": summary,
                        "description": description,
                        "issuetype": {"name": issue_type}
                    }
                });
                let resp = client
                    .post(&url)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("Jira API 请求失败: {}", e)))?;
                let status = resp.status();
                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| AppError::Internal(format!("解析响应失败: {}", e)))?;
                if !status.is_success() {
                    return Ok(error_result(&format!("Jira API 返回 {}", status)));
                }
                Ok(json_result(&serde_json::json!({
                    "key": body.get("key"),
                    "id": body.get("id"),
                    "self": body.get("self")
                })))
            }
            "search_issues" => {
                let jql = args
                    .get("jql")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 jql".into()))?;
                let max_results = args
                    .get("max_results")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(20);
                let url = format!("{}/rest/api/3/search", base_url);
                let resp = client
                    .get(&url)
                    .query(&[
                        ("jql", jql),
                        ("maxResults", &max_results.to_string()),
                    ])
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("Jira API 请求失败: {}", e)))?;
                let status = resp.status();
                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| AppError::Internal(format!("解析响应失败: {}", e)))?;
                if !status.is_success() {
                    return Ok(error_result(&format!("Jira API 返回 {}", status)));
                }
                let summary: Vec<serde_json::Value> = body
                    .get("issues")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|i| {
                                serde_json::json!({
                                    "key": i.get("key"),
                                    "summary": i
                                        .get("fields")
                                        .and_then(|f| f.get("summary")),
                                    "status": i
                                        .get("fields")
                                        .and_then(|f| f.get("status"))
                                        .and_then(|s| s.get("name")),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(json_result(&serde_json::json!({
                    "issues": summary,
                    "total": body.get("total"),
                    "count": summary.len()
                })))
            }
            _ => Ok(error_result(&format!("未知工具: {}", tool_name))),
        }
    }

    async fn configure(&self, config: serde_json::Value) -> Result<(), AppError> {
        let base_url = config
            .get("base_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Validation("configure 缺少 base_url".into()))?;
        let email = config
            .get("email")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Validation("configure 缺少 email".into()))?;
        let api_token = config
            .get("api_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Validation("configure 缺少 api_token".into()))?;
        let credentials = format!("{}:{}", email, api_token);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        *self.base_url.write().await = Some(base_url.trim_end_matches('/').to_string());
        *self.basic_auth.write().await = Some(encoded);
        Ok(())
    }
}
