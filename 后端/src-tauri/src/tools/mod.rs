pub mod apply_patch;
pub mod orchestrate;
pub mod parallel;
pub mod plan;
pub mod tool_search;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::models::tool::{
    ToolCallRequest, ToolCallResult, ToolDefinition, ToolError,
    ToolInfo, ToolPermission,
};

pub use self::apply_patch::ApplyPatchTool;
pub use self::plan::PlanTool;
use self::tool_search::ToolSearchTool;

/// 工具注册中心
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Box<dyn RegisteredTool>>>>,
    #[allow(dead_code)]
    search_tool: Arc<ToolSearchTool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let search_tool = Arc::new(ToolSearchTool::new());
        let mut registry = Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            search_tool: search_tool.clone(),
        };

        // 注册内置工具
        registry.register(Box::new(ApplyPatchTool::new()));
        registry.register(Box::new(PlanTool::new()));

        // 注册搜索工具为特殊工具
        let search_name = search_tool.definition().name.clone();
        registry.tools.try_write().unwrap().insert(
            search_name,
            Box::new(search_tool.as_ref().clone()),
        );

        registry
    }

    pub fn register(&mut self, tool: Box<dyn RegisteredTool>) {
        let name = tool.definition().name.clone();
        self.tools.try_write().unwrap().insert(name, tool);
    }

    pub fn get(&self, name: &str) -> Option<ToolInfo> {
        self.tools
            .try_read()
            .ok()?
            .get(name)
            .map(|t| ToolInfo {
                definition: t.definition(),
                permission: t.permission(),
            })
    }

    pub fn list_all(&self) -> Vec<ToolInfo> {
        self.tools
            .try_read()
            .map(|tools| {
                tools
                    .values()
                    .map(|t| ToolInfo {
                        definition: t.definition(),
                        permission: t.permission(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn list_definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .try_read()
            .map(|tools| tools.values().map(|t| t.definition()).collect())
            .unwrap_or_default()
    }

    pub async fn call(&self, request: ToolCallRequest) -> Result<ToolCallResult, AppError> {
        let tool = {
            let tools = self.tools.read().await;
            tools
                .get(&request.tool_name)
                .map(|t| t.as_ref().clone_box())
                .ok_or(AppError::NotFound)?
        };

        tool.call(request.arguments).await.map_err(|e| {
            AppError::Internal(format!("工具 '{}' 执行失败: {}", request.tool_name, e.message))
        })
    }
}

/// 已注册的工具 trait
#[async_trait::async_trait]
pub trait RegisteredTool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    fn permission(&self) -> ToolPermission;
    fn clone_box(&self) -> Box<dyn RegisteredTool>;
    async fn call(&self, arguments: serde_json::Value) -> Result<ToolCallResult, ToolError>;
}

/// 将 ToolRegistry 作为搜索工具的数据源
impl ToolSearchTool {
    pub async fn sync_from_registry(&self, registry: &ToolRegistry) {
        let definitions = registry.list_definitions();
        self.index_tools(definitions).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_create() {
        let registry = ToolRegistry::new();
        let tools = registry.list_all();
        // 默认包含 apply_patch, plan, tool_search
        assert!(tools.len() >= 3);
    }

    #[test]
    fn test_registry_get_tool() {
        let registry = ToolRegistry::new();
        let tool = registry.get("apply_patch");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().definition.name, "apply_patch");
    }
}