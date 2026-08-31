use serde::{Deserialize, Serialize};

/// Prompt 模板类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PromptTemplateType {
    /// 系统主提示词 (对标 gpt_5_1_prompt.md)
    System,
    /// 代码审查提示词 (对标 review_prompt.md)
    Review,
    /// 上下文压缩提示词 (对标 compact/prompt.md)
    Compact,
    /// 计划制定提示词 (对标 update_plan)
    Plan,
    /// 子 Agent 提示词 (对标 multi_agents)
    SubAgent,
    /// 实时协作提示词 (对标 realtime/backend_prompt.md)
    Realtime,
    /// 渐进式披露 - 技能使用指南
    SkillsHowTo,
}

impl PromptTemplateType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::System => "system",
            Self::Review => "review",
            Self::Compact => "compact",
            Self::Plan => "plan",
            Self::SubAgent => "sub_agent",
            Self::Realtime => "realtime",
            Self::SkillsHowTo => "skills_how_to",
        }
    }
}

/// 模板变量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVariable {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
}

/// 模板元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplateMeta {
    pub name: String,
    pub template_type: PromptTemplateType,
    pub description: String,
    pub version: String,
    pub variables: Vec<PromptVariable>,
}

/// 渲染请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPromptRequest {
    /// 模板类型
    pub template_type: PromptTemplateType,
    /// 变量值映射
    pub variables: std::collections::HashMap<String, String>,
}

/// 渲染结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPromptResult {
    /// 渲染后的 prompt 文本
    pub content: String,
    /// 估算的 token 数 (按 4 字符/token 粗估)
    pub estimated_tokens: usize,
    /// 使用的模板名称
    pub template_name: String,
    /// 模板版本
    pub template_version: String,
}

/// AGENTS.md 文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsMdFile {
    /// 文件路径
    pub path: String,
    /// 文件内容
    pub content: String,
    /// 所在目录层级深度 (从项目根或 CWD 计算)
    pub depth: usize,
    /// 作用域根目录
    pub scope_root: String,
}

/// 层级指令组装结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchicalInstructions {
    /// 来源文件列表
    pub sources: Vec<String>,
    /// 组装后的完整指令
    pub assembled: String,
    /// 总字节数
    pub total_bytes: usize,
    /// 是否被截断
    pub truncated: bool,
}

/// 系统 Prompt 组装请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembleSystemPromptRequest {
    /// 项目根目录
    pub project_root: Option<String>,
    /// 当前工作目录
    pub cwd: Option<String>,
    /// 用户自定义指令
    pub user_instructions: Option<String>,
    /// 技能列表 (用于注入)
    pub skill_names: Option<Vec<String>>,
    /// 最大字节预算
    pub max_bytes: Option<usize>,
    /// 是否包含子 Agent 指令
    pub include_child_agent_instructions: Option<bool>,
}

/// 系统 Prompt 组装结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembleSystemPromptResult {
    /// 最终系统 prompt
    pub system_prompt: String,
    /// 各部分 prompt 来源
    pub parts: Vec<PromptPart>,
    /// 估算 token 数
    pub estimated_tokens: usize,
    /// 总字节数
    pub total_bytes: usize,
}

/// Prompt 组成部分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPart {
    /// 部分名称
    pub name: String,
    /// 来源
    pub source: String,
    /// 内容
    pub content: String,
    /// 字节数
    pub bytes: usize,
}