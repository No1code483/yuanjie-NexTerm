//! doc_generation Skill — 文档生成（根据代码生成文档注释）
//!
//! 强制规则：通过 CloudApiRouter 走云端 API，禁止本地底层智能模型。

use async_trait::async_trait;

use crate::error::app_error::AppError;
use crate::skills::builtin::{BuiltinSkill, SkillExecutionContext, SkillInput, SkillOutput};

pub struct DocGenerationSkill;

#[async_trait]
impl BuiltinSkill for DocGenerationSkill {
    fn name(&self) -> &str {
        "doc_generation"
    }

    fn description(&self) -> &str {
        "为代码与 API 生成文档注释、README 与变更日志，提升代码可维护性"
    }

    fn short_description(&self) -> &str {
        "文档生成"
    }

    fn category(&self) -> &str {
        "文档"
    }

    fn trigger_patterns(&self) -> Vec<String> {
        vec![
            "doc".into(),
            "文档".into(),
            "readme".into(),
            "注释".into(),
        ]
    }

    fn priority(&self) -> i32 {
        7
    }

    fn system_prompt(&self) -> String {
        "你是一名技术文档工程师。\
         请根据用户提供的代码生成规范的文档注释。\
         要求：\
         1. 遵循目标语言的文档规范（Rust: ///, TS: JSDoc, Python: docstring）；\
         2. 描述函数/类/模块的用途、参数、返回值、异常与示例；\
         3. 语言简洁准确，避免冗余；\
         4. 输出带文档注释的完整代码块。"
            .to_string()
    }

    fn build_prompt(&self, input: &SkillInput) -> String {
        format!(
            "# 文档生成任务\n\n\
             ## 语言\n{}\n\n\
             ## 文件\n{}\n\n\
             ## 特殊要求\n{}\n\n\
             ## 待文档化代码\n```\n{}\n```",
            input.language,
            input.file_path,
            if input.instruction.is_empty() {
                "请为所有公共 API 生成文档注释".to_string()
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
