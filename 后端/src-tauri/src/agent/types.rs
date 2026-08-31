//! Yuan Code v3.1 Task 3.1.1 — Agent 类型系统（Phase 3）
//!
//! 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.1.1
//!       + 参考 D:\Desktop\codex-main 源码（agent_communication.rs / codex_thread.rs 设计思路）
//!
//! 与现有 agent/role.rs 的区别：
//! - role.rs 是「角色配置」层（architect/coder/reviewer/debugger/planner/devops/security）
//! - types.rs 是「任务类型」层，对应 7 种编程任务分类：
//!     编码 / 重构 / 测试 / 文档 / 调试 / 迁移 / 评审
//! - 任务类型决定了 Agent 的 prompt 模板、Plan 默认结构、安全检查等级
//!
//! 强制约束（项目核心设计意图 §三 / §八）：
//! - 所有 Agent 的 AI 调用必须走 cloud_api_router（云端 API）
//! - 禁止本地底层智能模型用于编程生成

use serde::{Deserialize, Serialize};

/// Yuan Code v3.1 Agent 任务类型 — 7 种
///
/// 依据：功能展望/01_Yuan_Code_对标Codex升级_v3_65.md §Phase 3（7 种 Agent 类型）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// 编码：从需求生成新代码（函数/类/模块）
    Coding,
    /// 重构：在不改变行为的前提下改进代码结构
    Refactor,
    /// 测试：生成单元测试 / 集成测试 / E2E 测试
    Test,
    /// 文档：生成/更新 API 文档、README、注释
    Documentation,
    /// 调试：定位 Bug + 提出最小修复方案
    Debug,
    /// 迁移：版本升级 / 框架切换 / API 兼容
    Migration,
    /// 评审：审查代码质量、安全、性能
    Review,
}

impl AgentType {
    /// 全部 7 种类型
    pub const ALL: &'static [AgentType] = &[
        AgentType::Coding,
        AgentType::Refactor,
        AgentType::Test,
        AgentType::Documentation,
        AgentType::Debug,
        AgentType::Migration,
        AgentType::Review,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            AgentType::Coding => "coding",
            AgentType::Refactor => "refactor",
            AgentType::Test => "test",
            AgentType::Documentation => "documentation",
            AgentType::Debug => "debug",
            AgentType::Migration => "migration",
            AgentType::Review => "review",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AgentType::Coding => "编码 Agent",
            AgentType::Refactor => "重构 Agent",
            AgentType::Test => "测试 Agent",
            AgentType::Documentation => "文档 Agent",
            AgentType::Debug => "调试 Agent",
            AgentType::Migration => "迁移 Agent",
            AgentType::Review => "评审 Agent",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            AgentType::Coding => "从需求生成新代码：函数、类、模块、API 端点",
            AgentType::Refactor => "在不改变外部行为的前提下改进代码结构、可读性、性能",
            AgentType::Test => "生成单元测试、集成测试、E2E 测试，提升覆盖率",
            AgentType::Documentation => "生成或更新 API 文档、README、内联注释",
            AgentType::Debug => "分析错误日志、堆栈、复现路径，定位根因并提出最小修复",
            AgentType::Migration => "版本升级、框架切换、API 兼容性迁移",
            AgentType::Review => "审查代码质量、安全漏洞、性能问题、最佳实践",
        }
    }

    /// 默认系统提示词（每种 Agent 类型有专属 prompt 模板）
    pub fn default_system_prompt(&self) -> String {
        let base = "You are a Yuan Code v3.1 Agent. You MUST call cloud APIs (OpenAI/Claude/etc.) \
                   for all programming tasks. Using local underlying intelligence models for code \
                   generation is FORBIDDEN by project design intent (§3/§8).";
        match self {
            AgentType::Coding => format!(
                "{base}\n\nRole: Coding Agent\n\
                 Your job: implement new code (functions, classes, modules) from requirements.\n\
                 - Always read existing code before writing new code\n\
                 - Follow the project's coding standards\n\
                 - Include error handling and edge cases\n\
                 - Output a clear Plan before implementation\n\
                 - Generate diffs for user review"
            ),
            AgentType::Refactor => format!(
                "{base}\n\nRole: Refactor Agent\n\
                 Your job: improve code structure WITHOUT changing external behavior.\n\
                 - Preserve public API signatures\n\
                 - Verify behavior equivalence via existing tests (or generate new tests first)\n\
                 - Output before/after diffs for review\n\
                 - Never silently change behavior"
            ),
            AgentType::Test => format!(
                "{base}\n\nRole: Test Agent\n\
                 Your job: generate tests (unit/integration/e2e) that verify behavior.\n\
                 - Prefer testing behavior over implementation details\n\
                 - Cover happy path + edge cases + error cases\n\
                 - Aim for >80% coverage on the target module\n\
                 - Run tests in sandbox before reporting success"
            ),
            AgentType::Documentation => format!(
                "{base}\n\nRole: Documentation Agent\n\
                 Your job: generate or update documentation.\n\
                 - API docs: signatures, parameters, return types, examples\n\
                 - README: setup, usage, contribution\n\
                 - Inline comments: only where non-obvious logic exists\n\
                 - Never document trivial getters/setters"
            ),
            AgentType::Debug => format!(
                "{base}\n\nRole: Debug Agent\n\
                 Your job: locate bugs and propose MINIMAL fixes.\n\
                 - Reproduce the issue first (write a failing test if possible)\n\
                 - Trace from symptom to root cause (logs, stack traces, code paths)\n\
                 - Propose the smallest fix that addresses the root cause\n\
                 - Verify the fix via the reproduction test"
            ),
            AgentType::Migration => format!(
                "{base}\n\nRole: Migration Agent\n\
                 Your job: migrate code across versions/frameworks/APIs.\n\
                 - Identify all call sites that need changes\n\
                 - Provide a step-by-step migration plan with rollback points\n\
                 - Preserve behavior unless explicitly changing it\n\
                 - Run tests after each step"
            ),
            AgentType::Review => format!(
                "{base}\n\nRole: Code Review Agent\n\
                 Your job: review code for quality, security, performance.\n\
                 - Categorize findings by severity (blocker/major/minor/nit)\n\
                 - Provide actionable suggestions (not just complaints)\n\
                 - Check: bugs, security vulnerabilities, performance, best practices\n\
                 - NEVER modify code directly — only review and suggest"
            ),
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "coding" => Some(AgentType::Coding),
            "refactor" => Some(AgentType::Refactor),
            "test" => Some(AgentType::Test),
            "documentation" | "docs" | "doc" => Some(AgentType::Documentation),
            "debug" | "debugger" => Some(AgentType::Debug),
            "migration" | "migrate" => Some(AgentType::Migration),
            "review" | "reviewer" => Some(AgentType::Review),
            _ => None,
        }
    }
}

/// Agent 执行阶段（与 codex_thread 的 Phase 概念对齐）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentPhase {
    /// 待启动
    Pending,
    /// 制定计划
    Planning,
    /// 等待用户批准计划
    PlanReview,
    /// 在沙箱内执行
    Executing,
    /// 等待用户 review diff
    AwaitingReview,
    /// 已完成
    Completed,
    /// 用户拒绝
    Rejected,
    /// 错误
    Errored,
}

impl AgentPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentPhase::Pending => "pending",
            AgentPhase::Planning => "planning",
            AgentPhase::PlanReview => "plan_review",
            AgentPhase::Executing => "executing",
            AgentPhase::AwaitingReview => "awaiting_review",
            AgentPhase::Completed => "completed",
            AgentPhase::Rejected => "rejected",
            AgentPhase::Errored => "errored",
        }
    }
}

/// Agent 计划的单个步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPlanStep {
    /// 步骤序号（1-based）
    pub step_id: u32,
    pub title: String,
    pub description: String,
    /// 涉及的文件路径（用于 review 前预览）
    #[serde(default)]
    pub target_files: Vec<String>,
    /// 是否需要用户在执行前确认（敏感操作）
    #[serde(default)]
    pub requires_confirmation: bool,
    /// 步骤状态：pending/in_progress/completed/skipped/failed
    pub status: String,
}

/// Agent 完整执行计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPlan {
    pub agent_id: String,
    pub agent_type: String,
    pub title: String,
    pub summary: String,
    pub steps: Vec<AgentPlanStep>,
    pub estimated_steps: u32,
    pub estimated_duration_sec: Option<u32>,
    pub created_at: i64,
}

/// 单个文件的 diff（用户 review 时展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFileDiff {
    pub path: String,
    /// unified diff 文本
    pub unified_diff: String,
    /// 修改前内容（可选，便于双栏展示）
    #[serde(default)]
    pub original_content: Option<String>,
    /// 修改后内容
    #[serde(default)]
    pub modified_content: Option<String>,
    /// 新增行数
    pub added_lines: u32,
    /// 删除行数
    pub removed_lines: u32,
    /// 是否为新建文件
    pub is_new_file: bool,
}

/// Agent 执行结果（提交给用户 review）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub agent_id: String,
    pub agent_type: String,
    pub plan: AgentPlan,
    /// 涉及的所有文件 diff
    pub diffs: Vec<AgentFileDiff>,
    /// 在沙箱中执行的命令日志
    #[serde(default)]
    pub execution_log: Vec<String>,
    /// 模型 / token 使用统计
    pub provider: String,
    pub model_used: String,
    pub tokens_used: Option<i64>,
    /// Agent 的总结说明
    pub summary: String,
    pub completed_at: i64,
}

/// Agent 用户 review 决议
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentReviewDecision {
    /// 批准并应用所有 diff
    Approve,
    /// 拒绝并丢弃所有 diff
    Reject,
    /// 部分批准（仅应用 selected_files 中的文件）
    ApprovePartial,
}

/// 用户 review 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReviewRequest {
    pub agent_id: String,
    pub decision: AgentReviewDecision,
    /// 当 decision = ApprovePartial 时，要应用的文件路径列表
    #[serde(default)]
    pub selected_files: Vec<String>,
    /// 拒绝/部分批准时的反馈
    #[serde(default)]
    pub feedback: Option<String>,
    /// 工作区根路径（应用 diff 时拼绝对路径）
    #[serde(default)]
    pub workspace_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_all_has_seven_variants() {
        assert_eq!(AgentType::ALL.len(), 7, "应恰好 7 种 Agent 类型");
    }

    #[test]
    fn test_agent_type_from_str_roundtrip() {
        for t in AgentType::ALL {
            let s = t.as_str();
            let back = AgentType::from_str(s).expect("应可往返");
            assert_eq!(back, *t);
        }
    }

    #[test]
    fn test_agent_type_from_str_accepts_aliases() {
        assert_eq!(AgentType::from_str("docs"), Some(AgentType::Documentation));
        assert_eq!(AgentType::from_str("debugger"), Some(AgentType::Debug));
        assert_eq!(AgentType::from_str("migrate"), Some(AgentType::Migration));
        assert_eq!(AgentType::from_str("reviewer"), Some(AgentType::Review));
    }

    #[test]
    fn test_agent_type_from_str_rejects_unknown() {
        assert_eq!(AgentType::from_str("nonexistent"), None);
    }

    #[test]
    fn test_default_system_prompt_contains_cloud_api_constraint() {
        for t in AgentType::ALL {
            let prompt = t.default_system_prompt();
            assert!(prompt.contains("cloud"), "{} prompt 应提及 cloud API", t.as_str());
            assert!(prompt.contains("FORBIDDEN"), "{} prompt 应禁止本地模型", t.as_str());
        }
    }

    #[test]
    fn test_default_system_prompt_review_agent_cannot_modify_code() {
        let prompt = AgentType::Review.default_system_prompt();
        assert!(prompt.contains("NEVER modify"));
    }

    #[test]
    fn test_agent_phase_as_str() {
        assert_eq!(AgentPhase::Pending.as_str(), "pending");
        assert_eq!(AgentPhase::AwaitingReview.as_str(), "awaiting_review");
    }
}
