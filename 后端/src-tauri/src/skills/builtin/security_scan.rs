//! security_scan Skill — 安全扫描（扫描代码安全漏洞）
//!
//! 强制规则：通过 CloudApiRouter 走云端 API，禁止本地底层智能模型。

use async_trait::async_trait;

use crate::error::app_error::AppError;
use crate::skills::builtin::{BuiltinSkill, SkillExecutionContext, SkillInput, SkillOutput};

pub struct SecurityScanSkill;

#[async_trait]
impl BuiltinSkill for SecurityScanSkill {
    fn name(&self) -> &str {
        "security_scan"
    }

    fn description(&self) -> &str {
        "扫描代码安全漏洞：注入攻击、XSS、硬编码密钥、不安全配置、权限绕过等风险"
    }

    fn short_description(&self) -> &str {
        "安全扫描"
    }

    fn category(&self) -> &str {
        "安全"
    }

    fn trigger_patterns(&self) -> Vec<String> {
        vec![
            "security".into(),
            "安全".into(),
            "audit".into(),
            "vulnerability".into(),
            "漏洞".into(),
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
        "你是一名应用安全工程师，擅长代码安全审计。\
         请扫描用户提供的代码，识别安全漏洞。\
         要求：\
         1. 按 OWASP Top 10 分类列出发现的问题；\
         2. 每个问题给出：位置（行号/函数）、严重级别（高/中/低）、描述、修复建议；\
         3. 若无问题，明确说明代码未发现明显安全风险；\
         4. 不要修改代码，仅给出审计报告。"
            .to_string()
    }

    fn build_prompt(&self, input: &SkillInput) -> String {
        format!(
            "# 安全扫描任务\n\n\
             ## 语言\n{}\n\n\
             ## 文件\n{}\n\n\
             ## 关注点\n{}\n\n\
             ## 待扫描代码\n```\n{}\n```",
            input.language,
            input.file_path,
            if input.instruction.is_empty() {
                "全量安全扫描".to_string()
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
