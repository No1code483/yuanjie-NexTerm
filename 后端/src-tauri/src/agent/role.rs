//! Agent 角色配置 — 对标 Codex agent/role.rs
//!
//! 角色系统在 Session 创建时为 Agent 注入特定的系统提示词、模型配置、
//! 工具集和权限边界。支持内置角色 + 用户自定义角色。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 默认角色名
pub const DEFAULT_ROLE_NAME: &str = "default";

/// Agent 角色名列表
pub const BUILT_IN_ROLE_NAMES: &[&str] = &[
    "default",
    "architect",
    "coder",
    "reviewer",
    "debugger",
    "planner",
    "devops",
    "security",
];

/// Agent 角色配置 — 对标 Codex AgentRoleConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRoleConfig {
    /// 角色名称
    pub name: String,
    /// 角色描述
    pub description: String,
    /// 系统提示词（覆盖默认）
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// 追加的系统提示词（拼接到默认之后）
    #[serde(default)]
    pub system_prompt_append: Option<String>,
    /// 模型名称（覆盖默认）
    #[serde(default)]
    pub model: Option<String>,
    /// 模型提供商（覆盖默认）
    #[serde(default)]
    pub model_provider: Option<String>,
    /// 温度参数
    #[serde(default)]
    pub temperature: Option<f32>,
    /// 最大 Token 数
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// 允许的工具列表（空 = 全部允许）
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// 禁用的工具列表
    #[serde(default)]
    pub denied_tools: Vec<String>,
    /// 是否启用沙箱
    #[serde(default)]
    pub sandbox_enabled: Option<bool>,
    /// 昵称候选列表
    #[serde(default)]
    pub nickname_candidates: Vec<String>,
    /// 角色颜色
    #[serde(default)]
    pub color: Option<String>,
    /// 自定义配置（扩展字段）
    #[serde(default)]
    pub extra: HashMap<String, String>,
}

impl AgentRoleConfig {
    /// 获取有效系统提示词（替换或拼接）
    pub fn resolve_system_prompt(&self, base: &str) -> String {
        if let Some(ref prompt) = self.system_prompt {
            // 完全替换
            prompt.clone()
        } else if let Some(ref append) = self.system_prompt_append {
            // 拼接
            format!("{base}\n\n{append}")
        } else {
            base.to_string()
        }
    }

    /// 工具是否被允许
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        if self.denied_tools.iter().any(|t| t == tool_name) {
            return false;
        }
        if self.allowed_tools.is_empty() {
            return true;
        }
        self.allowed_tools.iter().any(|t| t == tool_name)
    }
}

/// 角色注册表
#[derive(Debug, Clone, Default)]
pub struct RoleRegistry {
    roles: HashMap<String, AgentRoleConfig>,
}

impl RoleRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_builtin_roles();
        registry
    }

    /// 注册内置角色
    fn register_builtin_roles(&mut self) {
        self.register(AgentRoleConfig {
            name: "default".into(),
            description: "通用 Agent，无特殊角色约束".into(),
            nickname_candidates: vec![
                "Alpha".into(), "Beta".into(), "Gamma".into(),
                "Delta".into(), "Epsilon".into(), "Zeta".into(),
            ],
            ..Default::default()
        });

        self.register(AgentRoleConfig {
            name: "architect".into(),
            description: "系统架构师，负责高层设计和技术决策".into(),
            system_prompt_append: Some(
                "You are a System Architect. Your role is to design high-level system \
                 architecture, make technology decisions, and create implementation plans. \
                 Focus on scalability, maintainability, and best practices. \
                 Do NOT write implementation code directly — delegate to coder agents."
                    .into(),
            ),
            model: Some("gpt-4o".into()),
            temperature: Some(0.3),
            denied_tools: vec!["execute_code".into(), "run_shell".into()],
            color: Some("#00F0FF".into()),
            ..Default::default()
        });

        self.register(AgentRoleConfig {
            name: "coder".into(),
            description: "代码实现者，负责编写和修改代码".into(),
            system_prompt_append: Some(
                "You are a skilled Coder. Write clean, well-documented, and efficient code. \
                 Follow the project's coding standards. Always include error handling and \
                 edge cases. Test your code before finalizing."
                    .into(),
            ),
            model: Some("gpt-4o".into()),
            temperature: Some(0.2),
            allowed_tools: vec![
                "read_file".into(), "write_file".into(), "edit_file".into(),
                "search_code".into(), "execute_code".into(), "run_shell".into(),
            ],
            color: Some("#50FA7B".into()),
            ..Default::default()
        });

        self.register(AgentRoleConfig {
            name: "reviewer".into(),
            description: "代码审查者，负责检查代码质量和安全性".into(),
            system_prompt_append: Some(
                "You are a Code Reviewer. Your job is to review code for bugs, \
                 security vulnerabilities, performance issues, and adherence to best \
                 practices. Be thorough but constructive. Do NOT modify code directly."
                    .into(),
            ),
            model: Some("gpt-4o".into()),
            temperature: Some(0.1),
            denied_tools: vec![
                "write_file".into(), "edit_file".into(), "execute_code".into(),
                "run_shell".into(),
            ],
            allowed_tools: vec!["read_file".into(), "search_code".into()],
            color: Some("#FFB86C".into()),
            ..Default::default()
        });

        self.register(AgentRoleConfig {
            name: "debugger".into(),
            description: "调试专家，负责定位和修复 Bug".into(),
            system_prompt_append: Some(
                "You are a Debugger. Analyze error logs, stack traces, and code \
                 to identify root causes. Propose minimal, targeted fixes. \
                 Always verify your fix by reasoning through the execution path."
                    .into(),
            ),
            model: Some("gpt-4o".into()),
            temperature: Some(0.2),
            color: Some("#FF5555".into()),
            ..Default::default()
        });

        self.register(AgentRoleConfig {
            name: "planner".into(),
            description: "任务规划者，负责分解任务和制定执行计划".into(),
            system_prompt_append: Some(
                "You are a Task Planner. Break down complex tasks into manageable \
                 subtasks, estimate effort, identify dependencies, and create \
                 execution roadmaps. Do NOT implement — only plan."
                    .into(),
            ),
            model: Some("gpt-4o".into()),
            temperature: Some(0.3),
            denied_tools: vec!["write_file".into(), "edit_file".into()],
            color: Some("#BD93F9".into()),
            ..Default::default()
        });
    }

    /// 注册角色
    pub fn register(&mut self, role: AgentRoleConfig) {
        self.roles.insert(role.name.clone(), role);
    }

    /// 获取角色配置
    pub fn get(&self, name: &str) -> Option<&AgentRoleConfig> {
        self.roles.get(name)
    }

    /// 列出所有角色名
    pub fn list_names(&self) -> Vec<&str> {
        self.roles.keys().map(|s| s.as_str()).collect()
    }

    /// 列出所有角色
    pub fn list_all(&self) -> Vec<&AgentRoleConfig> {
        self.roles.values().collect()
    }

    /// 角色是否存在
    pub fn exists(&self, name: &str) -> bool {
        self.roles.contains_key(name)
    }

    /// 解析角色名（默认回退到 "default"）
    pub fn resolve_role(&self, name: Option<&str>) -> (&AgentRoleConfig, String) {
        let name = name.unwrap_or(DEFAULT_ROLE_NAME);
        if let Some(role) = self.roles.get(name) {
            (role, name.to_string())
        } else {
            (self.roles.get(DEFAULT_ROLE_NAME).expect("default role must exist"), DEFAULT_ROLE_NAME.to_string())
        }
    }
}

impl Default for AgentRoleConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            system_prompt: None,
            system_prompt_append: None,
            model: None,
            model_provider: None,
            temperature: None,
            max_tokens: None,
            allowed_tools: Vec::new(),
            denied_tools: Vec::new(),
            sandbox_enabled: None,
            nickname_candidates: Vec::new(),
            color: None,
            extra: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    //! RoleRegistry / AgentRoleConfig 单元测试（v1.52 测试体系 Phase 2 - 2.5.1）
    //! 参见：03_测试体系_单元与集成测试.md §2.2.1（agent 优先级 P1）

    use super::*;

    // ===== resolve_system_prompt =====

    #[test]
    fn test_resolve_system_prompt_uses_base_when_no_override() {
        let role = AgentRoleConfig::default();
        assert_eq!(role.resolve_system_prompt("BASE"), "BASE");
    }

    #[test]
    fn test_resolve_system_prompt_replace_when_system_prompt_set() {
        let mut role = AgentRoleConfig::default();
        role.system_prompt = Some("CUSTOM".into());
        assert_eq!(role.resolve_system_prompt("BASE"), "CUSTOM");
    }

    #[test]
    fn test_resolve_system_prompt_append_when_only_append_set() {
        let mut role = AgentRoleConfig::default();
        role.system_prompt_append = Some("APPEND".into());
        assert_eq!(role.resolve_system_prompt("BASE"), "BASE\n\nAPPEND");
    }

    #[test]
    fn test_resolve_system_prompt_replace_takes_priority_over_append() {
        let mut role = AgentRoleConfig::default();
        role.system_prompt = Some("REPLACE".into());
        role.system_prompt_append = Some("APPEND".into());
        assert_eq!(role.resolve_system_prompt("BASE"), "REPLACE");
    }

    // ===== is_tool_allowed =====

    #[test]
    fn test_is_tool_allowed_returns_true_when_no_restrictions() {
        let role = AgentRoleConfig::default();
        assert!(role.is_tool_allowed("any_tool"));
    }

    #[test]
    fn test_is_tool_allowed_denied_tool_returns_false() {
        let mut role = AgentRoleConfig::default();
        role.denied_tools = vec!["dangerous".into()];
        assert!(!role.is_tool_allowed("dangerous"));
    }

    #[test]
    fn test_is_tool_allowed_allowed_list_respects_membership() {
        let mut role = AgentRoleConfig::default();
        role.allowed_tools = vec!["read_file".into(), "write_file".into()];
        assert!(role.is_tool_allowed("read_file"));
        assert!(role.is_tool_allowed("write_file"));
        assert!(!role.is_tool_allowed("execute_code"));
    }

    #[test]
    fn test_is_tool_allowed_denied_overrides_allowed() {
        let mut role = AgentRoleConfig::default();
        role.allowed_tools = vec!["read_file".into()];
        role.denied_tools = vec!["read_file".into()];
        // denied 优先级高于 allowed
        assert!(!role.is_tool_allowed("read_file"));
    }

    // ===== RoleRegistry =====

    #[test]
    fn test_role_registry_new_includes_builtin_roles() {
        let registry = RoleRegistry::new();
        for name in BUILT_IN_ROLE_NAMES {
            assert!(registry.exists(name), "内置角色 {} 应存在", name);
        }
    }

    #[test]
    fn test_role_registry_get_returns_config() {
        let registry = RoleRegistry::new();
        let coder = registry.get("coder").expect("coder 角色应存在");
        assert_eq!(coder.name, "coder");
        assert!(!coder.description.is_empty());
    }

    #[test]
    fn test_role_registry_get_returns_none_for_unknown() {
        let registry = RoleRegistry::new();
        assert!(registry.get("nonexistent_role").is_none());
    }

    #[test]
    fn test_role_registry_register_custom_role() {
        let mut registry = RoleRegistry::new();
        let custom = AgentRoleConfig {
            name: "custom".into(),
            description: "测试自定义角色".into(),
            ..Default::default()
        };
        registry.register(custom);
        assert!(registry.exists("custom"));
        assert_eq!(registry.get("custom").unwrap().description, "测试自定义角色");
    }

    #[test]
    fn test_role_registry_list_names_contains_builtins() {
        let registry = RoleRegistry::new();
        let names = registry.list_names();
        for builtin in BUILT_IN_ROLE_NAMES {
            assert!(names.iter().any(|n| *n == *builtin), "list_names 应包含 {}", builtin);
        }
    }

    #[test]
    fn test_role_registry_resolve_role_with_none_falls_back_to_default() {
        let registry = RoleRegistry::new();
        let (role, name) = registry.resolve_role(None);
        assert_eq!(name, DEFAULT_ROLE_NAME);
        assert_eq!(role.name, DEFAULT_ROLE_NAME);
    }

    #[test]
    fn test_role_registry_resolve_role_unknown_falls_back_to_default() {
        let registry = RoleRegistry::new();
        let (role, name) = registry.resolve_role(Some("nonexistent"));
        assert_eq!(name, DEFAULT_ROLE_NAME);
        assert_eq!(role.name, DEFAULT_ROLE_NAME);
    }

    #[test]
    fn test_role_registry_resolve_role_known_returns_it() {
        let registry = RoleRegistry::new();
        let (role, name) = registry.resolve_role(Some("coder"));
        assert_eq!(name, "coder");
        assert_eq!(role.name, "coder");
    }

    #[test]
    fn test_builtin_coder_role_denies_nothing_in_default_tools() {
        // coder 角色应允许 read_file（在 allowed_tools 中）
        let registry = RoleRegistry::new();
        let coder = registry.get("coder").unwrap();
        assert!(coder.is_tool_allowed("read_file"));
        assert!(coder.is_tool_allowed("execute_code"));
    }

    #[test]
    fn test_builtin_reviewer_role_denies_write_tools() {
        let registry = RoleRegistry::new();
        let reviewer = registry.get("reviewer").unwrap();
        // reviewer 在 denied_tools 中包含 write_file/edit_file
        assert!(!reviewer.is_tool_allowed("write_file"));
        assert!(!reviewer.is_tool_allowed("edit_file"));
        // reviewer 在 allowed_tools 中包含 read_file（非空 allowed 优先于 denied）
        assert!(reviewer.is_tool_allowed("read_file"));
        // 不在 allowed 列表中的工具应被拒
        assert!(!reviewer.is_tool_allowed("execute_code"));
    }
}