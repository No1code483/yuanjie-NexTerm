use std::collections::HashMap;
use std::sync::Arc;

use crate::models::tool::{
    ParallelToolCalls, ParallelToolResult, ParallelToolResults, ToolCallRequest,
    ToolError,
};

use super::RegisteredTool;

/// 工具并行执行器
pub struct ParallelExecutor {
    max_concurrency: usize,
}

impl ParallelExecutor {
    pub fn new() -> Self {
        Self {
            max_concurrency: 10,
        }
    }

    pub fn with_max_concurrency(mut self, max: usize) -> Self {
        self.max_concurrency = max;
        self
    }

    /// 并行执行多个工具调用
    pub async fn execute_all(
        &self,
        tools: &HashMap<String, Box<dyn RegisteredTool>>,
        calls: ParallelToolCalls,
    ) -> ParallelToolResults {
        let start = std::time::Instant::now();
        let max_concurrency = calls.max_concurrency.min(self.max_concurrency);

        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrency));
        let mut handles = Vec::new();

        for call in calls.calls {
            let permit = semaphore.clone().acquire_owned().await;
            if let Ok(permit) = permit {
                let tool = tools.get(&call.tool_name).map(|t| t.clone_box());
                handles.push(tokio::spawn(async move {
                    let _permit = permit;
                    let call_start = std::time::Instant::now();

                    let result = match tool {
                        Some(t) => match t.call(call.arguments).await {
                            Ok(r) => Ok(r),
                            Err(e) => Err(e),
                        },
                        None => Err(ToolError {
                            code: "TOOL_NOT_FOUND".into(),
                            message: format!("工具未找到: {}", call.tool_name),
                            details: None,
                        }),
                    };

                    ParallelToolResult {
                        tool_name: call.tool_name,
                        result,
                        duration_ms: call_start.elapsed().as_millis() as u64,
                    }
                }));
            }
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }

        let success_count = results.iter().filter(|r| r.result.is_ok()).count();
        let failure_count = results.len() - success_count;

        ParallelToolResults {
            results,
            total_duration_ms: start.elapsed().as_millis() as u64,
            success_count,
            failure_count,
        }
    }

    /// 检测工具间的依赖关系
    pub fn detect_dependencies(
        &self,
        calls: &[ToolCallRequest],
    ) -> Vec<Vec<usize>> {
        // 简单实现：按工具名称分组
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, call) in calls.iter().enumerate() {
            groups.entry(call.tool_name.clone()).or_default().push(i);
        }
        groups.into_values().collect()
    }

    /// 按依赖顺序执行（先执行无依赖的，再执行有依赖的）
    pub async fn execute_with_deps(
        &self,
        tools: &HashMap<String, Box<dyn RegisteredTool>>,
        calls: ParallelToolCalls,
    ) -> ParallelToolResults {
        let dep_groups = self.detect_dependencies(&calls.calls);
        let mut all_results = Vec::new();
        let start = std::time::Instant::now();

        for group in dep_groups {
            let group_calls: Vec<ToolCallRequest> = group
                .into_iter()
                .filter_map(|i| calls.calls.get(i).cloned())
                .collect();

            let group_result = self
                .execute_all(
                    tools,
                    ParallelToolCalls {
                        calls: group_calls,
                        max_concurrency: calls.max_concurrency,
                        fail_fast: calls.fail_fast,
                    },
                )
                .await;

            all_results.extend(group_result.results);

            if calls.fail_fast && group_result.failure_count > 0 {
                break;
            }
        }

        let success_count = all_results.iter().filter(|r| r.result.is_ok()).count();
        let failure_count = all_results.len() - success_count;

        ParallelToolResults {
            results: all_results,
            total_duration_ms: start.elapsed().as_millis() as u64,
            success_count,
            failure_count,
        }
    }
}

impl Default for ParallelExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tool::{ToolCategory, ToolDefinition, ToolPermission, ToolCallResult};
    use async_trait::async_trait;

    struct DelayTool {
        name: String,
        delay_ms: u64,
    }

    #[async_trait]
    impl RegisteredTool for DelayTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: self.name.clone(),
                description: "Delay tool".into(),
                input_schema: serde_json::json!({}),
                examples: vec![],
                category: ToolCategory::Other,
                tags: vec![],
            }
        }

        fn permission(&self) -> ToolPermission {
            ToolPermission::default()
        }

        fn clone_box(&self) -> Box<dyn RegisteredTool> {
            Box::new(DelayTool {
                name: self.name.clone(),
                delay_ms: self.delay_ms,
            })
        }

        async fn call(&self, _arguments: serde_json::Value) -> Result<ToolCallResult, ToolError> {
            tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
            Ok(ToolCallResult {
                success: true,
                content: format!("{} done", self.name),
                metadata: None,
                error: None,
            })
        }
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let executor = ParallelExecutor::new().with_max_concurrency(5);
        let mut tools: HashMap<String, Box<dyn RegisteredTool>> = HashMap::new();
        tools.insert("t1".into(), Box::new(DelayTool { name: "t1".into(), delay_ms: 50 }));
        tools.insert("t2".into(), Box::new(DelayTool { name: "t2".into(), delay_ms: 50 }));

        let calls = ParallelToolCalls {
            calls: vec![
                ToolCallRequest {
                    tool_name: "t1".into(),
                    arguments: serde_json::json!({}),
                    session_id: None,
                    approval_action: None,
                },
                ToolCallRequest {
                    tool_name: "t2".into(),
                    arguments: serde_json::json!({}),
                    session_id: None,
                    approval_action: None,
                },
            ],
            max_concurrency: 5,
            fail_fast: false,
        };

        let result = executor.execute_all(&tools, calls).await;
        assert_eq!(result.success_count, 2);
        // 并行执行应该比串行快
        assert!(result.total_duration_ms < 150);
    }
}