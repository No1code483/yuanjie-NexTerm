//! api_design Skill — API 设计（根据需求生成 API 接口设计）
//!
//! 强制规则：通过 CloudApiRouter 走云端 API，禁止本地底层智能模型。

use async_trait::async_trait;

use crate::error::app_error::AppError;
use crate::skills::builtin::{BuiltinSkill, SkillExecutionContext, SkillInput, SkillOutput};

pub struct ApiDesignSkill;

#[async_trait]
impl BuiltinSkill for ApiDesignSkill {
    fn name(&self) -> &str {
        "api_design"
    }

    fn description(&self) -> &str {
        "根据需求生成 API 接口设计：RESTful/GraphQL 规范、参数命名、版本兼容性、错误码设计"
    }

    fn short_description(&self) -> &str {
        "API 设计"
    }

    fn category(&self) -> &str {
        "架构"
    }

    fn trigger_patterns(&self) -> Vec<String> {
        vec![
            "api".into(),
            "接口".into(),
            "endpoint".into(),
            "restful".into(),
        ]
    }

    fn priority(&self) -> i32 {
        7
    }

    fn system_prompt(&self) -> String {
        "你是一名 API 架构师，擅长 RESTful 与 GraphQL 接口设计。\
         请根据用户的需求生成 API 接口设计方案。\
         要求：\
         1. 遵循 RESTful 规范（资源命名、HTTP 方法、状态码）；\
         2. 列出每个接口：方法、路径、请求参数、响应示例、错误码；\
         3. 考虑版本兼容性、分页、鉴权；\
         4. 输出 OpenAPI 3.0 风格的接口定义（YAML 或 Markdown 表格均可）。"
            .to_string()
    }

    fn build_prompt(&self, input: &SkillInput) -> String {
        format!(
            "# API 设计任务\n\n\
             ## 目标语言/框架\n{}\n\n\
             ## 需求描述\n{}\n\n\
             ## 附加上下文\n{}",
            input.language,
            if input.instruction.is_empty() {
                if input.code.is_empty() {
                    "请描述需要设计的 API 需求".to_string()
                } else {
                    input.code.clone()
                }
            } else {
                input.instruction.clone()
            },
            input.context,
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
