use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::models::mcp::{
    McpHealthCheckResult, McpLifecycleConfig, McpRegisteredTool,
    McpServerRegistration, McpServerStatus, ToolCallResult,
};
use crate::mcp::{McpManager, McpToolCallHistoryEntry};

pub struct McpService {
    manager: Arc<RwLock<McpManager>>,
}

impl McpService {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(McpManager::new())),
        }
    }

    pub async fn register_server(
        &self,
        server: McpServerRegistration,
        auto_connect: bool,
    ) -> Result<McpServerStatus, AppError> {
        self.manager
            .read()
            .await
            .register_server(server, auto_connect)
            .await
    }

    // ===== 3.1 生命周期 =====

    pub async fn spawn_server(&self, server_id: &str) -> Result<McpServerStatus, AppError> {
        self.manager.read().await.spawn_server(server_id).await
    }

    pub async fn connect_server(&self, server_id: &str) -> Result<McpServerStatus, AppError> {
        self.manager.read().await.connect_server(server_id).await
    }

    pub async fn disconnect_server(&self, server_id: &str) -> Result<(), AppError> {
        self.manager.read().await.disconnect_server(server_id).await
    }

    pub async fn health_check(&self, server_id: &str) -> Result<McpHealthCheckResult, AppError> {
        self.manager.read().await.health_check(server_id).await
    }

    pub async fn health_check_all(&self) -> Result<Vec<McpHealthCheckResult>, AppError> {
        Ok(self.manager.read().await.health_check_all().await)
    }

    pub async fn update_lifecycle_config(
        &self,
        server_id: &str,
        config: McpLifecycleConfig,
    ) -> Result<(), AppError> {
        self.manager
            .read()
            .await
            .update_lifecycle_config(server_id, config)
            .await
    }

    pub async fn get_lifecycle_state(&self, server_id: &str) -> Result<String, AppError> {
        self.manager
            .read()
            .await
            .get_lifecycle_state(server_id)
            .await
    }

    // ===== 工具管理 =====

    pub async fn list_servers(&self) -> Result<Vec<McpServerStatus>, AppError> {
        Ok(self.manager.read().await.list_servers().await)
    }

    pub async fn list_all_tools(&self) -> Result<Vec<McpRegisteredTool>, AppError> {
        Ok(self.manager.read().await.list_all_tools().await)
    }

    pub async fn server_status(&self, server_id: &str) -> Result<McpServerStatus, AppError> {
        self.manager.read().await.server_status(server_id).await
    }

    pub async fn call_tool(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<ToolCallResult, AppError> {
        self.manager
            .read()
            .await
            .call_tool(server_id, tool_name, arguments)
            .await
    }

    // ===== 3.2 工具 Schema 塑形 =====

    pub async fn list_shaped_tools(&self) -> Result<Vec<McpRegisteredTool>, AppError> {
        Ok(self.manager.read().await.list_shaped_tools().await)
    }

    pub async fn group_tools_by_namespace(
        &self,
    ) -> Result<std::collections::HashMap<String, Vec<McpRegisteredTool>>, AppError> {
        Ok(self.manager.read().await.group_tools_by_namespace().await)
    }

    pub async fn list_tools_by_namespace(
        &self,
        namespace: &str,
    ) -> Result<Vec<McpRegisteredTool>, AppError> {
        Ok(self.manager.read().await.list_tools_by_namespace(namespace).await)
    }

    pub async fn get_tool_namespaces(&self) -> Result<Vec<String>, AppError> {
        Ok(self.manager.read().await.get_tool_namespaces().await)
    }

    pub async fn update_shaper_config(
        &self,
        preferred_servers: Option<Vec<String>>,
        enable_namespace: Option<bool>,
    ) -> Result<(), AppError> {
        self.manager
            .read()
            .await
            .update_shaper_config(preferred_servers, enable_namespace)
            .await;
        Ok(())
    }

    // ===== 3.3 工具过滤与搜索 =====

    pub async fn search_tools(&self, query: &str) -> Result<Vec<McpRegisteredTool>, AppError> {
        Ok(self.manager.read().await.search_tools(query).await)
    }

    pub async fn set_tool_enabled(&self, tool_name: &str, enabled: bool) -> Result<(), AppError> {
        self.manager.read().await.set_tool_enabled(tool_name, enabled).await;
        Ok(())
    }

    pub async fn set_enabled_tools(&self, tools: Vec<String>) -> Result<(), AppError> {
        self.manager.read().await.set_enabled_tools(tools).await;
        Ok(())
    }

    pub async fn set_disabled_tools(&self, tools: Vec<String>) -> Result<(), AppError> {
        self.manager.read().await.set_disabled_tools(tools).await;
        Ok(())
    }

    pub async fn list_filtered_tools(&self) -> Result<Vec<McpRegisteredTool>, AppError> {
        Ok(self.manager.read().await.list_filtered_tools().await)
    }

    // ===== 3.4 MCP 资源支持 =====

    pub async fn list_resources(
        &self,
        server_id: &str,
    ) -> Result<Vec<crate::models::mcp::Resource>, AppError> {
        self.manager.read().await.list_resources(server_id).await
    }

    pub async fn read_resource(
        &self,
        server_id: &str,
        uri: &str,
    ) -> Result<crate::models::mcp::ReadResourceResult, AppError> {
        self.manager.read().await.read_resource(server_id, uri).await
    }

    pub async fn list_prompts(
        &self,
        server_id: &str,
    ) -> Result<Vec<crate::models::mcp::Prompt>, AppError> {
        self.manager.read().await.list_prompts(server_id).await
    }

    pub async fn get_prompt(
        &self,
        server_id: &str,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<crate::models::mcp::GetPromptResult, AppError> {
        self.manager.read().await.get_prompt(server_id, name, arguments).await
    }

    // ===== 3.5 延迟工具加载 =====

    pub async fn set_deferred_namespaces(&self, namespaces: Vec<String>) -> Result<(), AppError> {
        self.manager.read().await.set_deferred_namespaces(namespaces).await;
        Ok(())
    }

    pub async fn get_deferred_namespaces(&self) -> Result<Vec<String>, AppError> {
        Ok(self.manager.read().await.get_deferred_namespaces().await)
    }

    pub async fn load_namespace(&self, namespace: &str) -> Result<McpServerStatus, AppError> {
        self.manager.read().await.load_namespace(namespace).await
    }

    pub async fn unload_namespace(&self, namespace: &str) -> Result<(), AppError> {
        self.manager.read().await.unload_namespace(namespace).await
    }

    // ===== D1 v3.1 Task 3.2.1-3.2.9: 内置 MCP 服务器管理 =====

    pub async fn list_builtin_servers(&self) -> Result<Vec<String>, AppError> {
        Ok(self.manager.read().await.list_builtin_servers().await)
    }

    pub async fn list_builtin_tools(&self) -> Result<Vec<McpRegisteredTool>, AppError> {
        Ok(self.manager.read().await.list_builtin_tools().await)
    }

    pub async fn call_builtin_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<ToolCallResult, AppError> {
        self.manager
            .read()
            .await
            .call_builtin_tool(server_name, tool_name, arguments)
            .await
    }

    pub async fn configure_builtin(
        &self,
        server_name: &str,
        config: serde_json::Value,
    ) -> Result<(), AppError> {
        self.manager
            .read()
            .await
            .configure_builtin(server_name, config)
            .await
    }

    // ===== D1 v3.1 Task 3.2.11: 工具调用历史 =====

    pub async fn list_call_history(&self) -> Result<Vec<McpToolCallHistoryEntry>, AppError> {
        Ok(self.manager.read().await.list_call_history().await)
    }
}