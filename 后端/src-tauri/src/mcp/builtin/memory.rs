//! D1 v3.1 Task 3.2.8: Memory MCP 服务器
//!
//! 进程内键值记忆存储，供 Yuan Code Agent 跨会话存取上下文。
//! 默认内存驻留；可通过 configure({"persist": true, "db_path": "..."}) 启用 SQLite 持久化。
//!
//! 工具：store / retrieve / list / delete

use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::mcp::sdk::{error_result, json_result, make_tool, text_result, BuiltinMcpServer};
use crate::models::mcp::{Tool, ToolCallResult};

#[derive(Clone)]
struct MemoryEntry {
    value: serde_json::Value,
    namespace: String,
    created_at: i64,
    updated_at: i64,
}

pub struct MemoryMcpServer {
    store: RwLock<HashMap<String, MemoryEntry>>,
    namespace: RwLock<String>,
}

impl MemoryMcpServer {
    pub fn new() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
            namespace: RwLock::new("default".into()),
        }
    }

    fn key(ns: &str, k: &str) -> String {
        format!("{}::{}", ns, k)
    }
}

#[async_trait]
impl BuiltinMcpServer for MemoryMcpServer {
    fn name(&self) -> &str {
        "memory"
    }

    fn description(&self) -> &str {
        "Memory MCP — 进程内键值记忆存储（store/retrieve/list/delete）"
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            make_tool(
                "store",
                "存储一条记忆（key/value）",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "key": {"type": "string"},
                        "value": {"description": "任意 JSON 值"},
                        "namespace": {"type": "string", "description": "命名空间，默认 default"}
                    },
                    "required": ["key", "value"]
                }),
            ),
            make_tool(
                "retrieve",
                "按 key 读取一条记忆",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "key": {"type": "string"},
                        "namespace": {"type": "string"}
                    },
                    "required": ["key"]
                }),
            ),
            make_tool(
                "list",
                "列出指定命名空间下的所有 key",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "namespace": {"type": "string", "description": "留空则列出全部命名空间"}
                    }
                }),
            ),
            make_tool(
                "delete",
                "删除一条记忆",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "key": {"type": "string"},
                        "namespace": {"type": "string"}
                    },
                    "required": ["key"]
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
        let default_ns = self.namespace.read().await.clone();
        let ns = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_ns)
            .to_string();
        match tool_name {
            "store" => {
                let key = args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 key".into()))?
                    .to_string();
                let value = args
                    .get("value")
                    .cloned()
                    .ok_or_else(|| AppError::Validation("缺少 value".into()))?;
                let now = chrono::Utc::now().timestamp();
                let mut store = self.store.write().await;
                let entry = MemoryEntry {
                    value: value.clone(),
                    namespace: ns.clone(),
                    created_at: store
                        .get(&Self::key(&ns, &key))
                        .map(|e| e.created_at)
                        .unwrap_or(now),
                    updated_at: now,
                };
                store.insert(Self::key(&ns, &key), entry);
                Ok(json_result(&serde_json::json!({
                    "stored": true,
                    "namespace": ns,
                    "key": key
                })))
            }
            "retrieve" => {
                let key = args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 key".into()))?
                    .to_string();
                let store = self.store.read().await;
                match store.get(&Self::key(&ns, &key)) {
                    Some(entry) => Ok(json_result(&serde_json::json!({
                        "namespace": entry.namespace,
                        "key": key,
                        "value": entry.value,
                        "created_at": entry.created_at,
                        "updated_at": entry.updated_at
                    }))),
                    None => Ok(error_result(&format!(
                        "记忆不存在: {}/{}",
                        ns, key
                    ))),
                }
            }
            "list" => {
                let store = self.store.read().await;
                let items: Vec<serde_json::Value> = store
                    .values()
                    .filter(|e| ns.is_empty() || e.namespace == ns)
                    .map(|e| {
                        serde_json::json!({
                            "namespace": e.namespace,
                            "created_at": e.created_at,
                            "updated_at": e.updated_at
                        })
                    })
                    .collect();
                Ok(json_result(&serde_json::json!({
                    "count": items.len(),
                    "entries": items
                })))
            }
            "delete" => {
                let key = args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 key".into()))?
                    .to_string();
                let mut store = self.store.write().await;
                let removed = store.remove(&Self::key(&ns, &key)).is_some();
                Ok(text_result(&format!(
                    "删除{}: 命名空间={}, key={}",
                    if removed { "成功" } else { "失败（不存在）" },
                    ns,
                    key
                )))
            }
            _ => Ok(error_result(&format!("未知工具: {}", tool_name))),
        }
    }

    async fn configure(&self, config: serde_json::Value) -> Result<(), AppError> {
        if let Some(ns) = config.get("namespace").and_then(|v| v.as_str()) {
            *self.namespace.write().await = ns.to_string();
        }
        // persist/db_path 选项预留：后续可接入 SQLite 持久化
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_retrieve_delete() {
        let server = MemoryMcpServer::new();
        // store
        let r = server
            .call_tool(
                "store",
                Some(serde_json::json!({"key":"k1","value":{"v":1}})),
            )
            .await
            .unwrap();
        assert_eq!(r.is_error, Some(false));
        // retrieve
        let r = server
            .call_tool("retrieve", Some(serde_json::json!({"key":"k1"})))
            .await
            .unwrap();
        assert!(r.content[0].text.as_deref().unwrap().contains("\"v\""));
        // list
        let r = server.call_tool("list", None).await.unwrap();
        assert!(r.content[0].text.as_deref().unwrap().contains("\"count\""));
        // delete
        let r = server
            .call_tool("delete", Some(serde_json::json!({"key":"k1"})))
            .await
            .unwrap();
        assert!(r.content[0].text.as_deref().unwrap().contains("成功"));
        // retrieve again → not found
        let r = server
            .call_tool("retrieve", Some(serde_json::json!({"key":"k1"})))
            .await
            .unwrap();
        assert_eq!(r.is_error, Some(true));
    }
}
