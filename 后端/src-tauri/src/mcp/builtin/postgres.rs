//! D1 v3.1 Task 3.2.5: Postgres MCP 服务器
//!
//! 骨架实现：工具 schema 与调用路径已就位，实际连接需在 Cargo.toml 中为 sqlx 启用
//! "postgres" feature 后接入真实 Postgres。当前返回明确的配置提示，保证可编译。
//!
//! 工具：list_tables / query / describe_table

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::mcp::sdk::{error_result, json_result, make_tool, BuiltinMcpServer};
use crate::models::mcp::{Tool, ToolCallResult};

pub struct PostgresMcpServer {
    connection_string: RwLock<Option<String>>,
}

impl PostgresMcpServer {
    pub fn new() -> Self {
        Self {
            connection_string: RwLock::new(None),
        }
    }
}

#[async_trait]
impl BuiltinMcpServer for PostgresMcpServer {
    fn name(&self) -> &str {
        "postgres"
    }

    fn description(&self) -> &str {
        "Postgres MCP — 查询 Postgres 数据库（list_tables/query/describe_table）"
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            make_tool(
                "list_tables",
                "列出 Postgres 数据库中的所有表",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "schema": {"type": "string", "default": "public"}
                    }
                }),
            ),
            make_tool(
                "query",
                "执行只读 SELECT 查询",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "sql": {"type": "string"},
                        "limit": {"type": "integer", "default": 100}
                    },
                    "required": ["sql"]
                }),
            ),
            make_tool(
                "describe_table",
                "描述指定表的结构（列名/类型/注释）",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "table_name": {"type": "string"},
                        "schema": {"type": "string", "default": "public"}
                    },
                    "required": ["table_name"]
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
        let conn = self.connection_string.read().await.clone();
        if conn.is_none() {
            return Ok(error_result(
                "Postgres connection_string 未配置，请先调用 configure({\"connection_string\": \"postgresql://...\"})",
            ));
        }
        // 骨架：实际执行需启用 sqlx "postgres" feature。
        // 调用路径已就位：配置 → 解析连接串 → 建池 → sqlx::query。
        // 此处返回工具调用回执，便于前端联调与延迟验证（Task 3.2.12）。
        match tool_name {
            "list_tables" => {
                let schema = args
                    .get("schema")
                    .and_then(|v| v.as_str())
                    .unwrap_or("public");
                Ok(json_result(&serde_json::json!({
                    "tool": "list_tables",
                    "schema": schema,
                    "status": "skeleton",
                    "note": "需启用 sqlx postgres feature 后接入真实 Postgres"
                })))
            }
            "query" => {
                let sql = args
                    .get("sql")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 sql".into()))?;
                Ok(json_result(&serde_json::json!({
                    "tool": "query",
                    "sql": sql,
                    "status": "skeleton",
                    "note": "需启用 sqlx postgres feature 后接入真实 Postgres"
                })))
            }
            "describe_table" => {
                let table_name = args
                    .get("table_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 table_name".into()))?;
                Ok(json_result(&serde_json::json!({
                    "tool": "describe_table",
                    "table_name": table_name,
                    "status": "skeleton",
                    "note": "需启用 sqlx postgres feature 后接入真实 Postgres"
                })))
            }
            _ => Ok(error_result(&format!("未知工具: {}", tool_name))),
        }
    }

    async fn configure(&self, config: serde_json::Value) -> Result<(), AppError> {
        if let Some(cs) = config.get("connection_string").and_then(|v| v.as_str()) {
            *self.connection_string.write().await = Some(cs.to_string());
            Ok(())
        } else {
            Err(AppError::Validation(
                "configure 缺少 connection_string 字段".into(),
            ))
        }
    }
}
