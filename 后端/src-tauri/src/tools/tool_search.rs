use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::tool::{
    ToolCallResult, ToolCategory, ToolDefinition, ToolError, ToolExample,
    ToolMatchType, ToolPermission, ToolSearchRequest, ToolSearchResponse,
    ToolSearchResult,
};
use super::RegisteredTool;

/// 工具搜索 - 对标 Codex 的 tool_search
#[derive(Clone)]
pub struct ToolSearchTool {
    tool_definitions: Arc<RwLock<Vec<ToolDefinition>>>,
}

impl ToolSearchTool {
    pub fn new() -> Self {
        Self {
            tool_definitions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn index_tools(&self, tools: Vec<ToolDefinition>) {
        let mut defs = self.tool_definitions.write().await;
        *defs = tools;
    }

    pub async fn add_tool(&self, tool: ToolDefinition) {
        self.tool_definitions.write().await.push(tool);
    }

    /// 搜索工具
    pub async fn search(&self, request: ToolSearchRequest) -> ToolSearchResponse {
        let query = request.query.to_lowercase();
        let defs = self.tool_definitions.read().await;
        let mut results: Vec<ToolSearchResult> = Vec::new();

        for tool in defs.iter() {
            if let Some(ref category) = request.category {
                if tool.category != *category {
                    continue;
                }
            }

            let name_lower = tool.name.to_lowercase();
            let desc_lower = tool.description.to_lowercase();
            let tags_lower: Vec<String> = tool.tags.iter().map(|t| t.to_lowercase()).collect();

            let mut score = 0.0f64;
            let mut match_type = ToolMatchType::DescriptionMatch;

            // 精确名称匹配
            if name_lower == query {
                score = 1.0;
                match_type = ToolMatchType::ExactName;
            }
            // 名称包含查询
            else if name_lower.contains(&query) {
                score = 0.8;
                match_type = ToolMatchType::NameContains;
            }
            // 描述包含查询
            else if desc_lower.contains(&query) {
                score = 0.5;
                match_type = ToolMatchType::DescriptionMatch;
            }

            // 标签匹配
            if tags_lower.iter().any(|t| t.contains(&query)) {
                score = score.max(0.6);
                match_type = ToolMatchType::TagMatch;
            }

            // 词语级匹配加分
            for word in query.split_whitespace() {
                if name_lower.contains(word) {
                    score += 0.1;
                }
                if desc_lower.contains(word) {
                    score += 0.05;
                }
            }

            if score > 0.0 {
                results.push(ToolSearchResult {
                    tool: tool.clone(),
                    relevance_score: score.min(1.0),
                    match_type,
                });
            }
        }

        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total_count = results.len();
        let max = request.max_results.unwrap_or(10);
        results.truncate(max);

        ToolSearchResponse {
            results,
            total_count,
        }
    }

    /// 获取可发现工具建议
    pub async fn suggest_discoverable(
        &self,
        user_query: &str,
        max_suggestions: usize,
    ) -> Vec<String> {
        let response = self
            .search(ToolSearchRequest {
                query: user_query.to_string(),
                category: None,
                max_results: Some(max_suggestions),
            })
            .await;

        response
            .results
            .into_iter()
            .map(|r| r.tool.name)
            .collect()
    }
}

#[async_trait]
impl RegisteredTool for ToolSearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "tool_search".into(),
            description: "搜索可用的工具。根据关键词、分类或标签查找匹配的工具，返回按相关性排序的结果。用于发现和了解可用的工具功能。".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "搜索关键词"
                    },
                    "category": {
                        "type": "string",
                        "enum": ["FileEdit", "Plan", "Shell", "Search", "Agent", "Info", "Other"],
                        "description": "工具分类过滤"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "最大返回结果数"
                    }
                },
                "required": ["query"]
            }),
            examples: vec![
                ToolExample {
                    description: "搜索文件编辑工具".into(),
                    arguments: serde_json::json!({
                        "query": "edit file",
                        "max_results": 5
                    }),
                },
            ],
            category: ToolCategory::Search,
            tags: vec!["search".into(), "discover".into(), "tools".into()],
        }
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission {
            requires_approval: false,
            sandbox_required: false,
            max_retries: 1,
            timeout_ms: 5_000,
            ..Default::default()
        }
    }

    fn clone_box(&self) -> Box<dyn RegisteredTool> {
        Box::new(self.clone())
    }

    async fn call(&self, arguments: serde_json::Value) -> Result<ToolCallResult, ToolError> {
        let query = arguments["query"].as_str().unwrap_or("");
        let max_results = arguments["max_results"].as_u64().unwrap_or(10) as usize;
        let category = arguments["category"].as_str().map(|c| match c {
            "FileEdit" => ToolCategory::FileEdit,
            "Plan" => ToolCategory::Plan,
            "Shell" => ToolCategory::Shell,
            "Search" => ToolCategory::Search,
            "Agent" => ToolCategory::Agent,
            "Info" => ToolCategory::Info,
            _ => ToolCategory::Other,
        });

        let response = self
            .search(ToolSearchRequest {
                query: query.to_string(),
                category,
                max_results: Some(max_results),
            })
            .await;

        let content = if response.results.is_empty() {
            format!("未找到匹配 '{}' 的工具", query)
        } else {
            let mut s = format!("搜索 '{}' 的结果 ({}/{}):\n\n", query, response.results.len(), response.total_count);
            for r in &response.results {
                s.push_str(&format!(
                    "- **{}** (相关性: {:.0}%)\n  {}\n",
                    r.tool.name,
                    r.relevance_score * 100.0,
                    r.tool.description
                ));
            }
            s
        };

        Ok(ToolCallResult {
            success: true,
            content,
            metadata: None,
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_tools() {
        let search = ToolSearchTool::new();
        search.index_tools(vec![
            ToolDefinition {
                name: "apply_patch".into(),
                description: "Apply unified diff patches to files".into(),
                input_schema: serde_json::json!({}),
                examples: vec![],
                category: ToolCategory::FileEdit,
                tags: vec!["edit".into(), "file".into()],
            },
            ToolDefinition {
                name: "update_plan".into(),
                description: "Create and manage task plans".into(),
                input_schema: serde_json::json!({}),
                examples: vec![],
                category: ToolCategory::Plan,
                tags: vec!["plan".into()],
            },
        ])
        .await;

        let response = search
            .search(ToolSearchRequest {
                query: "patch".into(),
                category: None,
                max_results: Some(5),
            })
            .await;

        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].tool.name, "apply_patch");
    }

    #[tokio::test]
    async fn test_suggest_discoverable() {
        let search = ToolSearchTool::new();
        search.index_tools(vec![
            ToolDefinition {
                name: "apply_patch".into(),
                description: "Apply patches".into(),
                input_schema: serde_json::json!({}),
                examples: vec![],
                category: ToolCategory::FileEdit,
                tags: vec![],
            },
        ])
        .await;

        let suggestions = search.suggest_discoverable("patch", 3).await;
        assert!(!suggestions.is_empty());
        assert!(suggestions.contains(&"apply_patch".to_string()));
    }
}