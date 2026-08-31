//! perf_analysis Skill — 性能分析（分析代码性能瓶颈）
//!
//! 强制规则：通过 CloudApiRouter 走云端 API，禁止本地底层智能模型。

use async_trait::async_trait;

use crate::error::app_error::AppError;
use crate::skills::builtin::{BuiltinSkill, SkillExecutionContext, SkillInput, SkillOutput};

pub struct PerfAnalysisSkill;

#[async_trait]
impl BuiltinSkill for PerfAnalysisSkill {
    fn name(&self) -> &str {
        "perf_analysis"
    }

    fn description(&self) -> &str {
        "分析代码性能瓶颈：识别热点函数、不必要的内存分配、O(n²) 复杂度与 IO 瓶颈"
    }

    fn short_description(&self) -> &str {
        "性能分析"
    }

    fn category(&self) -> &str {
        "性能"
    }

    fn trigger_patterns(&self) -> Vec<String> {
        vec![
            "perf".into(),
            "性能".into(),
            "optimize".into(),
            "优化".into(),
            "bottleneck".into(),
        ]
    }

    fn priority(&self) -> i32 {
        8
    }

    fn file_patterns(&self) -> Vec<String> {
        vec!["*.rs".into(), "*.ts".into(), "*.py".into(), "*.go".into()]
    }

    fn system_prompt(&self) -> String {
        "你是一名性能优化专家。\
         请分析用户提供的代码，识别性能瓶颈并给出优化建议。\
         要求：\
         1. 列出每个性能问题：位置、类型（CPU/内存/IO）、严重级别、复杂度分析；\
         2. 给出优化后的代码片段对比（优化前 → 优化后）；\
         3. 说明优化的预期收益；\
         4. 不要重构无关代码。"
            .to_string()
    }

    fn build_prompt(&self, input: &SkillInput) -> String {
        format!(
            "# 性能分析任务\n\n\
             ## 语言\n{}\n\n\
             ## 文件\n{}\n\n\
             ## 附加上下文（性能数据/场景）\n{}\n\n\
             ## 用户指令\n{}\n\n\
             ## 待分析代码\n```\n{}\n```",
            input.language,
            input.file_path,
            input.context,
            if input.instruction.is_empty() {
                "请分析性能瓶颈".to_string()
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
