use crate::models::mcp::McpRegisteredTool;

/// 工具过滤器
///
/// 支持：
/// - 启用/禁用工具列表
/// - 工具搜索（名称/描述）
/// - 模式匹配（前缀/包含/正则）
pub struct ToolFilter {
    /// 启用的工具名列表（为空表示全部启用）
    enabled_tools: Vec<String>,
    /// 禁用的工具名列表
    disabled_tools: Vec<String>,
    /// 搜索模式
    search_mode: SearchMode,
}

/// 搜索模式
#[derive(Debug, Clone, PartialEq)]
pub enum SearchMode {
    /// 精确匹配
    Exact,
    /// 前缀匹配
    Prefix,
    /// 包含匹配
    Contains,
    /// 模糊匹配（忽略大小写）
    Fuzzy,
}

impl Default for ToolFilter {
    fn default() -> Self {
        Self {
            enabled_tools: Vec::new(),
            disabled_tools: Vec::new(),
            search_mode: SearchMode::Contains,
        }
    }
}

impl ToolFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置启用的工具列表
    pub fn set_enabled_tools(&mut self, tools: Vec<String>) {
        self.enabled_tools = tools;
    }

    /// 设置禁用的工具列表
    pub fn set_disabled_tools(&mut self, tools: Vec<String>) {
        self.disabled_tools = tools;
    }

    /// 添加启用工具
    pub fn enable_tool(&mut self, tool_name: &str) {
        self.disabled_tools.retain(|t| t != tool_name);
        if !self.enabled_tools.contains(&tool_name.to_string()) {
            self.enabled_tools.push(tool_name.to_string());
        }
    }

    /// 添加禁用工具
    pub fn disable_tool(&mut self, tool_name: &str) {
        self.enabled_tools.retain(|t| t != tool_name);
        if !self.disabled_tools.contains(&tool_name.to_string()) {
            self.disabled_tools.push(tool_name.to_string());
        }
    }

    /// 设置搜索模式
    pub fn set_search_mode(&mut self, mode: SearchMode) {
        self.search_mode = mode;
    }

    /// 应用过滤（启用/禁用）
    pub fn apply_enabled_filter(&self, tools: &[McpRegisteredTool]) -> Vec<McpRegisteredTool> {
        tools
            .iter()
            .filter(|t| self.is_tool_enabled(&t.tool.name))
            .cloned()
            .collect()
    }

    /// 检查工具是否启用
    fn is_tool_enabled(&self, tool_name: &str) -> bool {
        // 如果在禁用列表中，直接返回 false
        if self.disabled_tools.iter().any(|d| d == tool_name) {
            return false;
        }

        // 如果启用列表为空，全部启用
        if self.enabled_tools.is_empty() {
            return true;
        }

        // 在启用列表中
        self.enabled_tools.iter().any(|e| e == tool_name)
    }

    /// 搜索工具
    pub fn search_tools(
        &self,
        tools: &[McpRegisteredTool],
        query: &str,
    ) -> Vec<McpRegisteredTool> {
        if query.is_empty() {
            return tools.to_vec();
        }

        let filtered = self.apply_enabled_filter(tools);
        let query_lower = query.to_lowercase();

        filtered
            .into_iter()
            .filter(|t| self.matches_search(&t.tool.name, &t.tool.description, &query_lower))
            .collect()
    }

    /// 检查工具是否匹配搜索条件
    fn matches_search(&self, name: &str, description: &str, query: &str) -> bool {
        let name_lower = name.to_lowercase();
        let desc_lower = description.to_lowercase();

        match self.search_mode {
            SearchMode::Exact => name_lower == *query || desc_lower == *query,
            SearchMode::Prefix => name_lower.starts_with(query) || desc_lower.starts_with(query),
            SearchMode::Contains => name_lower.contains(query) || desc_lower.contains(query),
            SearchMode::Fuzzy => {
                // 模糊匹配：逐个字符检查
                Self::fuzzy_match(&name_lower, query)
                    || Self::fuzzy_match(&desc_lower, query)
            }
        }
    }

    /// 简单模糊匹配：检查 query 的所有字符是否按顺序出现在 target 中
    fn fuzzy_match(target: &str, query: &str) -> bool {
        let mut target_chars = target.chars();
        for qc in query.chars() {
            loop {
                match target_chars.next() {
                    Some(tc) if tc == qc => break,
                    Some(_) => continue,
                    None => return false,
                }
            }
        }
        true
    }

    /// 获取启用的工具列表
    pub fn get_enabled_tools(&self) -> &[String] {
        &self.enabled_tools
    }

    /// 获取禁用的工具列表
    pub fn get_disabled_tools(&self) -> &[String] {
        &self.disabled_tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::mcp::Tool;

    fn make_tool(name: &str, desc: &str) -> McpRegisteredTool {
        McpRegisteredTool {
            server_id: "test".to_string(),
            server_name: "Test".to_string(),
            tool: Tool {
                name: name.to_string(),
                description: desc.to_string(),
                input_schema: serde_json::json!({}),
            },
        }
    }

    #[test]
    fn test_enable_disable() {
        let mut filter = ToolFilter::new();
        let tools = vec![
            make_tool("tool_a", "desc a"),
            make_tool("tool_b", "desc b"),
            make_tool("tool_c", "desc c"),
        ];

        filter.disable_tool("tool_b");
        let result = filter.apply_enabled_filter(&tools);
        assert_eq!(result.len(), 2);

        filter.enable_tool("tool_a");
        filter.enable_tool("tool_c");
        let result = filter.apply_enabled_filter(&tools);
        assert_eq!(result.len(), 2); // tool_b still disabled
    }

    #[test]
    fn test_search_contains() {
        let filter = ToolFilter::new();
        let tools = vec![
            make_tool("read_file", "Read a file"),
            make_tool("write_file", "Write to a file"),
            make_tool("search_code", "Search codebase"),
        ];

        let result = filter.search_tools(&tools, "file");
        assert_eq!(result.len(), 2);

        let result = filter.search_tools(&tools, "code");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_fuzzy_match() {
        let filter = ToolFilter {
            search_mode: SearchMode::Fuzzy,
            ..Default::default()
        };
        let tools = vec![
            make_tool("read_file", "Read a file"),
            make_tool("search_code", "Search codebase"),
        ];

        let result = filter.search_tools(&tools, "rdf");
        assert_eq!(result.len(), 1); // "read_file" matches "rdf"
    }
}