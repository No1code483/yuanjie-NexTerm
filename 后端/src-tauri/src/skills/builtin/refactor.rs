//! refactor Skill — 代码重构（调用云端 API 分析代码 + 提供重构建议）
//!
//! 强制规则：通过 CloudApiRouter 走云端 API，禁止本地底层智能模型。

use async_trait::async_trait;

use crate::error::app_error::AppError;
use crate::skills::builtin::{BuiltinSkill, SkillExecutionContext, SkillInput, SkillOutput};

pub struct RefactorSkill;

#[async_trait]
impl BuiltinSkill for RefactorSkill {
    fn name(&self) -> &str {
        "refactor"
    }

    fn description(&self) -> &str {
        "重构现有代码：提取方法、内联变量、简化条件、优化结构与可读性，保持外部行为不变"
    }

    fn short_description(&self) -> &str {
        "代码重构"
    }

    fn category(&self) -> &str {
        "编程"
    }

    fn trigger_patterns(&self) -> Vec<String> {
        vec![
            "refactor".into(),
            "重构".into(),
            "clean".into(),
            "simplify".into(),
        ]
    }

    fn priority(&self) -> i32 {
        8
    }

    fn file_patterns(&self) -> Vec<String> {
        vec!["*.rs".into(), "*.ts".into(), "*.py".into(), "*.go".into()]
    }

    fn system_prompt(&self) -> String {
        "你是一名资深软件工程师，擅长代码重构。\
         请分析用户提供的代码，给出重构建议与重构后的完整代码。\
         要求：\
         1. 保持外部行为不变（不改变公共 API 与输出）；\
         2. 提升可读性、降低复杂度、消除重复；\
         3. 先用要点列出重构点，再给出重构后的完整代码块；\
         4. 不要输出与重构无关的解释。"
            .to_string()
    }

    fn build_prompt(&self, input: &SkillInput) -> String {
        format!(
            "# 重构任务\n\n\
             ## 语言\n{}\n\n\
             ## 文件\n{}\n\n\
             ## 用户指令\n{}\n\n\
             ## 待重构代码\n```\n{}\n```",
            input.language,
            input.file_path,
            if input.instruction.is_empty() {
                "请给出整体重构建议".to_string()
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
