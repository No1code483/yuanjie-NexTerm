//! Yuan Code v3.2 Skill 系统（Phase 5 Task 3.3.1）— 10 个内置 Skill
//!
//! 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §Phase 5（v1.54）
//!       + 项目核心设计意图 §三 / §八（强制规则 8.1.1：Yuan Code 编程 AI 必须走云端 API）
//!
//! 设计：
//! - `BuiltinSkill` trait：每个内置 Skill 实现 name/description/system_prompt/build_prompt/execute
//! - `execute()` 通过 `SkillExecutionContext` 调用 `CloudApiRouter`（强制云端 API）
//! - `BuiltinSkillRegistry`：进程内注册表，按 name 查找，列出全部
//! - 10 个 Skill：refactor / test_generation / doc_generation / formatter / dep_update
//!                security_scan / perf_analysis / api_design / db_migration / code_review
//!
//! 强制约束（项目核心设计意图 §三 / §八）：
//! - 所有 Skill 的 AI 调用必须走云端 API（OpenAI/Claude/...），禁止本地底层智能模型
//! - `execute()` 内部通过 `CloudApiRouter::route_programming_request()` 路由，
//!   该方法 `validate_cloud_only()` 守卫拒绝本地 provider（ollama 等）

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::crypto::mek_manager::MekManager;
use crate::error::app_error::AppError;
use crate::services::cloud_api_router::{CloudApiRouter, ProgrammingRequest};

pub mod api_design;
pub mod code_review;
pub mod db_migration;
pub mod dep_update;
pub mod doc_generation;
pub mod formatter;
pub mod perf_analysis;
pub mod refactor;
pub mod security_scan;
pub mod test_generation;

/// Skill 输入参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInput {
    /// 待处理代码（可选，部分 Skill 如 api_design 可空）
    #[serde(default)]
    pub code: String,
    /// 代码语言（rust/ts/python/...）
    #[serde(default)]
    pub language: String,
    /// 文件路径（可选）
    #[serde(default)]
    pub file_path: String,
    /// 用户指令（自然语言需求）
    #[serde(default)]
    pub instruction: String,
    /// 附加上下文（如依赖清单、错误日志等）
    #[serde(default)]
    pub context: String,
}

/// Skill 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    /// Skill 名称
    pub skill_name: String,
    /// 云端 API 返回内容
    pub content: String,
    /// 实际使用的 provider
    pub provider: String,
    /// 实际使用的模型
    pub model_used: String,
    /// token 用量（若可用）
    pub tokens_used: Option<i64>,
}

/// Skill 执行上下文 — 封装云端 API 路由所需的全部依赖
///
/// 由命令层（`yuan_skill_execute`）构造，传入 Skill 的 `execute()`。
/// Skill 不直接持有 pool/mek_manager，避免 trait 对象生命周期复杂化。
pub struct SkillExecutionContext {
    pub router: Arc<CloudApiRouter>,
    pub pool: SqlitePool,
    pub mek_manager: Arc<RwLock<MekManager>>,
    pub user_id: i64,
    /// 云端 API provider（必须为 CLOUD_API_PROVIDERS 之一）
    pub provider: String,
    /// 具体模型名
    pub model_name: String,
}

impl SkillExecutionContext {
    /// 调用云端 API（强制云端，禁止本地底层智能模型）
    ///
    /// 复用 CloudApiRouter::route_programming_request，由其 validate_cloud_only() 守卫。
    pub async fn call_cloud(
        &self,
        system_prompt: String,
        prompt: String,
        skill_name: &str,
    ) -> Result<SkillOutput, AppError> {
        let req = ProgrammingRequest {
            provider: self.provider.clone(),
            model_name: self.model_name.clone(),
            prompt,
            system_prompt: Some(system_prompt),
            temperature: Some(0.2),
            max_tokens: Some(4096),
            stream: false,
            conversation_id: None,
            agent_id: Some(format!("skill_{}", skill_name)),
        };

        let resp = self
            .router
            .route_programming_request(&self.pool, &self.mek_manager, self.user_id, req)
            .await?;

        Ok(SkillOutput {
            skill_name: skill_name.to_string(),
            content: resp.content,
            provider: resp.provider,
            model_used: resp.model_used,
            tokens_used: resp.tokens_used,
        })
    }
}

/// 内置 Skill trait — 每个 Skill 实现此接口
///
/// 强制规则：`execute()` 必须通过 `ctx.call_cloud()` 走云端 API，
/// 禁止调用本地底层智能模型（项目核心设计意图 §八 8.1.1）。
#[async_trait]
pub trait BuiltinSkill: Send + Sync {
    /// Skill 唯一标识（snake_case，如 `refactor` / `test_generation`）
    fn name(&self) -> &str;
    /// 完整描述
    fn description(&self) -> &str;
    /// 简短描述（市场展示用）
    fn short_description(&self) -> &str;
    /// 市场分类：编程/测试/安全/文档/性能/架构/调试/迁移
    fn category(&self) -> &str;
    /// 触发词
    fn trigger_patterns(&self) -> Vec<String>;
    /// 优先级（0-10，越高越优先）
    fn priority(&self) -> i32 {
        8
    }
    /// 文件匹配模式（可空）
    fn file_patterns(&self) -> Vec<String> {
        Vec::new()
    }
    /// 系统提示词（定义 Skill 的角色与输出规范）
    fn system_prompt(&self) -> String;
    /// 由输入构造用户 prompt
    fn build_prompt(&self, input: &SkillInput) -> String;
    /// 执行 Skill — 调用云端 API
    async fn execute(&self, input: SkillInput, ctx: &SkillExecutionContext) -> Result<SkillOutput, AppError>;
}

/// 内置 Skill 注册表
pub struct BuiltinSkillRegistry {
    skills: Vec<Box<dyn BuiltinSkill>>,
}

impl BuiltinSkillRegistry {
    /// 构造包含全部 10 个内置 Skill 的注册表
    pub fn new() -> Self {
        let skills: Vec<Box<dyn BuiltinSkill>> = vec![
            Box::new(refactor::RefactorSkill),
            Box::new(test_generation::TestGenerationSkill),
            Box::new(doc_generation::DocGenerationSkill),
            Box::new(formatter::FormatterSkill),
            Box::new(dep_update::DepUpdateSkill),
            Box::new(security_scan::SecurityScanSkill),
            Box::new(perf_analysis::PerfAnalysisSkill),
            Box::new(api_design::ApiDesignSkill),
            Box::new(db_migration::DbMigrationSkill),
            Box::new(code_review::CodeReviewSkill),
        ];
        Self { skills }
    }

    /// 按 name 查找 Skill
    pub fn find(&self, name: &str) -> Option<&dyn BuiltinSkill> {
        self.skills
            .iter()
            .find(|s| s.name() == name)
            .map(|s| s.as_ref())
    }

    /// 列出全部 Skill 名称
    pub fn list_names(&self) -> Vec<String> {
        self.skills.iter().map(|s| s.name().to_string()).collect()
    }

    /// 列出全部 Skill 元数据（市场展示用）
    pub fn list_metadata(&self) -> Vec<BuiltinSkillInfo> {
        self.skills
            .iter()
            .map(|s| BuiltinSkillInfo {
                name: s.name().to_string(),
                description: s.description().to_string(),
                short_description: s.short_description().to_string(),
                category: s.category().to_string(),
                trigger_patterns: s.trigger_patterns(),
                file_patterns: s.file_patterns(),
                priority: s.priority(),
            })
            .collect()
    }

    /// 执行指定 Skill
    pub async fn execute(
        &self,
        name: &str,
        input: SkillInput,
        ctx: &SkillExecutionContext,
    ) -> Result<SkillOutput, AppError> {
        let skill = self.find(name).ok_or_else(|| {
            AppError::Validation(format!("内置 Skill '{}' 不存在", name))
        })?;
        skill.execute(input, ctx).await
    }
}

impl Default for BuiltinSkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 内置 Skill 元数据（市场展示用，与 SkillMarketEntry 字段对齐）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinSkillInfo {
    pub name: String,
    pub description: String,
    pub short_description: String,
    pub category: String,
    pub trigger_patterns: Vec<String>,
    pub file_patterns: Vec<String>,
    pub priority: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_10_skills() {
        let reg = BuiltinSkillRegistry::new();
        assert_eq!(reg.list_names().len(), 10, "应包含 10 个内置 Skill");
    }

    #[test]
    fn test_registry_contains_all_required_names() {
        let reg = BuiltinSkillRegistry::new();
        let names = reg.list_names();
        for required in [
            "refactor",
            "test_generation",
            "doc_generation",
            "formatter",
            "dep_update",
            "security_scan",
            "perf_analysis",
            "api_design",
            "db_migration",
            "code_review",
        ] {
            assert!(
                names.iter().any(|n| n == required),
                "缺少内置 Skill: {}",
                required
            );
        }
    }

    #[test]
    fn test_find_skill() {
        let reg = BuiltinSkillRegistry::new();
        assert!(reg.find("refactor").is_some());
        assert!(reg.find("nonexistent_skill").is_none());
    }
}
