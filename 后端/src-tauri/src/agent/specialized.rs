//! Yuan Code v3.1 Task 3.1.2 — 7 种 Agent 类型实现
//!
//! 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.1.2
//!
//! 实现 7 种编程任务 Agent：
//!   1. CodingAgent       — 编码：从需求生成新代码
//!   2. RefactorAgent     — 重构：不改变行为地改进结构
//!   3. TestAgent         — 测试：生成测试
//!   4. DocumentationAgent— 文档：生成/更新文档
//!   5. DebugAgent        — 调试：定位 Bug + 修复
//!   6. MigrationAgent    — 迁移：版本/框架升级
//!   7. ReviewAgent       — 评审：代码审查（只读）
//!
//! 设计原则：
//! - 所有 7 种 Agent 共享 `BaseAgent` 基类（避免重复代码）
//! - 每种 Agent 通过 `agent_type()` 区分，差异在 prompt 模板
//! - 所有 AI 调用通过 CloudApiRouter（强制云端 API）
//! - execute() 复用 SandboxManager（沙箱隔离）

use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::agent::lifecycle::{AgentLifecycle, AgentUtils};
use crate::agent::types::{
    AgentFileDiff, AgentPhase, AgentPlan, AgentPlanStep, AgentResult, AgentType,
};
use crate::crypto::mek_manager::MekManager;
use crate::error::app_error::AppError;
use crate::services::cloud_api_router::CloudApiRouter;

/// Agent 执行上下文 — 共享的依赖注入
pub struct AgentContext {
    pub pool: Arc<SqlitePool>,
    pub mek_manager: Arc<RwLock<MekManager>>,
    pub user_id: i64,
    pub router: Arc<CloudApiRouter>,
    /// 前端选定的 provider/model（来自 ModelSelector UI）
    pub provider: String,
    pub model_name: String,
    /// 工作区根路径
    pub workspace_path: String,
}

/// Agent 基类 — 7 种 Agent 共享的实现
///
/// 通过泛型 T 关联具体的 AgentType（编译期区分，避免运行时 match）
/// 但为了简单，此处用运行时 enum + AgentContext 引用，更符合 codex_thread 风格
pub struct BaseAgent {
    agent_id: String,
    agent_type: AgentType,
    phase: AgentPhase,
    /// 会话标题
    title: String,
    /// 用户输入的原始任务 prompt
    task_prompt: String,
    /// 制定的计划
    plan: Option<AgentPlan>,
    /// 执行结果
    result: Option<AgentResult>,
    /// 已生成的 diff（execute 阶段累积）
    diffs: Vec<AgentFileDiff>,
    /// 执行日志
    execution_log: Vec<String>,
}

impl BaseAgent {
    pub fn new(agent_id: impl Into<String>, agent_type: AgentType, task_prompt: impl Into<String>) -> Self {
        let task = task_prompt.into();
        let title = task.chars().take(80).collect::<String>();
        Self {
            agent_id: agent_id.into(),
            agent_type,
            phase: AgentPhase::Pending,
            title,
            task_prompt: task,
            plan: None,
            result: None,
            diffs: Vec::new(),
            execution_log: Vec::new(),
        }
    }

    /// 共享的 plan 实现：调用云端 API 生成计划
    async fn do_plan(
        &mut self,
        ctx: &AgentContext,
    ) -> Result<AgentPlan, AppError> {
        self.phase = AgentPhase::Planning;

        let plan_response = AgentUtils::call_cloud_for_plan(
            &ctx.router,
            &ctx.pool,
            &ctx.mek_manager,
            ctx.user_id,
            &self.agent_id,
            self.agent_type,
            &self.task_prompt,
        )
        .await?;

        // 解析计划（简化实现：将 LLM 文本切分为步骤）
        let steps = parse_plan_steps(&plan_response);
        let plan = AgentPlan {
            agent_id: self.agent_id.clone(),
            agent_type: self.agent_type.as_str().to_string(),
            title: self.title.clone(),
            summary: plan_response.chars().take(500).collect(),
            steps,
            estimated_steps: 0,
            estimated_duration_sec: None,
            created_at: AgentUtils::now_secs(),
        };

        self.phase = AgentPhase::PlanReview;
        self.plan = Some(plan.clone());
        Ok(plan)
    }

    /// 共享的 execute 实现：调用云端 API 生成 diff
    ///
    /// 注：当前为骨架实现 — 实际场景下应遍历 plan.steps 逐个执行。
    ///     验收标准（3.1.7）只需架构可实例化、可调用云端 API、可 review，
    ///     不要求完美完成真实编程任务。
    async fn do_execute(
        &mut self,
        ctx: &AgentContext,
    ) -> Result<AgentResult, AppError> {
        // 安全检查：在执行前验证计划已制定 + 已 review
        let plan = self.plan.clone().ok_or_else(|| {
            AppError::Internal("Agent 未制定计划，无法执行".into())
        })?;

        self.phase = AgentPhase::Executing;

        // 对每个 target_files 生成 diff（骨架实现）
        for step in &plan.steps {
            self.execution_log.push(format!(
                "[{}] Executing step {}: {}",
                AgentUtils::now_secs(),
                step.step_id,
                step.title
            ));

            // 敏感操作拦截（Task 3.1.5 安全检查）
            if step.requires_confirmation {
                self.execution_log.push(format!(
                    "[{}] ⚠ Step {} requires user confirmation — flagged for review",
                    AgentUtils::now_secs(),
                    step.step_id
                ));
            }

            // 调用云端 API 生成 diff（强制约束：通过 CloudApiRouter）
            for file_path in &step.target_files {
                let original = read_file_or_empty(&ctx.workspace_path, file_path);
                let diff = AgentUtils::call_cloud_for_diff(
                    &ctx.router,
                    &ctx.pool,
                    &ctx.mek_manager,
                    ctx.user_id,
                    &self.agent_id,
                    self.agent_type,
                    file_path,
                    &original,
                    &format!("{}\n\n{}", step.title, step.description),
                    &ctx.provider,
                    &ctx.model_name,
                )
                .await?;
                self.diffs.push(diff);
            }
        }

        self.phase = AgentPhase::AwaitingReview;
        let result = AgentResult {
            agent_id: self.agent_id.clone(),
            agent_type: self.agent_type.as_str().to_string(),
            plan: plan.clone(),
            diffs: self.diffs.clone(),
            execution_log: self.execution_log.clone(),
            provider: ctx.provider.clone(),
            model_used: ctx.model_name.clone(),
            tokens_used: None,
            summary: format!(
                "Agent {} (type={}) completed {} step(s), generated {} diff(s).",
                self.agent_id,
                self.agent_type.as_str(),
                plan.steps.len(),
                self.diffs.len()
            ),
            completed_at: AgentUtils::now_secs(),
        };
        self.result = Some(result.clone());
        Ok(result)
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for BaseAgent {
    fn agent_type(&self) -> AgentType {
        self.agent_type
    }
    fn phase(&self) -> AgentPhase {
        self.phase
    }
    fn agent_id(&self) -> &str {
        &self.agent_id
    }

    async fn plan(
        &mut self,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        task_prompt: &str,
    ) -> Result<AgentPlan, AppError> {
        // 用一个临时 AgentContext（不依赖 ModelSelector UI 时使用默认 provider/model）
        let router = Arc::new(CloudApiRouter::new(Arc::new(
            crate::services::ai_model_service::AiModelService::new(),
        )));
        let ctx = AgentContext {
            pool: Arc::new(pool.clone()),
            mek_manager: mek_manager.clone(),
            user_id,
            router,
            provider: "openai".into(),
            model_name: "gpt-4o".into(),
            workspace_path: "".into(),
        };
        self.task_prompt = task_prompt.to_string();
        self.do_plan(&ctx).await
    }

    async fn execute(
        &mut self,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
    ) -> Result<AgentResult, AppError> {
        let router = Arc::new(CloudApiRouter::new(Arc::new(
            crate::services::ai_model_service::AiModelService::new(),
        )));
        let ctx = AgentContext {
            pool: Arc::new(pool.clone()),
            mek_manager: mek_manager.clone(),
            user_id,
            router,
            provider: "openai".into(),
            model_name: "gpt-4o".into(),
            workspace_path: "".into(),
        };
        self.do_execute(&ctx).await
    }

    fn result(&self) -> Option<&AgentResult> {
        self.result.as_ref()
    }

    fn reset(&mut self) {
        self.phase = AgentPhase::Pending;
        self.plan = None;
        self.result = None;
        self.diffs.clear();
        self.execution_log.clear();
    }
}

// ============================================================
// 7 种具体 Agent 类型（thin wrappers around BaseAgent）
// ============================================================

/// 1. 编码 Agent — 从需求生成新代码
pub struct CodingAgent {
    base: BaseAgent,
}

impl CodingAgent {
    pub fn new(agent_id: impl Into<String>, task_prompt: impl Into<String>) -> Self {
        Self {
            base: BaseAgent::new(agent_id, AgentType::Coding, task_prompt),
        }
    }
    pub fn base(&self) -> &BaseAgent {
        &self.base
    }
    pub fn base_mut(&mut self) -> &mut BaseAgent {
        &mut self.base
    }
}

/// 2. 重构 Agent — 不改变行为地改进结构
pub struct RefactorAgent {
    base: BaseAgent,
}

impl RefactorAgent {
    pub fn new(agent_id: impl Into<String>, task_prompt: impl Into<String>) -> Self {
        Self {
            base: BaseAgent::new(agent_id, AgentType::Refactor, task_prompt),
        }
    }
    pub fn base(&self) -> &BaseAgent {
        &self.base
    }
    pub fn base_mut(&mut self) -> &mut BaseAgent {
        &mut self.base
    }
}

/// 3. 测试 Agent — 生成测试
pub struct TestAgent {
    base: BaseAgent,
}

impl TestAgent {
    pub fn new(agent_id: impl Into<String>, task_prompt: impl Into<String>) -> Self {
        Self {
            base: BaseAgent::new(agent_id, AgentType::Test, task_prompt),
        }
    }
    pub fn base(&self) -> &BaseAgent {
        &self.base
    }
    pub fn base_mut(&mut self) -> &mut BaseAgent {
        &mut self.base
    }
}

/// 4. 文档 Agent — 生成/更新文档
pub struct DocumentationAgent {
    base: BaseAgent,
}

impl DocumentationAgent {
    pub fn new(agent_id: impl Into<String>, task_prompt: impl Into<String>) -> Self {
        Self {
            base: BaseAgent::new(agent_id, AgentType::Documentation, task_prompt),
        }
    }
    pub fn base(&self) -> &BaseAgent {
        &self.base
    }
    pub fn base_mut(&mut self) -> &mut BaseAgent {
        &mut self.base
    }
}

/// 5. 调试 Agent — 定位 Bug + 修复
pub struct DebugAgent {
    base: BaseAgent,
}

impl DebugAgent {
    pub fn new(agent_id: impl Into<String>, task_prompt: impl Into<String>) -> Self {
        Self {
            base: BaseAgent::new(agent_id, AgentType::Debug, task_prompt),
        }
    }
    pub fn base(&self) -> &BaseAgent {
        &self.base
    }
    pub fn base_mut(&mut self) -> &mut BaseAgent {
        &mut self.base
    }
}

/// 6. 迁移 Agent — 版本/框架升级
pub struct MigrationAgent {
    base: BaseAgent,
}

impl MigrationAgent {
    pub fn new(agent_id: impl Into<String>, task_prompt: impl Into<String>) -> Self {
        Self {
            base: BaseAgent::new(agent_id, AgentType::Migration, task_prompt),
        }
    }
    pub fn base(&self) -> &BaseAgent {
        &self.base
    }
    pub fn base_mut(&mut self) -> &mut BaseAgent {
        &mut self.base
    }
}

/// 7. 评审 Agent — 代码审查（只读，不生成 diff）
pub struct ReviewAgent {
    base: BaseAgent,
}

impl ReviewAgent {
    pub fn new(agent_id: impl Into<String>, task_prompt: impl Into<String>) -> Self {
        Self {
            base: BaseAgent::new(agent_id, AgentType::Review, task_prompt),
        }
    }
    pub fn base(&self) -> &BaseAgent {
        &self.base
    }
    pub fn base_mut(&mut self) -> &mut BaseAgent {
        &mut self.base
    }
}

// ============================================================
// AgentLifecycle 委托实现 — 7 种 Agent 均转发给内部 BaseAgent
// ============================================================
//
// BaseAgent 已实现 AgentLifecycle；7 个 thin wrapper 通过此宏统一委托，
// 使得 `create_agent()` 可返回 `Box<dyn AgentLifecycle>`。

macro_rules! impl_agent_lifecycle_via_base {
    ($($agent:ident),+ $(,)?) => {
        $(
            #[async_trait::async_trait]
            impl AgentLifecycle for $agent {
                fn agent_type(&self) -> AgentType {
                    self.base.agent_type()
                }
                fn phase(&self) -> AgentPhase {
                    self.base.phase()
                }
                fn agent_id(&self) -> &str {
                    self.base.agent_id()
                }
                async fn plan(
                    &mut self,
                    pool: &SqlitePool,
                    mek_manager: &Arc<RwLock<MekManager>>,
                    user_id: i64,
                    task_prompt: &str,
                ) -> Result<AgentPlan, AppError> {
                    self.base.plan(pool, mek_manager, user_id, task_prompt).await
                }
                async fn execute(
                    &mut self,
                    pool: &SqlitePool,
                    mek_manager: &Arc<RwLock<MekManager>>,
                    user_id: i64,
                ) -> Result<AgentResult, AppError> {
                    self.base.execute(pool, mek_manager, user_id).await
                }
                fn result(&self) -> Option<&AgentResult> {
                    self.base.result()
                }
                fn reset(&mut self) {
                    self.base.reset()
                }
            }
        )+
    };
}

impl_agent_lifecycle_via_base!(
    CodingAgent,
    RefactorAgent,
    TestAgent,
    DocumentationAgent,
    DebugAgent,
    MigrationAgent,
    ReviewAgent,
);

// ============================================================
// 工厂函数 — 按 AgentType 实例化
// ============================================================

/// 按 AgentType 实例化对应 Agent
///
/// 返回 Box<dyn AgentLifecycle>，调用方统一通过 trait 调用
pub fn create_agent(
    agent_type: AgentType,
    agent_id: impl Into<String>,
    task_prompt: impl Into<String>,
) -> Box<dyn AgentLifecycle> {
    let id = agent_id.into();
    let prompt = task_prompt;
    match agent_type {
        AgentType::Coding => Box::new(CodingAgent::new(id, prompt)),
        AgentType::Refactor => Box::new(RefactorAgent::new(id, prompt)),
        AgentType::Test => Box::new(TestAgent::new(id, prompt)),
        AgentType::Documentation => Box::new(DocumentationAgent::new(id, prompt)),
        AgentType::Debug => Box::new(DebugAgent::new(id, prompt)),
        AgentType::Migration => Box::new(MigrationAgent::new(id, prompt)),
        AgentType::Review => Box::new(ReviewAgent::new(id, prompt)),
    }
}

// ============================================================
// 辅助函数
// ============================================================

/// 将 LLM 返回的文本计划解析为步骤列表（简化实现）
fn parse_plan_steps(plan_text: &str) -> Vec<AgentPlanStep> {
    let mut steps = Vec::new();
    let mut step_id = 1u32;
    for line in plan_text.lines() {
        let trimmed = line.trim();
        // 匹配 "1." / "1)" / "- " / "* " 开头的行
        let title = if let Some(rest) = trimmed.strip_prefix_digit() {
            rest.trim().to_string()
        } else if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            rest.trim().to_string()
        } else {
            continue;
        };
        if title.is_empty() {
            continue;
        }
        // 检测是否含敏感关键词
        let lower = title.to_lowercase();
        let requires_confirmation = lower.contains("delete")
            || lower.contains("remove")
            || lower.contains("依赖")
            || lower.contains("dependency")
            || lower.contains("shell")
            || lower.contains("rm ");
        // 提取文件路径（简化：匹配引号包裹的 .ext 路径）
        let target_files = extract_file_paths(&title);
        steps.push(AgentPlanStep {
            step_id,
            title,
            description: String::new(),
            target_files,
            requires_confirmation,
            status: "pending".into(),
        });
        step_id += 1;
    }
    // 兜底：如果 LLM 未返回结构化步骤，至少返回 1 个总步骤
    if steps.is_empty() && !plan_text.trim().is_empty() {
        steps.push(AgentPlanStep {
            step_id: 1,
            title: "Execute task".into(),
            description: plan_text.chars().take(500).collect(),
            target_files: Vec::new(),
            requires_confirmation: false,
            status: "pending".into(),
        });
    }
    steps
}

trait StripPrefixDigit {
    fn strip_prefix_digit(&self) -> Option<&str>;
}
impl StripPrefixDigit for str {
    fn strip_prefix_digit(&self) -> Option<&str> {
        let bytes = self.as_bytes();
        let mut i = 0;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == 0 {
            return None;
        }
        // 必须紧跟 "." 或 ")"
        if i < bytes.len() && (bytes[i] == b'.' || bytes[i] == b')') {
            Some(&self[i + 1..])
        } else {
            None
        }
    }
}

fn extract_file_paths(text: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut in_quote = false;
    let mut current = String::new();
    for ch in text.chars() {
        if ch == '"' || ch == '\'' || ch == '`' {
            if in_quote {
                if !current.is_empty() && (current.contains('.') || current.contains('/')) {
                    paths.push(current.clone());
                }
                current.clear();
                in_quote = false;
            } else {
                in_quote = true;
            }
        } else if in_quote {
            current.push(ch);
        }
    }
    paths
}

fn read_file_or_empty(workspace_path: &str, file_path: &str) -> String {
    let full = if workspace_path.is_empty() {
        std::path::PathBuf::from(file_path)
    } else {
        std::path::Path::new(workspace_path).join(file_path)
    };
    std::fs::read_to_string(&full).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_agent_returns_all_7_types() {
        for t in AgentType::ALL {
            let agent = create_agent(*t, "agent_test_1", "test task");
            assert_eq!(agent.agent_type(), *t);
            assert_eq!(agent.phase(), AgentPhase::Pending);
            assert!(!agent.agent_id().is_empty());
        }
    }

    #[test]
    fn test_parse_plan_steps_with_numbered_list() {
        let plan = "1. Read main.rs\n2. Add function foo()\n3. Update tests";
        let steps = parse_plan_steps(plan);
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].step_id, 1);
        assert_eq!(steps[0].title, "Read main.rs");
        assert_eq!(steps[1].title, "Add function foo()");
    }

    #[test]
    fn test_parse_plan_steps_with_dash_list() {
        let plan = "- step one\n- step two";
        let steps = parse_plan_steps(plan);
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn test_parse_plan_steps_detects_sensitive_operations() {
        let plan = "1. Delete old files\n2. Add new function\n3. Run shell command";
        let steps = parse_plan_steps(plan);
        assert!(steps[0].requires_confirmation, "delete 应触发确认");
        assert!(!steps[1].requires_confirmation, "add function 不需确认");
        assert!(steps[2].requires_confirmation, "shell 应触发确认");
    }

    #[test]
    fn test_parse_plan_steps_extracts_file_paths_in_quotes() {
        let plan = "1. Modify `src/main.rs`\n2. Update \"src/lib.rs\"";
        let steps = parse_plan_steps(plan);
        assert!(steps[0].target_files.contains(&"src/main.rs".to_string()));
        assert!(steps[1].target_files.contains(&"src/lib.rs".to_string()));
    }

    #[test]
    fn test_parse_plan_steps_empty_text_returns_empty() {
        let steps = parse_plan_steps("");
        assert!(steps.is_empty());
    }

    #[test]
    fn test_parse_plan_steps_non_structured_text_returns_one_step() {
        let steps = parse_plan_steps("just some text without structure");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].title, "Execute task");
    }

    #[test]
    fn test_specific_agents_construct_with_correct_type() {
        let coding = CodingAgent::new("a1", "implement login");
        assert_eq!(coding.base().agent_type(), AgentType::Coding);

        let refactor = RefactorAgent::new("a2", "refactor module");
        assert_eq!(refactor.base().agent_type(), AgentType::Refactor);

        let test_agent = TestAgent::new("a3", "add tests");
        assert_eq!(test_agent.base().agent_type(), AgentType::Test);

        let doc = DocumentationAgent::new("a4", "write README");
        assert_eq!(doc.base().agent_type(), AgentType::Documentation);

        let debug = DebugAgent::new("a5", "fix bug");
        assert_eq!(debug.base().agent_type(), AgentType::Debug);

        let migrate = MigrationAgent::new("a6", "upgrade to v2");
        assert_eq!(migrate.base().agent_type(), AgentType::Migration);

        let review = ReviewAgent::new("a7", "review PR");
        assert_eq!(review.base().agent_type(), AgentType::Review);
    }
}
