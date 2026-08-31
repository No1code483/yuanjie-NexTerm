//! D1 v3.1 Task 3.2.2: GitHub MCP 服务器
//!
//! 通过 GitHub REST API v3 操作仓库（PR / Issue / 分支）。
//! API Token 通过 configure({"token": "..."}) 注入，仅内存驻留。

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::mcp::sdk::{error_result, json_result, make_tool, BuiltinMcpServer};
use crate::models::mcp::{Tool, ToolCallResult};

pub struct GithubMcpServer {
    token: RwLock<Option<String>>,
}

impl GithubMcpServer {
    pub fn new() -> Self {
        Self {
            token: RwLock::new(None),
        }
    }

    async fn client(&self) -> Result<reqwest::Client, AppError> {
        let token = self.token.read().await.clone();
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("NexTerm-YuanCode-MCP"),
        );
        if let Some(t) = token {
            let auth = format!("Bearer {}", t);
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&auth)
                    .map_err(|e| AppError::Internal(format!("无效 token: {}", e)))?,
            );
        }
        reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| AppError::Internal(format!("构建 HTTP 客户端失败: {}", e)))
    }
}

#[async_trait]
impl BuiltinMcpServer for GithubMcpServer {
    fn name(&self) -> &str {
        "github"
    }

    fn description(&self) -> &str {
        "GitHub MCP — 操作仓库 PR/Issue/分支（GitHub REST API v3）"
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            make_tool(
                "list_pull_requests",
                "列出指定仓库的 Pull Request",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "owner": {"type": "string", "description": "仓库所有者"},
                        "repo": {"type": "string", "description": "仓库名"},
                        "state": {"type": "string", "description": "PR 状态", "default": "open"}
                    },
                    "required": ["owner", "repo"]
                }),
            ),
            make_tool(
                "create_issue",
                "在指定仓库创建 Issue",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "owner": {"type": "string"},
                        "repo": {"type": "string"},
                        "title": {"type": "string"},
                        "body": {"type": "string"}
                    },
                    "required": ["owner", "repo", "title"]
                }),
            ),
            make_tool(
                "list_branches",
                "列出指定仓库的分支",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "owner": {"type": "string"},
                        "repo": {"type": "string"}
                    },
                    "required": ["owner", "repo"]
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
        let token = self.token.read().await.clone();
        if token.is_none() {
            return Ok(error_result(
                "GitHub API token 未配置，请先调用 configure 注入 token",
            ));
        }
        let client = self.client().await?;
        match tool_name {
            "list_pull_requests" => {
                let owner = args
                    .get("owner")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 owner".into()))?;
                let repo = args
                    .get("repo")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 repo".into()))?;
                let state = args
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("open");
                let url = format!(
                    "https://api.github.com/repos/{}/{}/pulls?state={}",
                    owner, repo, state
                );
                let resp = client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("GitHub API 请求失败: {}", e)))?;
                let status = resp.status();
                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| AppError::Internal(format!("解析响应失败: {}", e)))?;
                if !status.is_success() {
                    return Ok(error_result(&format!(
                        "GitHub API 返回 {}: {}",
                        status,
                        body.get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("未知错误")
                    )));
                }
                // 精简输出
                let summary: Vec<serde_json::Value> = body
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .map(|pr| {
                                serde_json::json!({
                                    "number": pr.get("number"),
                                    "title": pr.get("title"),
                                    "state": pr.get("state"),
                                    "user": pr.get("user").and_then(|u| u.get("login")),
                                    "html_url": pr.get("html_url"),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(json_result(&serde_json::json!({
                    "total": summary.len(),
                    "pulls": summary
                })))
            }
            "create_issue" => {
                let owner = args
                    .get("owner")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 owner".into()))?;
                let repo = args
                    .get("repo")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 repo".into()))?;
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 title".into()))?;
                let body_text = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
                let url = format!("https://api.github.com/repos/{}/{}/issues", owner, repo);
                let resp = client
                    .post(&url)
                    .json(&serde_json::json!({"title": title, "body": body_text}))
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("GitHub API 请求失败: {}", e)))?;
                let status = resp.status();
                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| AppError::Internal(format!("解析响应失败: {}", e)))?;
                if !status.is_success() {
                    return Ok(error_result(&format!(
                        "GitHub API 返回 {}: {}",
                        status,
                        body.get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("未知错误")
                    )));
                }
                Ok(json_result(&serde_json::json!({
                    "number": body.get("number"),
                    "html_url": body.get("html_url"),
                    "title": body.get("title")
                })))
            }
            "list_branches" => {
                let owner = args
                    .get("owner")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 owner".into()))?;
                let repo = args
                    .get("repo")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 repo".into()))?;
                let url = format!("https://api.github.com/repos/{}/{}/branches", owner, repo);
                let resp = client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("GitHub API 请求失败: {}", e)))?;
                let status = resp.status();
                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| AppError::Internal(format!("解析响应失败: {}", e)))?;
                if !status.is_success() {
                    return Ok(error_result(&format!(
                        "GitHub API 返回 {}: {}",
                        status,
                        body.get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("未知错误")
                    )));
                }
                let summary: Vec<serde_json::Value> = body
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .map(|b| {
                                serde_json::json!({
                                    "name": b.get("name"),
                                    "protected": b.get("protected"),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(json_result(&serde_json::json!({
                    "total": summary.len(),
                    "branches": summary
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
