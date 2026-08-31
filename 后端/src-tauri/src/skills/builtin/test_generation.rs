//! test_generation Skill — 测试生成（根据代码生成测试用例）
//!
//! 强制规则：通过 CloudApiRouter 走云端 API，禁止本地底层智能模型。

use async_trait::async_trait;

use crate::error::app_error::AppError;
use crate::skills::builtin::{BuiltinSkill, SkillExecutionContext, SkillInput, SkillOutput};

pub struct TestGenerationSkill;

#[async_trait]
impl BuiltinSkill for TestGenerationSkill {
    fn name(&self) -> &str {
        "test_generation"
    }

    fn description(&self) -> &str {
        "为代码自动生成单元测试与集成测试，覆盖正常路径、边界情况与错误路径"
    }

    fn short_description(&self) -> &str {
        "测试生成"
    }

    fn category(&self) -> &str {
        "测试"
    }

    fn trigger_patterns(&self) -> Vec<String> {
        vec![
            "test".into(),
            "测试".into(),
            "coverage".into(),
            "unit test".into(),
        ]
    }

    fn priority(&self) -> i32 {
        9
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
        "你是一名测试工程师，擅长为代码编写高质量测试。\
         请根据用户提供的代码生成对应的测试用例。\
         要求：\
         1. 覆盖正常路径、边界值与错误路径；\
         2. 使用目标语言的主流测试框架（Rust: #[test], TS: jest/vitest, Python: pytest）；\
         3. 测试命名清晰，断言明确；\
         4. 输出完整可编译的测试代码块，不要省略。"
            .to_string()
    }

    fn build_prompt(&self, input: &SkillInput) -> String {
        format!(
            "# 测试生成任务\n\n\
             ## 语言\n{}\n\n\
             ## 文件\n{}\n\n\
             ## 附加要求\n{}\n\n\
             ## 待测试代码\n```\n{}\n```",
            input.language,
            input.file_path,
            if input.instruction.is_empty() {
                "请生成尽可能完整的测试覆盖".to_string()
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
