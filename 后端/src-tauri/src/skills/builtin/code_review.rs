//! code_review Skill — 代码审查（审查代码质量）
//!
//! 强制规则：通过 CloudApiRouter 走云端 API，禁止本地底层智能模型。

use async_trait::async_trait;

use crate::error::app_error::AppError;
use crate::skills::builtin::{BuiltinSkill, SkillExecutionContext, SkillInput, SkillOutput};

pub struct CodeReviewSkill;

#[async_trait]
impl BuiltinSkill for CodeReviewSkill {
    fn name(&self) -> &str {
        "code_review"
    }

    fn description(&self) -> &str {
        "审查代码质量：检查潜在问题、最佳实践违规、可维护性与性能，给出改进建议"
    }

    fn short_description(&self) -> &str {
        "代码审查"
    }

    fn category(&self) -> &str {
        "编程"
    }

    fn trigger_patterns(&self) -> Vec<String> {
        vec![
            "review".into(),
            "审查".into(),
            "pr".into(),
            "code review".into(),
        ]
    }

    fn priority(&self) -> i32 {
        10
    }

    fn file_patterns(&self) -> Vec<String> {
        vec![
            "*.rs".into(),
            "*.ts".into(),
            "*.py".into(),
            "*.go".into(),
        ]
    }

    fn system_prompt(&self) -> String {
        "你是一名资深代码审查员。\
         请审查用户提供的代码变更，检查潜在问题与改进点。\
         要求：\
         1. 按类别分组：正确性 / 可读性 / 性能 / 安全 / 最佳实践；\
         2. 每条意见给出：位置、严重级别（阻塞/建议）、说明、修改建议；\
         3. 优先标注阻塞级问题；\
         4. 若代码质量良好，明确说明无需修改。"
            .to_string()
    }

    fn build_prompt(&self, input: &SkillInput) -> String {
        format!(
            "# 代码审查任务\n\n\
             ## 语言\n{}\n\n\
             ## 文件\n{}\n\n\
             ## 审查重点\n{}\n\n\
             ## 待审查代码\n```\n{}\n```",
            input.language,
            input.file_path,
            if input.instruction.is_empty() {
                "全面审查".to_string()
            } else {
                input.instruction.clone()
            },
            input.code,
        )
    }

    async fn execute(
        &self,
        input: SkillInput,
        ctx: &SkillExecutionContext,
    ) -> Result<SkillOutput, AppError> {
        let prompt = self.build_prompt(&input);
        ctx.call_cloud(self.system_prompt(), prompt, self.name()).await
    }
}
