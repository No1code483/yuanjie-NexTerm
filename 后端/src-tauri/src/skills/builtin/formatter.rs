//! formatter Skill — 代码格式化
//!
//! 策略：优先调用本地格式化工具（rustfmt / prettier / black），失败时回退到云端 API。
//!
//! 强制规则（项目核心设计意图 §三 / §八）：
//! - 本地工具（rustfmt/prettier/black）是确定性格式化器，非 AI 生成，不违反"云端 API"约束；
//! - 当本地工具不可用时，回退走 CloudApiRouter 云端 API（禁止本地底层智能模型）。

use async_trait::async_trait;

use crate::error::app_error::AppError;
use crate::skills::builtin::{BuiltinSkill, SkillExecutionContext, SkillInput, SkillOutput};

pub struct FormatterSkill;

#[async_trait]
impl BuiltinSkill for FormatterSkill {
    fn name(&self) -> &str {
        "formatter"
    }

    fn description(&self) -> &str {
        "代码格式化：优先调用 rustfmt/prettier/black 本地工具，不可用时回退云端 API 格式化建议"
    }

    fn short_description(&self) -> &str {
        "代码格式化"
    }

    fn category(&self) -> &str {
        "编程"
    }

    fn trigger_patterns(&self) -> Vec<String> {
        vec![
            "format".into(),
            "格式化".into(),
            "prettier".into(),
            "rustfmt".into(),
        ]
    }

    fn priority(&self) -> i32 {
        6
    }

    fn file_patterns(&self) -> Vec<String> {
        vec![
            "*.rs".into(),
            "*.ts".into(),
            "*.tsx".into(),
            "*.js".into(),
            "*.py".into(),
        ]
    }

    fn system_prompt(&self) -> String {
        "你是一名代码格式化助手。\
         请按照目标语言的主流风格规范（Rust: rustfmt 默认, TS: prettier 默认, Python: black）\
         对用户提供的代码进行格式化。\
         要求：\
         1. 不改变代码语义；\
         2. 仅调整缩进、空格、换行、引号等风格；\
         3. 直接输出格式化后的完整代码块，不要解释。"
            .to_string()
    }

    fn build_prompt(&self, input: &SkillInput) -> String {
        format!(
            "# 格式化任务\n\n\
             ## 语言\n{}\n\n\
             ## 待格式化代码\n```\n{}\n```",
            input.language, input.code,
        )
    }

    async fn execute(
        &self,
        input: SkillInput,
        ctx: &SkillExecutionContext,
    ) -> Result<SkillOutput, AppError> {
        // 1. 尝试本地格式化工具（确定性，非 AI）
        if let Some(formatted) = try_local_formatter(&input).await {
            return Ok(SkillOutput {
                skill_name: self.name().to_string(),
                content: formatted,
                provider: "local".to_string(),
                model_used: "rustfmt/prettier/black".to_string(),
                tokens_used: None,
            });
        }

        // 2. 本地工具不可用 → 回退云端 API（强制云端，禁止本地底层智能模型）
        let prompt = self.build_prompt(&input);
        ctx.call_cloud(self.system_prompt(), prompt, self.name()).await
    }
}

/// 尝试本地格式化工具：按语言选择 rustfmt / prettier / black
///
/// 返回 Some(格式化后代码) 表示成功，None 表示工具不可用或失败（应回退云端）。
async fn try_local_formatter(input: &SkillInput) -> Option<String> {
    let code = input.code.as_str();
    if code.is_empty() {
        return None;
    }

    let lang = input.language.to_lowercase();
    let (cmd, args) = match lang.as_str() {
        "rust" | "rs" => ("rustfmt", vec!["--edition".to_string(), "2021".to_string()]),
        "typescript" | "ts" | "tsx" | "javascript" | "js" | "jsx" => {
            ("prettier", vec!["--parser".to_string(), lang.clone()])
        }
        "python" | "py" => ("black", vec!["-".to_string()]),
        _ => return None,
    };

    let mut output = tokio::process::Command::new(cmd)
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;

    use tokio::io::AsyncWriteExt;
    let (mut stdin, child) = (output.stdin.take()?, output);
    let code_bytes = code.as_bytes().to_vec();
    let write_task = tokio::spawn(async move {
        let _ = stdin.write_all(&code_bytes).await;
        stdin
    });
    let _ = write_task.await;
    let out = child.wait_with_output().await.ok()?;

    if !out.status.success() {
        return None;
    }
    let formatted = String::from_utf8(out.stdout).ok()?;
    if formatted.is_empty() {
        None
    } else {
        Some(formatted)
    }
}
