//! dep_update Skill — 依赖更新（检查并更新依赖版本）
//!
//! 强制规则：通过 CloudApiRouter 走云端 API，禁止本地底层智能模型。

use async_trait::async_trait;

use crate::error::app_error::AppError;
use crate::skills::builtin::{BuiltinSkill, SkillExecutionContext, SkillInput, SkillOutput};

pub struct DepUpdateSkill;

#[async_trait]
impl BuiltinSkill for DepUpdateSkill {
    fn name(&self) -> &str {
        "dep_update"
    }

    fn description(&self) -> &str {
        "检查项目依赖版本，识别过时依赖、已知漏洞版本，并生成更新后的依赖清单"
    }

    fn short_description(&self) -> &str {
        "依赖更新"
    }

    fn category(&self) -> &str {
        "编程"
    }

    fn trigger_patterns(&self) -> Vec<String> {
        vec![
            "dependency".into(),
            "依赖".into(),
            "update".into(),
            "upgrade".into(),
        ]
    }

    fn priority(&self) -> i32 {
        6
    }

    fn file_patterns(&self) -> Vec<String> {
        vec![
            "Cargo.toml".into(),
            "package.json".into(),
            "requirements.txt".into(),
            "go.mod".into(),
        ]
    }

    fn system_prompt(&self) -> String {
        "你是一名依赖管理专家。\
         请分析用户提供的依赖清单，识别可更新的依赖并给出更新建议。\
         要求：\
         1. 列出每个可更新依赖的当前版本与建议版本；\
         2. 标注是否存在已知重大破坏性变更（breaking change）；\
         3. 输出更新后的完整依赖清单代码块；\
         4. 若依赖清单为空，请提示用户提供 Cargo.toml / package.json 等文件内容。"
            .to_string()
    }

    fn build_prompt(&self, input: &SkillInput) -> String {
        format!(
            "# 依赖更新任务\n\n\
             ## 语言/生态\n{}\n\n\
             ## 用户指令\n{}\n\n\
             ## 当前依赖清单\n```\n{}\n```",
            input.language,
            if input.instruction.is_empty() {
                "请检查可更新的依赖".to_string()
            } else {
                input.instruction.clone()
            },
            if input.code.is_empty() {
                &input.context
            } else {
                &input.code
            },
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
