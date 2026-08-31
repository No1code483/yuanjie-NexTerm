//! D1 v3.1 Task 3.2.2-3.2.9: 8 个内置 MCP 服务器
//!
//! 每个内置服务器以进程内 trait 对象实现 BuiltinMcpServer，无需 spawn 子进程。
//! API Key 通过 configure() 注入（由云端 API Key 系统解密后传入明文，仅内存驻留）。
//!
//! 设计约束（项目核心设计意图）：
//! - 这些 MCP 服务器用于 Yuan Code 编程辅助（操作仓库/查询数据库/搜索网页等），
//!   本身不进行 AI 代码生成，AI 代码生成走云端 API 模型。
//! - 外部 API 调用（GitHub/GitLab/Slack/Jira）使用 reqwest，非阻塞 async。

pub mod github;
pub mod gitlab;
pub mod jira;
pub mod memory;
pub mod postgres;
pub mod slack;
pub mod sqlite;
pub mod websearch;

use std::sync::Arc;

use crate::mcp::sdk::BuiltinRegistry;

/// 注册全部 8 个内置 MCP 服务器到注册表
pub async fn register_all(registry: &BuiltinRegistry) {
    registry.register(Arc::new(github::GithubMcpServer::new())).await;
    registry.register(Arc::new(gitlab::GitlabMcpServer::new())).await;
    registry.register(Arc::new(sqlite::SqliteMcpServer::new())).await;
    registry.register(Arc::new(postgres::PostgresMcpServer::new())).await;
    registry.register(Arc::new(slack::SlackMcpServer::new())).await;
    registry.register(Arc::new(jira::JiraMcpServer::new())).await;
    registry.register(Arc::new(memory::MemoryMcpServer::new())).await;
    registry.register(Arc::new(websearch::WebSearchMcpServer::new())).await;
}
