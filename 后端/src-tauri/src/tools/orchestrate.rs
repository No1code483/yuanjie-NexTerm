use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::models::tool::{
    ApprovalState, ToolCallRequest, ToolCallResult,
};

use super::RegisteredTool;

/// 工具编排器 - 审批 → 沙箱 → 重试 → 降级 四阶段
pub struct ToolOrchestrator {
    approval_required: bool,
    sandbox_enabled: bool,
    max_retries: u32,
    fallback_registry: Arc<RwLock<Vec<Box<dyn RegisteredTool>>>>,
}

impl ToolOrchestrator {
    pub fn new() -> Self {
        Self {
            approval_required: true,
            sandbox_enabled: true,
            max_retries: 3,
            fallback_registry: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_approval(mut self, required: bool) -> Self {
        self.approval_required = required;
        self
    }

    pub fn with_sandbox(mut self, enabled: bool) -> Self {
        self.sandbox_enabled = enabled;
        self
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// 执行工具调用，经过完整的编排流程
    pub async fn execute(
        &self,
        tool: &dyn RegisteredTool,
        request: ToolCallRequest,
    ) -> Result<ToolCallResult, AppError> {
        let permission = tool.permission();
        let definition = tool.definition();

        // 阶段 1: 审批
        if self.approval_required && permission.requires_approval {
            if let Some(ref action) = request.approval_action {
                if !action.approved {
                    return Err(AppError::Internal(format!(
                        "工具 '{}' 需要审批，但被拒绝: {}",
                        definition.name,
                        action.reason.as_deref().unwrap_or("无原因")
                    )));
                }
            } else {
                // 需要审批但未提供审批动作
                return Err(AppError::Internal(format!(
                    "工具 '{}' 需要审批 (级别: {:?})",
                    definition.name, permission.approval_level
                )));
            }
        }

        // 阶段 2: 沙箱执行
        if self.sandbox_enabled && permission.sandbox_required {
            // 沙箱包装由上层沙箱模块处理，这里仅是标记
        }

        // 阶段 3: 重试
        let mut last_error = None;
        for attempt in 0..=permission.max_retries.min(self.max_retries) {
            match tool.call(request.arguments.clone()).await {
                Ok(result) => {
                    return Ok(result);
                }
                Err(e) => {
                    if attempt < permission.max_retries.min(self.max_retries) {
                        // 指数退避
                        let delay = std::time::Duration::from_millis(
                            100 * 2u64.pow(attempt),
                        );
                        tokio::time::sleep(delay).await;
                        last_error = Some(e);
                    } else {
                        last_error = Some(e);
                    }
                }
            }
        }

        // 阶段 4: 降级
        if let Some(error) = last_error {
            if let Some(fallback) = self.find_fallback(&definition.name).await {
                return fallback.call(request.arguments).await.map_err(|e| {
                    AppError::Internal(format!(
                        "工具 '{}' 执行失败，降级也失败: {}",
                        definition.name, e.message
                    ))
                });
            }
            return Err(AppError::Internal(format!(
                "工具 '{}' 执行失败: {}",
                definition.name, error.message
            )));
        }

        Err(AppError::Internal(format!(
            "工具 '{}' 执行失败（未知错误）",
            definition.name
        )))
    }

    /// 创建审批状态
    pub fn create_approval_state(
        &self,
        tool: &dyn RegisteredTool,
        request: &ToolCallRequest,
    ) -> Option<ApprovalState> {
        let permission = tool.permission();
        if !permission.requires_approval {
            return None;
        }

        Some(ApprovalState {
            pending: true,
            level: permission.approval_level.clone(),
            reason: format!(
                "工具 '{}' 需要审批才能执行: {}",
                tool.definition().name,
                tool.definition().description
            ),
            request_id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool.definition().name.clone(),
            arguments: request.arguments.clone(),
        })
    }

    async fn find_fallback(&self, tool_name: &str) -> Option<Box<dyn RegisteredTool>> {
        let registry = self.fallback_registry.read().await;
        for tool in registry.iter() {
            if tool.definition().name == tool_name {
                return Some(tool.clone_box());
            }
        }
        None
    }
}

impl Default for ToolOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tool::{
        ApprovalAction, ApprovalLevel, ToolCategory, ToolDefinition, ToolError, ToolExample,
        ToolPermission,
    };

    struct MockTool {
        should_fail: bool,
        fail_count: std::sync::atomic::AtomicU32,
    }

    impl MockTool {
        fn new() -> Self {
            Self {
                should_fail: false,
                fail_count: std::sync::atomic::AtomicU32::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl RegisteredTool for MockTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "mock_tool".into(),
                description: "Mock tool".into(),
                input_schema: serde_json::json!({}),
                examples: vec![],
                category: ToolCategory::Other,
                tags: vec![],
            }
        }

        fn permission(&self) -> ToolPermission {
            ToolPermission {
                requires_approval: true,
                approval_level: ApprovalLevel::Low,
                sandbox_required: false,
                max_retries: 2,
                timeout_ms: 1000,
                rate_limit_per_minute: None,
            }
        }

        fn clone_box(&self) -> Box<dyn RegisteredTool> {
            Box::new(Self {
                should_fail: self.should_fail,
                fail_count: std::sync::atomic::AtomicU32::new(0),
            })
        }

        async fn call(&self, _arguments: serde_json::Value) -> Result<ToolCallResult, ToolError> {
            if self.should_fail {
                Err(ToolError {
                    code: "MOCK_ERROR".into(),
                    message: "Mock failure".into(),
                    details: None,
                })
            } else {
                Ok(ToolCallResult {
                    success: true,
                    content: "ok".into(),
                    metadata: None,
                    error: None,
                })
            }
        }
    }

    #[tokio::test]
    async fn test_orchestrator_approval_required() {
        let orchestrator = ToolOrchestrator::new().with_approval(true);
        let tool = MockTool::new();

        let request = ToolCallRequest {
            tool_name: "mock_tool".into(),
            arguments: serde_json::json!({}),
            session_id: None,
            approval_action: None,
        };

        let result = orchestrator.execute(&tool, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_orchestrator_with_approval() {
        let orchestrator = ToolOrchestrator::new().with_approval(true);
        let tool = MockTool::new();

        let request = ToolCallRequest {
            tool_name: "mock_tool".into(),
            arguments: serde_json::json!({}),
            session_id: None,
            approval_action: Some(ApprovalAction {
                approved: true,
                reason: Some("test".into()),
                timestamp: 0,
            }),
        };

        let result = orchestrator.execute(&tool, request).await;
        assert!(result.is_ok());
    }
}