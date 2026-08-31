//! Yuan Code v3.2 Task 3.4.1 — 模型路由配置数据模型
//!
//! 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.4.1（Phase 6 多模型与协作）
//!       + 项目核心设计意图 §三 / §八（强制规则 8.1.1：Yuan Code 编程 AI 必须走云端 API）
//!
//! 与 api_key.rs 的关系：
//! - api_key.rs：按 provider 维度存储云端 API Key（密文）
//! - model_routing.rs：按 task_type 维度存储"使用哪个 provider+model"
//!
//! 强制约束（is_cloud_only = true）：
//! 编程任务路由规则只能选云端 API provider，禁止选本地底层智能模型（ollama 等）。
//! 守卫由 service 层 + CLOUD_API_PROVIDERS 白名单双重强制。

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 任务类型枚举（路由规则的 key）
///
/// 强制约束：programming 必须路由到云端 API provider。
/// 其他任务类型（分析/审查/文档）也默认走云端 API，因为 Yuan Code 编程 AI 必须走云端 API。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    /// 编程任务（Agent 7 种类型的 coding/refactor/test/migration/debug 等代码生成）
    Programming,
    /// 代码分析（CodeAnalysisRequest / 项目索引摘要等）
    Analysis,
    /// 代码审查（review Agent / safety check 等）
    Review,
    /// 文档生成（documentation Agent / 注释生成等）
    Documentation,
    /// 通用兜底（未指定 task_type 时使用）
    General,
}

impl TaskType {
    /// 字符串形式（用于 SQL 查询 + IPC 通信）
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::Programming => "programming",
            TaskType::Analysis => "analysis",
            TaskType::Review => "review",
            TaskType::Documentation => "documentation",
            TaskType::General => "general",
        }
    }

    /// 从字符串解析（容错：未知值回退到 General）
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "programming" => TaskType::Programming,
            "analysis" => TaskType::Analysis,
            "review" => TaskType::Review,
            "documentation" => TaskType::Documentation,
            _ => TaskType::General,
        }
    }

    /// 列出全部任务类型字符串（用于前端 UI 渲染可选项）
    pub fn all_str() -> &'static [&'static str] {
        &["programming", "analysis", "review", "documentation", "general"]
    }
}

/// 数据库行：model_routing_rules 表映射
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ModelRoutingRule {
    pub id: i64,
    pub user_id: i64,
    pub task_type: String,
    pub provider: String,
    pub model_name: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
    pub is_enabled: bool,
    /// 强制云端：true = 不允许将 provider 改为本地底层智能模型（守卫由 service 层强制）
    pub is_cloud_only: bool,
    pub priority: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 新增/更新路由规则请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRoutingRuleRequest {
    pub task_type: String,
    pub provider: String,
    pub model_name: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
    pub is_enabled: Option<bool>,
}

impl UpsertRoutingRuleRequest {
    /// 校验（强制约束：仅允许云端 provider）
    ///
    /// 守卫点 1：在 upsert 入口拒绝本地 provider（如 ollama），
    /// 守卫点 2 在 service::resolve_for_task_type 再次校验。
    pub fn validate(&self) -> Result<(), String> {
        let task_type = self.task_type.trim().to_lowercase();
        if task_type.is_empty() {
            return Err("task_type 不能为空".into());
        }
        if !TaskType::all_str().contains(&task_type.as_str()) {
            return Err(format!(
                "task_type 必须是 {} 之一",
                TaskType::all_str().join(", ")
            ));
        }

        let provider = self.provider.trim().to_lowercase();
        if provider.is_empty() {
            return Err("provider 不能为空".into());
        }
        // 强制约束（项目核心设计意图 §三/§八）：
        // 路由规则只能选云端 API provider，禁止本地底层智能模型用于编程生成
        if !CLOUD_ROUTING_PROVIDERS.contains(&provider.as_str()) {
            return Err(format!(
                "Yuan Code 模型路由规则仅允许云端 API provider（{}），\
                 禁止本地底层智能模型（如 ollama）[项目核心设计意图 §三/§八]",
                CLOUD_ROUTING_PROVIDERS.join(", ")
            ));
        }

        if self.model_name.trim().is_empty() {
            return Err("model_name 不能为空".into());
        }

        if let Some(t) = self.temperature {
            if !(0.0..=2.0).contains(&t) {
                return Err(format!("temperature 必须在 0.0-2.0 之间: {}", t));
            }
        }

        if let Some(m) = self.max_tokens {
            if m <= 0 {
                return Err(format!("max_tokens 必须大于 0: {}", m));
            }
        }

        Ok(())
    }
}

/// 路由解析结果（service 返回给 router）
#[derive(Debug, Clone)]
pub struct ResolvedRoute {
    pub task_type: String,
    pub provider: String,
    pub model_name: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
    /// 来源：rule（用户配置） | default（系统默认，rule 不存在或未启用时）
    pub source: RouteSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RouteSource {
    Rule,
    Default,
}

/// 允许用于路由的云端 provider 白名单
///
/// 复用 api_key::CLOUD_API_PROVIDERS，但此处独立常量以便在 model 层强制（不引入循环依赖）。
/// service 层会再调用 api_key::CLOUD_API_PROVIDERS 做最终守卫。
pub const CLOUD_ROUTING_PROVIDERS: &[&str] = &[
    "openai",
    "anthropic",
    "azure",
    "deepseek",
    "moonshot",
    "zhipu",
    "qwen",
    "custom",
];

/// 各 task_type 的默认 provider + model（用户未配置规则时回退）
///
/// 强制约束：默认值全部为云端 API provider，确保即使用户未配置也走云端 API
pub fn default_route_for_task(task_type: &str) -> (&'static str, &'static str, f64) {
    match task_type.to_lowercase().as_str() {
        "programming" => ("openai", "gpt-4o", 0.2),
        "analysis" => ("deepseek", "deepseek-chat", 0.0),
        "review" => ("anthropic", "claude-sonnet-4-20250514", 0.0),
        "documentation" => ("openai", "gpt-4o", 0.4),
        _ => ("openai", "gpt-4o", 0.3), // General
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_rejects_ollama_for_programming() {
        let req = UpsertRoutingRuleRequest {
            task_type: "programming".into(),
            provider: "ollama".into(),
            model_name: "qwen3:8b".into(),
            temperature: None,
            max_tokens: None,
            is_enabled: Some(true),
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("云端 API"), "应拒绝 ollama: {}", err);
        assert!(err.contains("禁止"), "应提示禁止本地模型: {}", err);
    }

    #[test]
    fn test_validate_accepts_openai_for_programming() {
        let req = UpsertRoutingRuleRequest {
            task_type: "programming".into(),
            provider: "openai".into(),
            model_name: "gpt-4o".into(),
            temperature: Some(0.2),
            max_tokens: None,
            is_enabled: Some(true),
        };
        req.validate().expect("openai 应通过校验");
    }

    #[test]
    fn test_validate_accepts_anthropic_for_review() {
        let req = UpsertRoutingRuleRequest {
            task_type: "review".into(),
            provider: "anthropic".into(),
            model_name: "claude-sonnet-4-20250514".into(),
            temperature: None,
            max_tokens: None,
            is_enabled: None,
        };
        req.validate().expect("anthropic 应通过校验");
    }

    #[test]
    fn test_validate_rejects_invalid_task_type() {
        let req = UpsertRoutingRuleRequest {
            task_type: "unknown_type".into(),
            provider: "openai".into(),
            model_name: "gpt-4o".into(),
            temperature: None,
            max_tokens: None,
            is_enabled: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_invalid_temperature() {
        let req = UpsertRoutingRuleRequest {
            task_type: "programming".into(),
            provider: "openai".into(),
            model_name: "gpt-4o".into(),
            temperature: Some(3.0),
            max_tokens: None,
            is_enabled: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_default_route_always_cloud() {
        for &tt in TaskType::all_str() {
            let (provider, _, _) = default_route_for_task(tt);
            assert!(
                CLOUD_ROUTING_PROVIDERS.contains(&provider),
                "task_type {} 默认 provider {} 必须是云端 API",
                tt,
                provider
            );
        }
    }

    #[test]
    fn test_task_type_roundtrip() {
        for &tt in TaskType::all_str() {
            let parsed = TaskType::from_str(tt);
            assert_eq!(parsed.as_str(), tt);
        }
        // 未知值回退到 General
        assert_eq!(TaskType::from_str("unknown").as_str(), "general");
    }
}
