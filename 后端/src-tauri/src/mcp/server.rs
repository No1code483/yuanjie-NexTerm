use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::mcp::{
    JsonRpcError, JsonRpcRequest, JsonRpcResponse, Prompt,
    Resource, ServerCapabilities, ServerInfo, Tool, ToolCallParams,
    ToolCallResult, ToolsCapability,
};

pub type ToolHandler =
    Arc<dyn Fn(ToolCallParams) -> Result<ToolCallResult, String> + Send + Sync>;

pub struct McpServer {
    tools: Arc<RwLock<HashMap<String, Tool>>>,
    handlers: Arc<RwLock<HashMap<String, ToolHandler>>>,
    resources: Arc<RwLock<HashMap<String, Resource>>>,
    prompts: Arc<RwLock<HashMap<String, Prompt>>>,
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            resources: Arc::new(RwLock::new(HashMap::new())),
            prompts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_tool(
        &self,
        tool: Tool,
        handler: ToolHandler,
    ) {
        let name = tool.name.clone();
        self.tools.write().await.insert(name.clone(), tool);
        self.handlers.write().await.insert(name, handler);
    }

    pub async fn register_resource(&self, resource: Resource, content: String, mime_type: Option<String>) {
        let uri = resource.uri.clone();
        self.resources.write().await.insert(uri, resource);
        // resource contents stored separately for read handling
        // In a full implementation this would be a separate store
        let _ = (content, mime_type);
    }

    pub async fn register_prompt(&self, prompt: Prompt) {
        let name = prompt.name.clone();
        self.prompts.write().await.insert(name, prompt);
    }

    pub async fn handle_request(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "tools/list" => self.handle_tools_list(request).await,
            "tools/call" => self.handle_tools_call(request).await,
            "resources/list" => self.handle_resources_list(request).await,
            "resources/read" => self.handle_resources_read(request).await,
            "prompts/list" => self.handle_prompts_list(request).await,
            "prompts/get" => self.handle_prompts_get(request).await,
            _ => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: request.id.clone(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("方法未找到: {}", request.method),
                    data: None,
                }),
            },
        }
    }

    async fn handle_initialize(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let server_info = ServerInfo {
            name: "YuanCode-MCP".into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let response = crate::models::mcp::InitializeResult {
            protocol_version: "2024-11-05".into(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: None,
                prompts: None,
                logging: None,
                experimental: None,
            },
            server_info,
            instructions: Some("YuanCode MCP Server - 文件操作、代码执行、知识库搜索".into()),
        };

        JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: request.id.clone(),
            result: Some(serde_json::to_value(response).unwrap_or_default()),
            error: None,
        }
    }

    async fn handle_tools_list(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let tools_guard = self.tools.read().await;
        let tools: Vec<&Tool> = tools_guard.values().collect();

        let result = serde_json::json!({
            "tools": tools
        });

        JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: request.id.clone(),
            result: Some(result),
            error: None,
        }
    }

    async fn handle_tools_call(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let params: ToolCallParams = match request
            .params
            .as_ref()
            .and_then(|p| serde_json::from_value(p.clone()).ok())
        {
            Some(p) => p,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id: request.id.clone(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "无效参数".into(),
                        data: None,
                    }),
                }
            }
        };

        let handlers = self.handlers.read().await;
        let handler = match handlers.get(&params.name) {
            Some(h) => h.clone(),
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id: request.id.clone(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: format!("工具未找到: {}", params.name),
                        data: None,
                    }),
                }
            }
        };

        match handler(params) {
            Ok(result) => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: request.id.clone(),
                result: Some(serde_json::to_value(result).unwrap_or_default()),
                error: None,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: request.id.clone(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32000,
                    message: e,
                    data: None,
                }),
            },
        }
    }

    async fn handle_resources_list(&self, _request: &JsonRpcRequest) -> JsonRpcResponse {
        let resources_guard = self.resources.read().await;
        let resources: Vec<&Resource> = resources_guard.values().collect();
        JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: _request.id.clone(),
            result: Some(serde_json::json!({ "resources": resources })),
            error: None,
        }
    }

    async fn handle_resources_read(&self, _request: &JsonRpcRequest) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: _request.id.clone(),
            result: Some(serde_json::json!({ "contents": [] })),
            error: None,
        }
    }

    async fn handle_prompts_list(&self, _request: &JsonRpcRequest) -> JsonRpcResponse {
        let prompts_guard = self.prompts.read().await;
        let prompts: Vec<&Prompt> = prompts_guard.values().collect();
        JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: _request.id.clone(),
            result: Some(serde_json::json!({ "prompts": prompts })),
            error: None,
        }
    }

    async fn handle_prompts_get(&self, _request: &JsonRpcRequest) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: _request.id.clone(),
            result: Some(serde_json::json!({ "messages": [] })),
            error: None,
        }
    }
}