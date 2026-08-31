//! Yuan Code v3.1 Task 3.1.1 — Agent 生命周期 trait
//!
//! 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.1.1
//!       + 参考 codex-main/codex-rs/core/src/codex_thread.rs 的 Phase 设计
//!
//! 设计：所有 7 种 Agent 类型实现统一的生命周期接口
//!   Pending → Planning → PlanReview → Executing → AwaitingReview → Completed/Rejected
//!
//! 强制约束：
//! - execute() 内部必须通过 CloudApiRouter 调用云端 API（禁止本地底层智能模型）
//! - execute() 内部必须在 SandboxManager 内执行代码（沙箱隔离）
//! - 敏感步骤必须经 safety_check 通过后才执行

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::agent::types::{
    AgentFileDiff, AgentPhase, AgentPlan, AgentResult, AgentType,
};
use crate::crypto::mek_manager::MekManager;
use crate::error::app_error::AppError;
use crate::services::cloud_api_router::{CloudApiRouter, ProgrammingRequest};

/// Agent 生命周期 trait — 对标 codex_thread 的统一抽象
///
/// 7 种 Agent 类型（CodingAgent / RefactorAgent / ...）必须实现此 trait
#[async_trait::async_trait]
pub trait AgentLifecycle: Send + Sync {
    /// Agent 类型
    fn agent_type(&self) -> AgentType;

    /// 当前阶段
    fn phase(&self) -> AgentPhase;

    /// Agent ID
    fn agent_id(&self) -> &str;

    /// 制定执行计划
    ///
    /// 内部必须通过 CloudApiRouter 调用云端 API 生成计划（强制约束）
    async fn plan(
        &mut self,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        task_prompt: &str,
    ) -> Result<AgentPlan, AppError>;

    /// 在沙箱内执行计划
    ///
    /// 内部必须经 safety_check 通过后执行；所有 AI 调用走 CloudApiRouter
    async fn execute(
        &mut self,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
    ) -> Result<AgentResult, AppError>;

    /// 获取最近一次执行结果（用于 review UI）
    fn result(&self) -> Option<&AgentResult>;

    /// 重置（清空计划与结果，回到 Pending）
    fn reset(&mut self);
}

/// 通用 Agent 工具函数 — 各具体 Agent 共用
pub struct AgentUtils;

impl AgentUtils {
    /// 当前时间戳（秒）
    pub fn now_secs() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    /// 调用 CloudApiRouter 生成计划骨架（共用方法）
    ///
    /// 各具体 Agent 通过此方法调用云端 API；具体的 prompt 模板由 Agent 类型决定。
    pub async fn call_cloud_for_plan(
        router: &CloudApiRouter,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        agent_id: &str,
        agent_type: AgentType,
        task_prompt: &str,
    ) -> Result<String, AppError> {
        let plan_prompt = format!(
            "# Task: Create an execution plan\n\
             Agent type: {}\n\
             Agent description: {}\n\n\
             Task:\n{}\n\n\
             Output a structured plan in markdown with numbered steps. \
             For each step, list target files if applicable, and mark whether \
             user confirmation is required (sensitive operations like file deletion, \
             dependency changes, or shell execution).",
            agent_type.as_str(),
            agent_type.description(),
            task_prompt
        );

        let req = ProgrammingRequest {
            provider: "openai".into(), // 默认 provider，前端可通过 ModelSelector 配置
            model_name: "gpt-4o".into(),
            prompt: plan_prompt,
            system_prompt: Some(agent_type.default_system_prompt()),
            temperature: Some(0.3),
            max_tokens: Some(2048),
            stream: false,
            conversation_id: None,
            agent_id: Some(agent_id.to_string()),
        };

        let resp = router
            .route_programming_request(pool, mek_manager, user_id, req)
            .await?;
        Ok(resp.content)
    }

    /// 调用 CloudApiRouter 生成代码 diff（共用方法）
    pub async fn call_cloud_for_diff(
        router: &CloudApiRouter,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        agent_id: &str,
        agent_type: AgentType,
        file_path: &str,
        original_content: &str,
        task_prompt: &str,
        provider: &str,
        model_name: &str,
    ) -> Result<AgentFileDiff, AppError> {
        let prompt = format!(
            "# Task: Modify file\n\
             File: {}\n\n\
             ## Current content:\n```\n{}\n```\n\n\
             ## Instruction:\n{}\n\n\
             Output the COMPLETE new file content in a single code block. \
             Do not output partial snippets or explanations outside the block.",
            file_path, original_content, task_prompt
        );

        let req = ProgrammingRequest {
            provider: provider.to_string(),
            model_name: model_name.to_string(),
            prompt,
            system_prompt: Some(agent_type.default_system_prompt()),
            temperature: Some(0.2),
            max_tokens: Some(4096),
            stream: false,
            conversation_id: None,
            agent_id: Some(agent_id.to_string()),
        };

        let resp = router
            .route_programming_request(pool, mek_manager, user_id, req)
            .await?;

        let modified_content = extract_code_block(&resp.content);
        let (added, removed) = count_diff_lines(original_content, &modified_content);
        let unified_diff = simple_unified_diff(original_content, &modified_content);

        Ok(AgentFileDiff {
            path: file_path.to_string(),
            unified_diff,
            original_content: Some(original_content.to_string()),
            modified_content: Some(modified_content),
            added_lines: added,
            removed_lines: removed,
            is_new_file: original_content.is_empty(),
        })
    }
}

/// 从 LLM 响应中提取代码块内容
///
/// 支持 ```language ... ``` 与纯文本两种格式
pub fn extract_code_block(raw: &str) -> String {
    // 查找第一个 ``` 与下一个 ```
    let bytes = raw.as_bytes();
    let start_marker = b"```";
    if let Some(start_idx) = find_subslice(bytes, start_marker) {
        let after_start = start_idx + 3;
        // 跳过行尾的 language 标记（如 ```rust\n）
        let line_end = bytes[after_start..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|p| after_start + p + 1)
            .unwrap_or(after_start);
        if let Some(end_idx) = find_subslice(&bytes[line_end..], start_marker) {
            return raw[line_end..line_end + end_idx].trim_end().to_string();
        }
    }
    // 没有代码块，返回原内容（trimmed）
    raw.trim().to_string()
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

/// 统计行级 diff 的新增/删除行数（简化实现：按行集合差集）
pub fn count_diff_lines(original: &str, modified: &str) -> (u32, u32) {
    let orig_lines: std::collections::HashSet<&str> = original.lines().collect();
    let mod_lines: std::collections::HashSet<&str> = modified.lines().collect();
    let added = mod_lines.difference(&orig_lines).count() as u32;
    let removed = orig_lines.difference(&mod_lines).count() as u32;
    (added, removed)
}

/// 生成简化的 unified diff 文本
pub fn simple_unified_diff(original: &str, modified: &str) -> String {
    let mut out = String::new();
    for line in original.lines() {
        out.push('-');
        out.push_str(line);
        out.push('\n');
    }
    for line in modified.lines() {
        out.push('+');
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_code_block_with_lang() {
        let raw = "Here is the code:\n```rust\nfn main() {}\n```\nDone.";
        assert_eq!(extract_code_block(raw), "fn main() {}");
    }

    #[test]
    fn test_extract_code_block_without_lang() {
        let raw = "```\nhello\nworld\n```";
        assert_eq!(extract_code_block(raw), "hello\nworld");
    }

    #[test]
    fn test_extract_code_block_no_code_block_returns_trimmed() {
        let raw = "  plain text  ";
        assert_eq!(extract_code_block(raw), "plain text");
    }

    #[test]
    fn test_count_diff_lines_all_new() {
        let (added, removed) = count_diff_lines("", "a\nb\nc");
        assert_eq!(added, 3);
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_count_diff_lines_all_removed() {
        let (added, removed) = count_diff_lines("a\nb\nc", "");
        assert_eq!(added, 0);
        assert_eq!(removed, 3);
    }

    #[test]
    fn test_count_diff_lines_partial() {
        let (added, removed) = count_diff_lines("a\nb\nc", "a\nx\nc");
        assert_eq!(added, 1); // x 新增
        assert_eq!(removed, 1); // b 删除
    }

    #[test]
    fn test_simple_unified_diff_has_plus_minus() {
        let diff = simple_unified_diff("a\nb", "a\nc");
        assert!(diff.contains("-a"));
        assert!(diff.contains("-b"));
        assert!(diff.contains("+a"));
        assert!(diff.contains("+c"));
    }

    #[test]
    fn test_agent_utils_now_secs_monotonic_non_decreasing() {
        let t1 = AgentUtils::now_secs();
        let t2 = AgentUtils::now_secs();
        assert!(t2 >= t1, "时间戳应单调非减");
    }
}
