use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::tool::{
    Plan, PlanStep, PlanStepStatus, ToolCallResult, ToolCategory,
    ToolDefinition, ToolError, ToolExample, ToolPermission,
};
use super::RegisteredTool;

/// Plan 工具 - 对标 Codex 的 update_plan
pub struct PlanTool {
    plans: Arc<RwLock<Vec<Plan>>>,
}

impl PlanTool {
    pub fn new() -> Self {
        Self {
            plans: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn create_plan(
        &self,
        title: &str,
        description: &str,
        steps: Vec<PlanStep>,
        session_id: &str,
    ) -> Result<Plan, ToolError> {
        let plan = Plan {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            description: description.to_string(),
            steps,
            status: PlanStepStatus::Pending,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            updated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            session_id: session_id.to_string(),
        };

        self.plans.write().await.push(plan.clone());
        Ok(plan)
    }

    pub async fn update_step(
        &self,
        plan_id: &str,
        step_id: &str,
        status: PlanStepStatus,
        result: Option<String>,
    ) -> Result<Plan, ToolError> {
        let mut plans = self.plans.write().await;
        let plan = plans
            .iter_mut()
            .find(|p| p.id == plan_id)
            .ok_or(ToolError {
                code: "PLAN_NOT_FOUND".into(),
                message: format!("计划不存在: {}", plan_id),
                details: None,
            })?;

        let step = plan
            .steps
            .iter_mut()
            .find(|s| s.id == step_id)
            .ok_or(ToolError {
                code: "STEP_NOT_FOUND".into(),
                message: format!("步骤不存在: {}", step_id),
                details: None,
            })?;

        step.status = status;
        if let Some(r) = result {
            step.result = Some(r);
        }
        step.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 更新计划状态
        let all_completed = plan.steps.iter().all(|s| s.status == PlanStepStatus::Completed);
        let any_failed = plan.steps.iter().any(|s| s.status == PlanStepStatus::Failed);
        plan.status = if all_completed {
            PlanStepStatus::Completed
        } else if any_failed {
            PlanStepStatus::Failed
        } else {
            PlanStepStatus::InProgress
        };
        plan.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(plan.clone())
    }

    pub async fn get_plan(&self, plan_id: &str) -> Option<Plan> {
        self.plans.read().await.iter().find(|p| p.id == plan_id).cloned()
    }

    pub async fn list_plans(&self, session_id: Option<&str>) -> Vec<Plan> {
        let plans = self.plans.read().await;
        if let Some(sid) = session_id {
            plans.iter().filter(|p| p.session_id == sid).cloned().collect()
        } else {
            plans.clone()
        }
    }

    pub async fn format_plan_summary(&self, plan_id: &str) -> Result<String, ToolError> {
        let plan = self.get_plan(plan_id).await.ok_or(ToolError {
            code: "PLAN_NOT_FOUND".into(),
            message: format!("计划不存在: {}", plan_id),
            details: None,
        })?;

        let mut summary = format!(
            "# 计划: {}\n\n{}\n\n状态: {:?}\n\n",
            plan.title, plan.description, plan.status
        );

        for step in &plan.steps {
            let status_icon = match step.status {
                PlanStepStatus::Completed => "✅",
                PlanStepStatus::InProgress => "🔄",
                PlanStepStatus::Failed => "❌",
                PlanStepStatus::Skipped => "⏭️",
                PlanStepStatus::Pending => "⏳",
            };
            summary.push_str(&format!(
                "- {} {}: {}\n",
                status_icon, step.title, step.description
            ));
            if let Some(ref result) = step.result {
                summary.push_str(&format!("  → {}\n", result));
            }
        }

        Ok(summary)
    }
}

#[async_trait]
impl RegisteredTool for PlanTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "update_plan".into(),
            description: "创建和管理任务执行计划。支持创建计划、更新步骤状态、查看计划摘要。用于追踪复杂多步骤任务的进度。".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["create", "update_step", "list", "summary"],
                        "description": "操作类型"
                    },
                    "plan_id": {
                        "type": "string",
                        "description": "计划 ID（update_step/list/summary 时使用）"
                    },
                    "title": {
                        "type": "string",
                        "description": "计划标题（create 时使用）"
                    },
                    "description": {
                        "type": "string",
                        "description": "计划描述"
                    },
                    "steps": {
                        "type": "array",
                        "description": "步骤列表"
                    },
                    "step_id": {
                        "type": "string",
                        "description": "步骤 ID（update_step 时使用）"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["pending", "in_progress", "completed", "failed", "skipped"],
                        "description": "步骤状态"
                    },
                    "result": {
                        "type": "string",
                        "description": "步骤执行结果"
                    }
                },
                "required": ["action"]
            }),
            examples: vec![
                ToolExample {
                    description: "创建执行计划".into(),
                    arguments: serde_json::json!({
                        "action": "create",
                        "title": "实现用户认证",
                        "description": "添加 JWT 认证系统",
                        "steps": [
                            {"id": "s1", "title": "创建用户模型", "description": "..."},
                            {"id": "s2", "title": "实现登录接口", "description": "..."}
                        ]
                    }),
                },
            ],
            category: ToolCategory::Plan,
            tags: vec!["plan".into(), "task".into(), "tracking".into()],
        }
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission {
            requires_approval: false,
            sandbox_required: false,
            max_retries: 1,
            timeout_ms: 10_000,
            ..Default::default()
        }
    }

    fn clone_box(&self) -> Box<dyn RegisteredTool> {
        Box::new(Self {
            plans: self.plans.clone(),
        })
    }

    async fn call(&self, arguments: serde_json::Value) -> Result<ToolCallResult, ToolError> {
        let action = arguments["action"].as_str().unwrap_or("list");
        let session_id = arguments["session_id"].as_str().unwrap_or("default");

        match action {
            "create" => {
                let title = arguments["title"].as_str().unwrap_or("未命名计划");
                let description = arguments["description"].as_str().unwrap_or("");
                let steps: Vec<PlanStep> = serde_json::from_value(
                    arguments["steps"].clone(),
                )
                .unwrap_or_default();

                let plan = self.create_plan(title, description, steps, session_id).await?;
                Ok(ToolCallResult {
                    success: true,
                    content: format!("计划已创建: {} ({})", plan.title, plan.id),
                    metadata: None,
                    error: None,
                })
            }
            "update_step" => {
                let plan_id = arguments["plan_id"].as_str().ok_or(ToolError {
                    code: "MISSING_ARGUMENT".into(),
                    message: "缺少 plan_id".into(),
                    details: None,
                })?;
                let step_id = arguments["step_id"].as_str().ok_or(ToolError {
                    code: "MISSING_ARGUMENT".into(),
                    message: "缺少 step_id".into(),
                    details: None,
                })?;
                let status_str = arguments["status"].as_str().unwrap_or("completed");
                let status = match status_str {
                    "pending" => PlanStepStatus::Pending,
                    "in_progress" => PlanStepStatus::InProgress,
                    "completed" => PlanStepStatus::Completed,
                    "failed" => PlanStepStatus::Failed,
                    "skipped" => PlanStepStatus::Skipped,
                    _ => return Err(ToolError {
                        code: "INVALID_STATUS".into(),
                        message: format!("无效状态: {}", status_str),
                        details: None,
                    }),
                };
                let result = arguments["result"].as_str().map(|s| s.to_string());

                self.update_step(plan_id, step_id, status, result).await?;
                Ok(ToolCallResult {
                    success: true,
                    content: format!("步骤 {} 已更新为 {:?}", step_id, status_str),
                    metadata: None,
                    error: None,
                })
            }
            "summary" => {
                let plan_id = arguments["plan_id"].as_str().ok_or(ToolError {
                    code: "MISSING_ARGUMENT".into(),
                    message: "缺少 plan_id".into(),
                    details: None,
                })?;
                let summary = self.format_plan_summary(plan_id).await?;
                Ok(ToolCallResult {
                    success: true,
                    content: summary,
                    metadata: None,
                    error: None,
                })
            }
            _ => {
                let plans = self.list_plans(Some(session_id)).await;
                let content = if plans.is_empty() {
                    "暂无计划".to_string()
                } else {
                    plans
                        .iter()
                        .map(|p| format!("- [{}] {} ({:?})", p.id, p.title, p.status))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                Ok(ToolCallResult {
                    success: true,
                    content,
                    metadata: None,
                    error: None,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_plan() {
        let tool = PlanTool::new();
        let steps = vec![PlanStep {
            id: "s1".into(),
            title: "Step 1".into(),
            description: "First step".into(),
            status: PlanStepStatus::Pending,
            dependencies: vec![],
            assigned_agent: None,
            result: None,
            created_at: 0,
            updated_at: 0,
        }];

        let plan = tool.create_plan("Test", "Test plan", steps, "test-session").await.unwrap();
        assert_eq!(plan.title, "Test");
        assert_eq!(plan.steps.len(), 1);
    }

    #[tokio::test]
    async fn test_update_step() {
        let tool = PlanTool::new();
        let steps = vec![PlanStep {
            id: "s1".into(),
            title: "Step 1".into(),
            description: "First step".into(),
            status: PlanStepStatus::Pending,
            dependencies: vec![],
            assigned_agent: None,
            result: None,
            created_at: 0,
            updated_at: 0,
        }];

        let plan = tool.create_plan("Test", "Test", steps, "s").await.unwrap();
        let updated = tool
            .update_step(&plan.id, "s1", PlanStepStatus::Completed, Some("Done".into()))
            .await
            .unwrap();

        assert_eq!(updated.steps[0].status, PlanStepStatus::Completed);
        assert_eq!(updated.status, PlanStepStatus::Completed);
    }
}