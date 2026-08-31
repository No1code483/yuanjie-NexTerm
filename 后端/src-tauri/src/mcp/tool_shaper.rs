use std::collections::HashMap;

use crate::models::mcp::McpRegisteredTool;

#[cfg(test)]
use crate::models::mcp::Tool;

/// 工具 Schema 塑形器
///
/// 负责：
/// - 工具名称规范化
/// - 去重（同名工具保留优先级更高的）
/// - 命名空间处理（server_name::tool_name）
pub struct ToolShaper {
    /// 命名空间分隔符
    pub namespace_separator: String,
    /// 是否启用命名空间
    pub enable_namespace: bool,
    /// 优先服务器列表（同名工具从这些服务器优先选择）
    pub preferred_servers: Vec<String>,
}

impl Default for ToolShaper {
    fn default() -> Self {
        Self {
            namespace_separator: "::".to_string(),
            enable_namespace: true,
            preferred_servers: Vec::new(),
        }
    }
}

impl ToolShaper {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置命名空间分隔符
    pub fn with_separator(mut self, sep: &str) -> Self {
        self.namespace_separator = sep.to_string();
        self
    }

    /// 设置是否启用命名空间
    pub fn with_namespace(mut self, enabled: bool) -> Self {
        self.enable_namespace = enabled;
        self
    }

    /// 设置优先服务器
    pub fn with_preferred_servers(mut self, servers: Vec<String>) -> Self {
        self.preferred_servers = servers;
        self
    }

    /// 规范化工具名称
    pub fn normalize_tool_name(name: &str) -> String {
        name.trim().to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect()
    }

    /// 为工具名添加命名空间
    pub fn namespace_tool_name(&self, server_name: &str, tool_name: &str) -> String {
        if self.enable_namespace {
            format!("{}{}{}", server_name, self.namespace_separator, tool_name)
        } else {
            tool_name.to_string()
        }
    }

    /// 从命名空间工具名中提取服务器名
    pub fn extract_server_name<'a>(&self, namespaced_name: &'a str) -> Option<&'a str> {
        if !self.enable_namespace {
            return None;
        }
        namespaced_name
            .split(&self.namespace_separator)
            .next()
    }

    /// 从命名空间工具名中提取原始工具名
    pub fn extract_tool_name<'a>(&self, namespaced_name: &'a str) -> Option<&'a str> {
        if !self.enable_namespace {
            return Some(namespaced_name);
        }
        namespaced_name
            .split(&self.namespace_separator)
            .nth(1)
    }

    /// 塑形工具列表：去重、命名空间、规范化
    ///
    /// 去重策略：
    /// 1. 优先服务器中的工具优先保留
    /// 2. 同名工具，服务器名字母序靠前的优先
    pub fn shape_tools(&self, tools: &[McpRegisteredTool]) -> Vec<McpRegisteredTool> {
        let mut deduped: HashMap<String, McpRegisteredTool> = HashMap::new();

        for tool in tools {
            let normalized_name = Self::normalize_tool_name(&tool.tool.name);

            if let Some(existing) = deduped.get(&normalized_name) {
                // 同名工具，按优先级决定是否替换
                if self.should_replace(&existing.server_id, &tool.server_id) {
                    deduped.insert(normalized_name, tool.clone());
                }
            } else {
                deduped.insert(normalized_name, tool.clone());
            }
        }

        // 转换为带命名空间的工具
        deduped
            .into_values()
            .map(|mut t| {
                let namespaced = self.namespace_tool_name(&t.server_name, &t.tool.name);
                let mut shaped_tool = t.tool.clone();
                shaped_tool.name = namespaced;
                t.tool = shaped_tool;
                t
            })
            .collect()
    }

    /// 判断同名工具是否应该替换现有的
    fn should_replace(&self, existing_server: &str, new_server: &str) -> bool {
        // 优先服务器优先
        let existing_priority = self
            .preferred_servers
            .iter()
            .position(|s| s == existing_server)
            .map(|p| p as i32)
            .unwrap_or(i32::MAX);

        let new_priority = self
            .preferred_servers
            .iter()
            .position(|s| s == new_server)
            .map(|p| p as i32)
            .unwrap_or(i32::MAX);

        if new_priority < existing_priority {
            return true;
        }
        if new_priority > existing_priority {
            return false;
        }

        // 同优先级，字母序靠前的优先
        new_server < existing_server
    }

    /// 按命名空间分组工具
    pub fn group_by_namespace(
        &self,
        tools: &[McpRegisteredTool],
    ) -> HashMap<String, Vec<McpRegisteredTool>> {
        let mut groups: HashMap<String, Vec<McpRegisteredTool>> = HashMap::new();

        for tool in tools {
            let namespace = self
                .extract_server_name(&tool.tool.name)
                .unwrap_or(&tool.server_name)
                .to_string();

            groups.entry(namespace).or_default().push(tool.clone());
        }

        groups
    }

    /// 获取所有命名空间
    pub fn get_namespaces(&self, tools: &[McpRegisteredTool]) -> Vec<String> {
        let mut namespaces: Vec<String> = tools
            .iter()
            .map(|t| {
                self.extract_server_name(&t.tool.name)
                    .unwrap_or(&t.server_name)
                    .to_string()
            })
            .collect();

        namespaces.sort();
        namespaces.dedup();
        namespaces
    }

    /// 过滤指定命名空间的工具
    pub fn filter_by_namespace(
        &self,
        tools: &[McpRegisteredTool],
        namespace: &str,
    ) -> Vec<McpRegisteredTool> {
        tools
            .iter()
            .filter(|t| {
                self.extract_server_name(&t.tool.name)
                    .map(|n| n == namespace)
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// 验证工具名称是否合法
    pub fn is_valid_tool_name(name: &str) -> bool {
        if name.is_empty() {
            return false;
        }
        let normalized = Self::normalize_tool_name(name);
        !normalized.is_empty() && normalized == name.to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool(server_id: &str, server_name: &str, tool_name: &str) -> McpRegisteredTool {
        McpRegisteredTool {
            server_id: server_id.to_string(),
            server_name: server_name.to_string(),
            tool: Tool {
                name: tool_name.to_string(),
                description: "test".to_string(),
                input_schema: serde_json::json!({}),
            },
        }
    }

    #[test]
    fn test_normalize_tool_name() {
        assert_eq!(
            ToolShaper::normalize_tool_name("My Tool!"),
            "mytool"
        );
        assert_eq!(
            ToolShaper::normalize_tool_name("  hello_world  "),
            "hello_world"
        );
    }

    #[test]
    fn test_namespace_tool_name() {
        let shaper = ToolShaper::new();
        assert_eq!(
            shaper.namespace_tool_name("server_a", "my_tool"),
            "server_a::my_tool"
        );
    }

    #[test]
    fn test_dedup_by_preferred_server() {
        let shaper = ToolShaper::new()
            .with_preferred_servers(vec!["server_b".to_string()]);

        let tools = vec![
            make_tool("server_a", "Server A", "my_tool"),
            make_tool("server_b", "Server B", "my_tool"),
        ];

        let shaped = shaper.shape_tools(&tools);
        assert_eq!(shaped.len(), 1);
        assert_eq!(shaped[0].server_id, "server_b");
        assert_eq!(shaped[0].tool.name, "Server B::my_tool");
    }

    #[test]
    fn test_group_by_namespace() {
        let shaper = ToolShaper::new();
        let tools = vec![
            make_tool("s1", "Server A", "tool_a"),
            make_tool("s2", "Server B", "tool_b"),
        ];

        let groups = shaper.group_by_namespace(&tools);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_is_valid_tool_name() {
        assert!(ToolShaper::is_valid_tool_name("hello_world"));
        assert!(!ToolShaper::is_valid_tool_name(""));
        assert!(!ToolShaper::is_valid_tool_name("Hello World"));
    }
}