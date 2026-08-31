//! D1 v3.1 Task 3.2.4: SQLite MCP 服务器
//!
//! 查询用户指定的本地 SQLite 数据库文件（复用 sqlx sqlite feature）。
//! 注意：与项目自身的 nexterm.db 隔离——仅连接用户通过 configure 指定的 db_path。
//!
//! 工具：list_tables / query / execute

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::mcp::sdk::{error_result, json_result, make_tool, BuiltinMcpServer};
use crate::models::mcp::{Tool, ToolCallResult};

pub struct SqliteMcpServer {
    pool: RwLock<Option<sqlx::SqlitePool>>,
    db_path: RwLock<Option<String>>,
}

impl SqliteMcpServer {
    pub fn new() -> Self {
        Self {
            pool: RwLock::new(None),
            db_path: RwLock::new(None),
        }
    }

    async fn ensure_pool(&self) -> Result<sqlx::SqlitePool, AppError> {
        let pool_opt = self.pool.read().await;
        if let Some(p) = pool_opt.as_ref() {
            return Ok(p.clone());
        }
        drop(pool_opt);
        // 双检锁
        let mut pool_guard = self.pool.write().await;
        if let Some(p) = pool_guard.as_ref() {
            return Ok(p.clone());
        }
        let db_path = self
            .db_path
            .read()
            .await
            .clone()
            .ok_or_else(|| AppError::Validation("SQLite db_path 未配置".into()))?;
        let url = if db_path.starts_with("sqlite:") {
            db_path
        } else {
            format!("sqlite://{}?mode=rwc", db_path)
        };
        let p = sqlx::SqlitePool::connect(&url)
            .await
            .map_err(|e| AppError::Internal(format!("连接 SQLite 失败: {}", e)))?;
        *pool_guard = Some(p.clone());
        Ok(p)
    }
}

#[async_trait]
impl BuiltinMcpServer for SqliteMcpServer {
    fn name(&self) -> &str {
        "sqlite"
    }

    fn description(&self) -> &str {
        "SQLite MCP — 查询本地 SQLite 数据库（list_tables/query/execute）"
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            make_tool(
                "list_tables",
                "列出数据库中的所有表",
                serde_json::json!({"type": "object", "properties": {}}),
            ),
            make_tool(
                "query",
                "执行只读 SELECT 查询并返回行",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "sql": {"type": "string", "description": "SELECT  SQL 语句"},
                        "limit": {"type": "integer", "default": 100}
                    },
                    "required": ["sql"]
                }),
            ),
            make_tool(
                "execute",
                "执行写操作（INSERT/UPDATE/DELETE/DDL）",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "sql": {"type": "string"}
                    },
                    "required": ["sql"]
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
        // 延迟连接：未配置时返回明确提示
        if self.db_path.read().await.is_none() {
            return Ok(error_result(
                "SQLite db_path 未配置，请先调用 configure({\"db_path\": \"...\"})",
            ));
        }
        match tool_name {
            "list_tables" => {
                let pool = self.ensure_pool().await?;
                let rows: Vec<(String,)> = sqlx::query_as(
                    "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
                )
                .fetch_all(&pool)
                .await
                .map_err(|e| AppError::Internal(format!("查询表列表失败: {}", e)))?;
                let tables: Vec<String> = rows.into_iter().map(|r| r.0).collect();
                Ok(json_result(&serde_json::json!({
                    "tables": tables,
                    "count": tables.len()
                })))
            }
            "query" => {
                let sql = args
                    .get("sql")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 sql".into()))?;
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(100) as i64;
                let upper = sql.trim().to_uppercase();
                if !upper.starts_with("SELECT") && !upper.starts_with("WITH") {
                    return Ok(error_result(
                        "query 工具仅支持 SELECT / WITH 语句，写操作请用 execute",
                    ));
                }
                let pool = self.ensure_pool().await?;
                let rows: Vec<serde_json::Value> = sqlx::query(sql)
                    .fetch_all(&pool)
                    .await
                    .map_err(|e| AppError::Internal(format!("查询失败: {}", e)))?
                    .into_iter()
                    .map(|row| {
                        // 将 sqlx Row 序列化为 JSON 对象
                        use sqlx::{Column, Row};
                        let mut obj = serde_json::Map::new();
                        for (i, col) in row.columns().iter().enumerate() {
                            let name = col.name();
                            let val: serde_json::Value =
                                match row.try_get::<Option<String>, _>(i) {
                                    Ok(Some(s)) => serde_json::Value::String(s),
                                    _ => match row.try_get::<Option<i64>, _>(i) {
                                        Ok(Some(n)) => serde_json::Value::Number(n.into()),
                                        _ => match row.try_get::<Option<f64>, _>(i) {
                                            Ok(Some(f)) => serde_json::json!(f),
                                            _ => serde_json::Value::Null,
                                        },
                                    },
                                };
                            obj.insert(name.into(), val);
                        }
                        serde_json::Value::Object(obj)
                    })
                    .take(limit as usize)
                    .collect();
                Ok(json_result(&serde_json::json!({
                    "rows": rows,
                    "count": rows.len()
                })))
            }
            "execute" => {
                let sql = args
                    .get("sql")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 sql".into()))?;
                let pool = self.ensure_pool().await?;
                let result = sqlx::query(sql)
                    .execute(&pool)
                    .await
                    .map_err(|e| AppError::Internal(format!("执行失败: {}", e)))?;
                Ok(json_result(&serde_json::json!({
                    "rows_affected": result.rows_affected(),
                    "last_insert_rowid": result.last_insert_rowid()
                })))
            }
            _ => Ok(error_result(&format!("未知工具: {}", tool_name))),
        }
    }

    async fn configure(&self, config: serde_json::Value) -> Result<(), AppError> {
        if let Some(db_path) = config.get("db_path").and_then(|v| v.as_str()) {
            *self.db_path.write().await = Some(db_path.to_string());
            // 重置连接池，下次调用时按新路径重建
            *self.pool.write().await = None;
            Ok(())
        } else {
            Err(AppError::Validation("configure 缺少 db_path 字段".into()))
        }
    }
}
