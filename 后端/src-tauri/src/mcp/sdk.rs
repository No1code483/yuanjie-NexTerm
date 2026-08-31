//! D1 v3.1 Task 3.2.1: MCP SDK — 内置 MCP 服务器框架
//!
//! 提供：
//! - JSON-RPC 2.0 协议解析辅助
//! - BuiltinMcpServer trait（内置服务器统一抽象）
//! - BuiltinRegistry（内置服务器注册表，按命名空间索引）
//! - 工具/结果构造辅助函数
//!
//! 设计依据：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md Phase 4
//! 与现有 client.rs（外部进程 MCP 客户端）互补：内置服务器以进程内 trait 对象实现，
//! 无需 spawn 子进程，调用延迟 < 500ms（Task 3.2.12 非阻塞架构保证）。

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::models::mcp::{Tool, ToolCallResult, ToolContent};

/// 内置 MCP 服务器统一抽象
///
/// 每个内置服务器实现此 trait，提供命名空间、工具列表与工具调用入口。
/// API Key / 连接字符串等敏感配置通过 `configure` 注入（由云端 API Key 系统解密后传入）。
#[async_trait]
pub trait BuiltinMcpServer: Send + Sync {
    /// 服务器名称（亦为命名空间，如 "github" / "sqlite"）
    fn name(&self) -> &str;
    /// 服务器描述
    fn description(&self) -> &str;
    /// 列出工具定义
    fn tools(&self) -> Vec<Tool>;
    /// 调用工具
    async fn call_tool(
        &self,
        tool_name: &str,
        args: Option<serde_json::Value>,
    ) -> Result<ToolCallResult, AppError>;
    /// 配置服务器（注入 API Key / 连接字符串等），默认空实现
    async fn configure(&self, _config: serde_json::Value) -> Result<(), AppError> {
        Ok(())
    }
}

/// 内置 MCP 服务器注册表
pub struct BuiltinRegistry {
    servers: RwLock<HashMap<String, Arc<dyn BuiltinMcpServer>>>,
}

impl BuiltinRegistry {
    pub fn new() -> Self {
        Self {
            servers: RwLock::new(HashMap::new()),
        }
    }

    /// 注册内置服务器
    pub async fn register(&self, server: Arc<dyn BuiltinMcpServer>) {
        let name = server.name().to_string();
        self.servers.write().await.insert(name, server);
    }

    /// 是否存在指定命名空间的服务器
    pub async fn has_server(&self, name: &str) -> bool {
        self.servers.read().await.contains_key(name)
    }

    /// 列出所有内置服务器名称
    pub async fn list_servers(&self) -> Vec<String> {
        self.servers.read().await.keys().cloned().collect()
    }

    /// 列出所有内置服务器的工具（返回 (server_name, tool) 列表）
    pub async fn list_all_tools(&self) -> Vec<(String, Tool)> {
        let mut out = vec![];
        for (name, server) in self.servers.read().await.iter() {
            for tool in server.tools() {
                out.push((name.clone(), tool));
            }
        }
        out
    }

    /// 调用内置服务器工具
    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        args: Option<serde_json::Value>,
    ) -> Result<ToolCallResult, AppError> {
        let server = {
            let servers = self.servers.read().await;
            servers
                .get(server_name)
                .cloned()
                .ok_or(AppError::NotFound)?
        };
        server.call_tool(tool_name, args).await
    }

    /// 配置指定内置服务器
    pub async fn configure(
        &self,
        server_name: &str,
        config: serde_json::Value,
    ) -> Result<(), AppError> {
        let server = {
            let servers = self.servers.read().await;
            servers
                .get(server_name)
                .cloned()
                .ok_or(AppError::NotFound)?
        };
        server.configure(config).await
    }
}

// ===== 工具/结果构造辅助函数 =====

/// 构造工具定义
pub fn make_tool(name: &str, description: &str, schema: serde_json::Value) -> Tool {
    Tool {
        name: name.into(),
        description: description.into(),
        input_schema: schema,
    }
}

/// 构造文本结果
pub fn text_result(text: &str) -> ToolCallResult {
    ToolCallResult {
        content: vec![ToolContent {
            content_type: "text".into(),
            text: Some(text.into()),
            data: None,
            mime_type: None,
        }],
        is_error: Some(false),
    }
}

/// 构造错误结果（isError=true）
pub fn error_result(text: &str) -> ToolCallResult {
    ToolCallResult {
        content: vec![ToolContent {
            content_type: "text".into(),
            text: Some(text.into()),
            data: None,
            mime_type: None,
        }],
        is_error: Some(true),
    }
}

/// 构造 JSON 结果（序列化为 pretty JSON 文本）
pub fn json_result(value: &serde_json::Value) -> ToolCallResult {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    text_result(&text)
}

/// 从参数中提取字符串字段
pub fn arg_str(args: &serde_json::Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// 从参数中提取可选字符串字段（带默认值）
pub fn arg_str_or(args: &serde_json::Value, key: &str, default: &str) -> String {
    arg_str(args, key).unwrap_or_else(|| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoServer;

    #[async_trait]
    impl BuiltinMcpServer for EchoServer {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "echo test server"
        }
        fn tools(&self) -> Vec<Tool> {
            vec![make_tool(
                "echo",
                "echo back",
                serde_json::json!({"type":"object","properties":{"msg":{"type":"string"}}}),
            )]
        }
        async fn call_tool(
            &self,
            tool_name: &str,
            args: Option<serde_json::Value>,
        ) -> Result<ToolCallResult, AppError> {
            if tool_name != "echo" {
                return Ok(error_result("未知工具"));
            }
            let msg = args
                .as_ref()
                .and_then(|a| a.get("msg"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Ok(text_result(&format!("echo: {}", msg)))
        }
    }

    #[tokio::test]
    async fn test_registry_register_and_call() {
        let reg = BuiltinRegistry::new();
        reg.register(Arc::new(EchoServer)).await;
        assert!(reg.has_server("echo").await);
        assert_eq!(reg.list_servers().await, vec!["echo"]);

        let tools = reg.list_all_tools().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].0, "echo");
        assert_eq!(tools[0].1.name, "echo");

        let res = reg
            .call_tool("echo", "echo", Some(serde_json::json!({"msg":"hi"})))
            .await
            .unwrap();
        assert_eq!(res.is_error, Some(false));
        assert_eq!(res.content[0].text.as_deref(), Some("echo: hi"));
    }

    #[test]
    fn test_helpers() {
        let t = make_tool("t", "d", serde_json::json!({}));
        assert_eq!(t.name, "t");
        assert_eq!(text_result("x").content[0].text.as_deref(), Some("x"));
        assert_eq!(error_result("e").is_error, Some(true));
        let j = json_result(&serde_json::json!({"a":1}));
        assert!(j.content[0].text.as_deref().unwrap().contains("\"a\""));
    }
}
