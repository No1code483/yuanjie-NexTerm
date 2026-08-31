//! D1 v3.1 Task 3.2.3: GitLab MCP 服务器
//!
//! 通过 GitLab REST API v4 操作仓库（项目 / MR / Issue）。
//! API Token 通过 configure({"token": "...", "api_url": "..."}) 注入。

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::mcp::sdk::{error_result, json_result, make_tool, BuiltinMcpServer};
use crate::models::mcp::{Tool, ToolCallResult};

pub struct GitlabMcpServer {
    token: RwLock<Option<String>>,
    api_url: RwLock<String>,
}

impl GitlabMcpServer {
    pub fn new() -> Self {
        Self {
            token: RwLock::new(None),
            api_url: RwLock::new("https://gitlab.com/api/v4".into()),
        }
    }

    async fn client(&self) -> Result<(reqwest::Client, String), AppError> {
        let token = self.token.read().await.clone();
        if token.is_none() {
            return Ok((reqwest::Client::new(), self.api_url.read().await.clone()));
        }
        let token = token.unwrap();
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))
                .map_err(|e| AppError::Internal(format!("无效 token: {}", e)))?,
        );
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| AppError::Internal(format!("构建 HTTP 客户端失败: {}", e)))?;
        Ok((client, self.api_url.read().await.clone()))
    }
}

#[async_trait]
impl BuiltinMcpServer for GitlabMcpServer {
    fn name(&self) -> &str {
        "gitlab"
    }

    fn description(&self) -> &str {
        "GitLab MCP — 操作 GitLab 仓库项目/MR/Issue（GitLab REST API v4）"
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            make_tool(
                "list_projects",
                "列出当前用户可访问的 GitLab 项目",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "search": {"type": "string", "description": "项目名搜索关键词"},
                        "per_page": {"type": "integer", "default": 20}
                    }
                }),
            ),
            make_tool(
                "list_merge_requests",
                "列出指定项目的 Merge Request",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_id": {"type": "integer", "description": "项目 ID"},
                        "state": {"type": "string", "default": "opened"}
                    },
                    "required": ["project_id"]
                }),
            ),
            make_tool(
                "create_issue",
                "在指定 GitLab 项目创建 Issue",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_id": {"type": "integer"},
                        "title": {"type": "string"},
                        "description": {"type": "string"}
                    },
                    "required": ["project_id", "title"]
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
            return Ok(error_result("GitLab API token 未配置，请先调用 configure 注入 token"));
        }
        let (client, api_url) = self.client().await?;
        match tool_name {
            "list_projects" => {
                let search = args.get("search").and_then(|v| v.as_str()).unwrap_or("");
                let per_page = args
                    .get("per_page")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(20);
                let mut url = format!("{}/projects?per_page={}", api_url, per_page);
                if !search.is_empty() {
                    url.push_str(&format!("&search={}", search));
                }
                let resp = client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("GitLab API 请求失败: {}", e)))?;
                let status = resp.status();
                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| AppError::Internal(format!("解析响应失败: {}", e)))?;
                if !status.is_success() {
                    return Ok(error_result(&format!("GitLab API 返回 {}", status)));
                }
                let summary: Vec<serde_json::Value> = body
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .map(|p| {
                                serde_json::json!({
                                    "id": p.get("id"),
                                    "name": p.get("name"),
                                    "path_with_namespace": p.get("path_with_namespace"),
                                    "web_url": p.get("web_url"),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(json_result(&serde_json::json!({
                    "total": summary.len(),
                    "projects": summary
                })))
            }
            "list_merge_requests" => {
                let project_id = args
                    .get("project_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| AppError::Validation("缺少 project_id".into()))?;
                let state = args
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("opened");
                let url = format!(
                    "{}/projects/{}/merge_requests?state={}",
                    api_url, project_id, state
                );
                let resp = client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("GitLab API 请求失败: {}", e)))?;
                let status = resp.status();
                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| AppError::Internal(format!("解析响应失败: {}", e)))?;
                if !status.is_success() {
                    return Ok(error_result(&format!("GitLab API 返回 {}", status)));
                }
                let summary: Vec<serde_json::Value> = body
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .map(|mr| {
                                serde_json::json!({
                                    "iid": mr.get("iid"),
                                    "title": mr.get("title"),
                                    "state": mr.get("state"),
                                    "author": mr.get("author").and_then(|a| a.get("username")),
                                    "web_url": mr.get("web_url"),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(json_result(&serde_json::json!({
                    "total": summary.len(),
                    "merge_requests": summary
                })))
            }
            "create_issue" => {
                let project_id = args
                    .get("project_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| AppError::Validation("缺少 project_id".into()))?;
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 title".into()))?;
                let description = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let url = format!("{}/projects/{}/issues", api_url, project_id);
                let resp = client
                    .post(&url)
                    .json(&serde_json::json!({"title": title, "description": description}))
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("GitLab API 请求失败: {}", e)))?;
                let status = resp.status();
                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| AppError::Internal(format!("解析响应失败: {}", e)))?;
                if !status.is_success() {
                    return Ok(error_result(&format!("GitLab API 返回 {}", status)));
                }
                Ok(json_result(&serde_json::json!({
                    "iid": body.get("iid"),
                    "web_url": body.get("web_url"),
                    "title": body.get("title")
                })))
            }
            _ => Ok(error_result(&format!("未知工具: {}", tool_name))),
        }
    }

    async fn configure(&self, config: serde_json::Value) -> Result<(), AppError> {
        if let Some(token) = config.get("token").and_then(|v| v.as_str()) {
            *self.token.write().await = Some(token.to_string());
        }
        if let Some(api_url) = config.get("api_url").and_then(|v| v.as_str()) {
            *self.api_url.write().await = api_url.to_string();
        }
        Ok(())
    }
}
