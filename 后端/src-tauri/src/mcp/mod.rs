pub mod builtin;
pub mod client;
pub mod lifecycle;
pub mod sdk;
pub mod server;
pub mod tool_filter;
pub mod tool_shaper;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::models::mcp::{
    McpHealthCheckResult, McpLifecycleConfig, McpRegisteredTool,
    McpServerRegistration, McpServerStatus, ToolCallResult,
};

use self::client::McpClient;
use self::lifecycle::{LifecycleConfig, McpLifecycleManager, McpLifecycleState};
use self::sdk::BuiltinRegistry;
use self::server::McpServer;
use self::tool_filter::ToolFilter;
use self::tool_shaper::ToolShaper;

/// D1 v3.1 Task 3.2.11: MCP 工具调用历史记录条目
#[derive(Debug, Clone, serde::Serialize)]
pub struct McpToolCallHistoryEntry {
    pub server_id: String,
    pub server_name: String,
    pub tool_name: String,
    pub arguments: Option<serde_json::Value>,
    pub success: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
    pub result_preview: Option<String>,
    pub called_at: i64,
}

/// 封装 MCP 客户端 + 生命周期管理器
struct McpClientInstance {
    client: McpClient,
    lifecycle: McpLifecycleManager,
}

impl McpClientInstance {
    fn new(id: &str) -> Self {
        Self {
            client: McpClient::new(id),
            lifecycle: McpLifecycleManager::new(None),
        }
    }

    fn lifecycle_state(&self) -> String {
        match self.lifecycle.state() {
            McpLifecycleState::Disconnected => "disconnected".into(),
            McpLifecycleState::Connecting => "connecting".into(),
            McpLifecycleState::Connected => "connected".into(),
            McpLifecycleState::HealthChecking => "health_checking".into(),
            McpLifecycleState::Unhealthy => "unhealthy".into(),
            McpLifecycleState::Reconnecting => "reconnecting".into(),
            McpLifecycleState::Shutdown => "shutdown".into(),
        }
    }
}

pub struct McpManager {
    servers: Arc<RwLock<HashMap<String, McpServerRegistration>>>,
    clients: Arc<RwLock<HashMap<String, McpClientInstance>>>,
    /// 保留字段：内置 MCP 服务器实例（当前通过 ensure_builtins_registered() 惰性注册，
    /// 此字段保留供未来直接调用场景使用）
    #[allow(dead_code)]
    builtin_server: Arc<McpServer>,
    shaper: Arc<RwLock<ToolShaper>>,
    filter: Arc<RwLock<ToolFilter>>,
    /// 延迟加载的命名空间
    deferred_namespaces: Arc<RwLock<Vec<String>>>,
    /// D1 v3.1 Task 3.2.1-3.2.9: 内置 MCP 服务器注册表（进程内，无子进程）
    builtin: Arc<BuiltinRegistry>,
    /// D1 v3.1 Task 3.2.11: 工具调用历史（内存环形缓冲，最近 100 条）
    history: Arc<RwLock<Vec<McpToolCallHistoryEntry>>>,
}

impl McpManager {
    pub fn new() -> Self {
        // 内置服务器注册在 ensure_builtins_registered() 中惰性完成
        // （new() 非 async，且 setup 闭包不在 tokio runtime 上下文内，避免 try_current 失败）
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
            builtin_server: Arc::new(McpServer::new()),
            shaper: Arc::new(RwLock::new(ToolShaper::new())),
            filter: Arc::new(RwLock::new(ToolFilter::new())),
            deferred_namespaces: Arc::new(RwLock::new(Vec::new())),
            builtin: Arc::new(BuiltinRegistry::new()),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// D1 v3.1 Task 3.2.1: 确保内置服务器已注册（幂等，首次调用前触发）
    pub async fn ensure_builtins_registered(&self) {
        if self.builtin.list_servers().await.is_empty() {
            builtin::register_all(&self.builtin).await;
        }
    }

    // ===== 3.1 生命周期: 注册 =====

    pub async fn register_server(
        &self,
        server: McpServerRegistration,
        auto_connect: bool,
    ) -> Result<McpServerStatus, AppError> {
        let id = server.id.clone();
        let name = server.name.clone();
        self.servers.write().await.insert(id.clone(), server.clone());

        if auto_connect && server.enabled {
            self.connect_server(&id).await
        } else {
            Ok(McpServerStatus {
                id,
                name,
                connected: false,
                tools_count: 0,
                resources_count: 0,
                prompts_count: 0,
                error: None,
                lifecycle_state: Some("disconnected".into()),
                reconnect_count: Some(0),
                consecutive_failures: Some(0),
            })
        }
    }

    // ===== 3.1 生命周期: Spawn（启动进程后异步连接） =====

    pub async fn spawn_server(
        &self,
        server_id: &str,
    ) -> Result<McpServerStatus, AppError> {
        let server = {
            self.servers
                .read()
                .await
                .get(server_id)
                .cloned()
                .ok_or(AppError::NotFound)?
        };

        let mut instance = McpClientInstance::new(server_id);
        instance.lifecycle.begin_connect();

        let result = instance.client.connect(
            &server.command,
            &server.args,
            server.env.as_ref(),
        ).await;

        match result {
            Ok(()) => {
                instance.lifecycle.on_connected();
                let tool_count = instance.client.tools().len();
                self.clients.write().await.insert(server_id.to_string(), instance);

                Ok(McpServerStatus {
                    id: server.id.clone(),
                    name: server.name.clone(),
                    connected: true,
                    tools_count: tool_count,
                    resources_count: 0,
                    prompts_count: 0,
                    error: None,
                    lifecycle_state: Some("connected".into()),
                    reconnect_count: Some(0),
                    consecutive_failures: Some(0),
                })
            }
            Err(e) => {
                let should_retry = instance.lifecycle.on_connect_failed();
                let state = instance.lifecycle_state();
                self.clients.write().await.insert(server_id.to_string(), instance);

                Ok(McpServerStatus {
                    id: server.id.clone(),
                    name: server.name.clone(),
                    connected: false,
                    tools_count: 0,
                    resources_count: 0,
                    prompts_count: 0,
                    error: Some(if should_retry {
                        format!("连接失败，将自动重连: {}", e)
                    } else {
                        format!("连接失败: {}", e)
                    }),
                    lifecycle_state: Some(state),
                    reconnect_count: None,
                    consecutive_failures: None,
                })
            }
        }
    }

    // ===== 3.1 生命周期: 连接 =====

    pub async fn connect_server(&self, server_id: &str) -> Result<McpServerStatus, AppError> {
        let server = {
            let servers = self.servers.read().await;
            servers
                .get(server_id)
                .cloned()
                .ok_or(AppError::NotFound)?
        };

        let mut instance = McpClientInstance::new(server_id);
        instance.lifecycle.begin_connect();

        let result = instance.client.connect(
            &server.command,
            &server.args,
            server.env.as_ref(),
        ).await;

        match result {
            Ok(()) => {
                instance.lifecycle.on_connected();
                let tool_count = instance.client.tools().len();
                self.clients
                    .write()
                    .await
                    .insert(server_id.to_string(), instance);

                Ok(McpServerStatus {
                    id: server.id.clone(),
                    name: server.name.clone(),
                    connected: true,
                    tools_count: tool_count,
                    resources_count: 0,
                    prompts_count: 0,
                    error: None,
                    lifecycle_state: Some("connected".into()),
                    reconnect_count: Some(0),
                    consecutive_failures: Some(0),
                })
            }
            Err(e) => {
                let should_retry = instance.lifecycle.on_connect_failed();
                let state = instance.lifecycle_state();
                self.clients.write().await.insert(server_id.to_string(), instance);

                Ok(McpServerStatus {
                    id: server.id.clone(),
                    name: server.name.clone(),
                    connected: false,
                    tools_count: 0,
                    resources_count: 0,
                    prompts_count: 0,
                    error: Some(if should_retry {
                        format!("连接失败，将自动重连: {}", e)
                    } else {
                        format!("连接失败: {}", e)
                    }),
                    lifecycle_state: Some(state),
                    reconnect_count: None,
                    consecutive_failures: None,
                })
            }
        }
    }

    // ===== 3.1 生命周期: 断开 =====

    pub async fn disconnect_server(&self, server_id: &str) -> Result<(), AppError> {
        if let Some(mut instance) = self.clients.write().await.remove(server_id) {
            instance.lifecycle.graceful_shutdown(&mut instance.client).await;
        }
        Ok(())
    }

    // ===== 3.1 生命周期: 健康检查 =====

    pub async fn health_check(&self, server_id: &str) -> Result<McpHealthCheckResult, AppError> {
        let mut clients = self.clients.write().await;
        let instance = clients
            .get_mut(server_id)
            .ok_or(AppError::NotFound)?;

        let start = std::time::Instant::now();
        let result = instance.client.ping().await;
        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(()) => {
                Ok(McpHealthCheckResult {
                    server_id: server_id.to_string(),
                    healthy: true,
                    latency_ms: Some(latency_ms),
                    error: None,
                    lifecycle_state: instance.lifecycle_state(),
                })
            }
            Err(e) => {
                let should_retry = instance.lifecycle.on_connect_failed();
                let state = instance.lifecycle_state();

                Ok(McpHealthCheckResult {
                    server_id: server_id.to_string(),
                    healthy: false,
                    latency_ms: Some(latency_ms),
                    error: Some(if should_retry {
                        format!("健康检查失败，将自动重连: {}", e)
                    } else {
                        format!("健康检查失败: {}", e)
                    }),
                    lifecycle_state: state,
                })
            }
        }
    }

    /// 对所有已连接服务器执行健康检查
    pub async fn health_check_all(&self) -> Vec<McpHealthCheckResult> {
        let mut results = Vec::new();
        let server_ids: Vec<String> = {
            self.clients.read().await.keys().cloned().collect()
        };

        for server_id in server_ids {
            match self.health_check(&server_id).await {
                Ok(result) => results.push(result),
                Err(_) => {
                    results.push(McpHealthCheckResult {
                        server_id: server_id.clone(),
                        healthy: false,
                        latency_ms: None,
                        error: Some("服务器未找到".into()),
                        lifecycle_state: "disconnected".into(),
                    });
                }
            }
        }

        results
    }

    /// 更新生命周期配置
    pub async fn update_lifecycle_config(
        &self,
        server_id: &str,
        config: McpLifecycleConfig,
    ) -> Result<(), AppError> {
        let mut clients = self.clients.write().await;
        let instance = clients
            .get_mut(server_id)
            .ok_or(AppError::NotFound)?;

        let lifecycle_config = LifecycleConfig {
            connect_timeout_ms: config.connect_timeout_ms.unwrap_or(30_000),
            health_check_interval_ms: config.health_check_interval_ms.unwrap_or(30_000),
            health_check_timeout_ms: config.health_check_timeout_ms.unwrap_or(5_000),
            max_reconnect_attempts: config.max_reconnect_attempts.unwrap_or(3),
            reconnect_interval_ms: config.reconnect_interval_ms.unwrap_or(5_000),
            graceful_shutdown_timeout_ms: 10_000,
            health_check_enabled: config.health_check_enabled.unwrap_or(true),
            auto_reconnect: config.auto_reconnect.unwrap_or(true),
        };

        instance.lifecycle.update_config(lifecycle_config);
        Ok(())
    }

    /// 获取生命周期状态
    pub async fn get_lifecycle_state(&self, server_id: &str) -> Result<String, AppError> {
        let clients = self.clients.read().await;
        let instance = clients
            .get(server_id)
            .ok_or(AppError::NotFound)?;

        Ok(instance.lifecycle_state())
    }

    // ===== 工具管理 =====

    pub async fn list_all_tools(&self) -> Vec<McpRegisteredTool> {
        let mut all_tools = vec![];

        let clients = self.clients.read().await;
        for (server_id, instance) in clients.iter() {
            let server_name = self
                .servers
                .read()
                .await
                .get(server_id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| server_id.clone());

            for tool in instance.client.tools() {
                all_tools.push(McpRegisteredTool {
                    server_id: server_id.clone(),
                    server_name: server_name.clone(),
                    tool: tool.clone(),
                });
            }
        }
        drop(clients);

        // D1 v3.1 Task 3.2.1-3.2.9: 追加内置 MCP 服务器工具
        self.ensure_builtins_registered().await;
        for (server_name, tool) in self.builtin.list_all_tools().await {
            all_tools.push(McpRegisteredTool {
                server_id: server_name.clone(),
                server_name,
                tool,
            });
        }

        all_tools
    }

    pub async fn server_status(&self, server_id: &str) -> Result<McpServerStatus, AppError> {
        let server = {
            self.servers
                .read()
                .await
                .get(server_id)
                .cloned()
                .ok_or(AppError::NotFound)?
        };

        let clients = self.clients.read().await;
        let instance = clients.get(server_id);
        let connected = instance.is_some() && instance.unwrap().client.is_connected();
        let tools_count = instance.map(|i| i.client.tools().len()).unwrap_or(0);
        let lifecycle_state = instance.map(|i| i.lifecycle_state());

        Ok(McpServerStatus {
            id: server.id,
            name: server.name,
            connected,
            tools_count,
            resources_count: 0,
            prompts_count: 0,
            error: None,
            lifecycle_state,
            reconnect_count: None,
            consecutive_failures: None,
        })
    }

    pub async fn list_servers(&self) -> Vec<McpServerStatus> {
        let servers = self.servers.read().await;
        let mut statuses = vec![];

        for (id, server) in servers.iter() {
            let clients = self.clients.read().await;
            let instance = clients.get(id);
            let connected = instance.is_some() && instance.unwrap().client.is_connected();
            let tools_count = instance.map(|i| i.client.tools().len()).unwrap_or(0);
            let lifecycle_state = instance.map(|i| i.lifecycle_state());

            statuses.push(McpServerStatus {
                id: id.clone(),
                name: server.name.clone(),
                connected,
                tools_count,
                resources_count: 0,
                prompts_count: 0,
                error: None,
                lifecycle_state,
                reconnect_count: None,
                consecutive_failures: None,
            });
        }

        statuses
    }

    pub async fn call_tool(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<ToolCallResult, AppError> {
        // D1 v3.1 Task 3.2.1-3.2.9: 优先路由到内置 MCP 服务器（命名空间命中即走进程内调用）
        self.ensure_builtins_registered().await;
        if self.builtin.has_server(server_id).await {
            let start = std::time::Instant::now();
            let result = self
                .builtin
                .call_tool(server_id, tool_name, arguments.clone())
                .await;
            let latency_ms = start.elapsed().as_millis() as u64;
            self.record_history(
                server_id,
                server_id,
                tool_name,
                arguments,
                &result,
                latency_ms,
            )
            .await;
            return result;
        }

        // 外部连接的 MCP 服务器
        let mut clients = self.clients.write().await;
        let instance = clients
            .get_mut(server_id)
            .ok_or(AppError::NotFound)?;

        let server_name = self
            .servers
            .read()
            .await
            .get(server_id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| server_id.to_string());
        let start = std::time::Instant::now();
        let result = instance.client.call_tool(tool_name, arguments.clone()).await;
        let latency_ms = start.elapsed().as_millis() as u64;
        self.record_history(
            server_id,
            &server_name,
            tool_name,
            arguments,
            &result,
            latency_ms,
        )
        .await;
        result
    }

    // ===== 3.2.1-3.2.9 内置 MCP 服务器管理 =====

    /// 列出内置 MCP 服务器名称
    pub async fn list_builtin_servers(&self) -> Vec<String> {
        self.ensure_builtins_registered().await;
        self.builtin.list_servers().await
    }

    /// 列出内置 MCP 服务器工具
    pub async fn list_builtin_tools(&self) -> Vec<McpRegisteredTool> {
        self.ensure_builtins_registered().await;
        self.builtin
            .list_all_tools()
            .await
            .into_iter()
            .map(|(server_name, tool)| McpRegisteredTool {
                server_id: server_name.clone(),
                server_name,
                tool,
            })
            .collect()
    }

    /// 调用内置 MCP 服务器工具（直接入口，绕过外部客户端查找）
    pub async fn call_builtin_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<ToolCallResult, AppError> {
        self.ensure_builtins_registered().await;
        let start = std::time::Instant::now();
        let result = self
            .builtin
            .call_tool(server_name, tool_name, arguments.clone())
            .await;
        let latency_ms = start.elapsed().as_millis() as u64;
        self.record_history(
            server_name,
            server_name,
            tool_name,
            arguments,
            &result,
            latency_ms,
        )
        .await;
        result
    }

    /// 配置内置 MCP 服务器（注入 API Key / 连接字符串等）
    pub async fn configure_builtin(
        &self,
        server_name: &str,
        config: serde_json::Value,
    ) -> Result<(), AppError> {
        self.ensure_builtins_registered().await;
        self.builtin.configure(server_name, config).await
    }

    // ===== 3.2.11 工具调用历史 =====

    /// 记录调用历史（保留最近 100 条）
    async fn record_history(
        &self,
        server_id: &str,
        server_name: &str,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
        result: &Result<ToolCallResult, AppError>,
        latency_ms: u64,
    ) {
        let mut history = self.history.write().await;
        let (success, error, result_preview) = match result {
            Ok(r) => {
                let preview = r
                    .content
                    .first()
                    .and_then(|c| c.text.clone())
                    .map(|t| t.chars().take(500).collect::<String>());
                (true, None, preview)
            }
            Err(e) => (false, Some(e.to_string()), None),
        };
        history.push(McpToolCallHistoryEntry {
            server_id: server_id.to_string(),
            server_name: server_name.to_string(),
            tool_name: tool_name.to_string(),
            arguments,
            success,
            latency_ms,
            error,
            result_preview,
            called_at: chrono::Utc::now().timestamp(),
        });
        // 环形缓冲：保留最近 100 条
        if history.len() > 100 {
            let drop_count = history.len() - 100;
            history.drain(0..drop_count);
        }
    }

    /// 获取工具调用历史（最新在前）
    pub async fn list_call_history(&self) -> Vec<McpToolCallHistoryEntry> {
        let history = self.history.read().await;
        history.iter().rev().cloned().collect()
    }

    // ===== 3.2 工具 Schema 塑形 =====

    /// 获取塑形后的工具列表（去重 + 命名空间）
    pub async fn list_shaped_tools(&self) -> Vec<McpRegisteredTool> {
        let raw = self.list_all_tools().await;
        let shaper = self.shaper.read().await;
        shaper.shape_tools(&raw)
    }

    /// 按命名空间分组工具
    pub async fn group_tools_by_namespace(&self) -> HashMap<String, Vec<McpRegisteredTool>> {
        let shaped = self.list_shaped_tools().await;
        let shaper = self.shaper.read().await;
        shaper.group_by_namespace(&shaped)
    }

    /// 过滤指定命名空间的工具
    pub async fn list_tools_by_namespace(&self, namespace: &str) -> Vec<McpRegisteredTool> {
        let shaped = self.list_shaped_tools().await;
        let shaper = self.shaper.read().await;
        shaper.filter_by_namespace(&shaped, namespace)
    }

    /// 获取所有命名空间
    pub async fn get_tool_namespaces(&self) -> Vec<String> {
        let shaped = self.list_shaped_tools().await;
        let shaper = self.shaper.read().await;
        shaper.get_namespaces(&shaped)
    }

    /// 更新塑形器配置
    pub async fn update_shaper_config(
        &self,
        preferred_servers: Option<Vec<String>>,
        enable_namespace: Option<bool>,
    ) {
        let mut shaper = self.shaper.write().await;
        let current_servers = shaper.preferred_servers.clone();
        let current_namespace = shaper.enable_namespace;

        let servers = preferred_servers.unwrap_or(current_servers);
        let namespace = enable_namespace.unwrap_or(current_namespace);

        *shaper = ToolShaper::new()
            .with_namespace(namespace)
            .with_preferred_servers(servers);
    }

    // ===== 3.3 工具过滤与搜索 =====

    /// 搜索工具
    pub async fn search_tools(&self, query: &str) -> Vec<McpRegisteredTool> {
        let shaped = self.list_shaped_tools().await;
        let filter = self.filter.read().await;
        filter.search_tools(&shaped, query)
    }

    /// 设置启用/禁用工具
    pub async fn set_tool_enabled(&self, tool_name: &str, enabled: bool) {
        let mut filter = self.filter.write().await;
        if enabled {
            filter.enable_tool(tool_name);
        } else {
            filter.disable_tool(tool_name);
        }
    }

    /// 设置启用工具列表
    pub async fn set_enabled_tools(&self, tools: Vec<String>) {
        let mut filter = self.filter.write().await;
        filter.set_enabled_tools(tools);
    }

    /// 设置禁用工具列表
    pub async fn set_disabled_tools(&self, tools: Vec<String>) {
        let mut filter = self.filter.write().await;
        filter.set_disabled_tools(tools);
    }

    /// 获取过滤后的工具列表
    pub async fn list_filtered_tools(&self) -> Vec<McpRegisteredTool> {
        let shaped = self.list_shaped_tools().await;
        let filter = self.filter.read().await;
        filter.apply_enabled_filter(&shaped)
    }

    // ===== 3.4 MCP 资源支持 =====

    /// 列出 MCP 资源
    pub async fn list_resources(
        &self,
        server_id: &str,
    ) -> Result<Vec<crate::models::mcp::Resource>, AppError> {
        let mut clients = self.clients.write().await;
        let instance = clients
            .get_mut(server_id)
            .ok_or(AppError::NotFound)?;

        instance.client.list_resources().await
    }

    /// 读取 MCP 资源
    pub async fn read_resource(
        &self,
        server_id: &str,
        uri: &str,
    ) -> Result<crate::models::mcp::ReadResourceResult, AppError> {
        let mut clients = self.clients.write().await;
        let instance = clients
            .get_mut(server_id)
            .ok_or(AppError::NotFound)?;

        instance.client.read_resource(uri).await
    }

    /// 列出 MCP Prompt 模板
    pub async fn list_prompts(
        &self,
        server_id: &str,
    ) -> Result<Vec<crate::models::mcp::Prompt>, AppError> {
        let mut clients = self.clients.write().await;
        let instance = clients
            .get_mut(server_id)
            .ok_or(AppError::NotFound)?;

        instance.client.list_prompts().await
    }

    /// 获取 MCP Prompt 模板
    pub async fn get_prompt(
        &self,
        server_id: &str,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<crate::models::mcp::GetPromptResult, AppError> {
        let mut clients = self.clients.write().await;
        let instance = clients
            .get_mut(server_id)
            .ok_or(AppError::NotFound)?;

        instance.client.get_prompt(name, arguments).await
    }

    // ===== 3.5 延迟工具加载 =====

    /// 设置延迟加载的命名空间
    pub async fn set_deferred_namespaces(&self, namespaces: Vec<String>) {
        let mut deferred = self.deferred_namespaces.write().await;
        *deferred = namespaces;
    }

    /// 获取延迟加载的命名空间
    pub async fn get_deferred_namespaces(&self) -> Vec<String> {
        self.deferred_namespaces.read().await.clone()
    }

    /// 按需加载命名空间（连接指定命名空间对应的服务器）
    pub async fn load_namespace(&self, namespace: &str) -> Result<McpServerStatus, AppError> {
        // 先在已注册的服务器中查找匹配的命名空间
        let server_id = {
            let servers = self.servers.read().await;
            servers
                .iter()
                .find(|(_, s)| s.name == namespace)
                .map(|(id, _)| id.clone())
                .or_else(|| {
                    servers.iter().find(|(id, _)| id.as_str() == namespace).map(|(id, _)| id.clone())
                })
        };

        match server_id {
            Some(id) => {
                // 检查是否已连接
                let already_connected = {
                    let clients = self.clients.read().await;
                    clients.contains_key(&id)
                };

                if already_connected {
                    // 从延迟列表中移除
                    let mut deferred = self.deferred_namespaces.write().await;
                    deferred.retain(|n| n != namespace);
                    self.server_status(&id).await
                } else {
                    let status = self.connect_server(&id).await;
                    // 连接成功后从延迟列表中移除
                    if status.is_ok() {
                        let mut deferred = self.deferred_namespaces.write().await;
                        deferred.retain(|n| n != namespace);
                    }
                    status
                }
            }
            None => Err(AppError::NotFound),
        }
    }

    /// 卸载命名空间（断开对应服务器）
    pub async fn unload_namespace(&self, namespace: &str) -> Result<(), AppError> {
        let server_id = {
            let servers = self.servers.read().await;
            servers
                .iter()
                .find(|(_, s)| s.name == namespace)
                .map(|(id, _)| id.clone())
                .or_else(|| {
                    servers.iter().find(|(id, _)| id.as_str() == namespace).map(|(id, _)| id.clone())
                })
                .ok_or(AppError::NotFound)?
        };

        self.disconnect_server(&server_id).await?;

        // 添加到延迟列表
        let mut deferred = self.deferred_namespaces.write().await;
        if !deferred.contains(&namespace.to_string()) {
            deferred.push(namespace.to_string());
        }

        Ok(())
    }
}