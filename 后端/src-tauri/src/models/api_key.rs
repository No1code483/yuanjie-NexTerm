//! Yuan Code v3.1 Task 3.5.1 — 云端 API Key 模型
//!
//! 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.5.1
//!       + 项目核心设计意图 §三（Yuan Code 编程 AI 必须走云端 API 模型）
//!
//! 与 ai_models 表的核心区别：本表按 provider 维度集中存储云端编程 API Key，
//! 与本地 ollama 模型彻底解耦，从数据层强制实现"编程任务禁用本地底层智能模型"。

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 数据库行：api_keys 表映射
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKey {
    pub id: i64,
    pub provider: String,
    pub display_name: Option<String>,
    /// AES-GCM 密文（与 api_key_nonce 配对，复用 crypto::aes_gcm + MEK）
    pub api_key_enc: Vec<u8>,
    pub api_key_nonce: Vec<u8>,
    pub api_url: Option<String>,
    /// 强制为 1：仅允许云端 API，禁止本地底层智能模型用于编程生成
    pub is_cloud_only: bool,
    pub is_enabled: bool,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    /// 安全审计修复（多用户隔离批次 1）：所有者用户 ID（v116 迁移新增）
    pub user_id: i64,
}

/// 对外响应（不暴露密文）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyResponse {
    pub id: i64,
    pub provider: String,
    pub display_name: Option<String>,
    pub api_url: Option<String>,
    pub is_cloud_only: bool,
    pub is_enabled: bool,
    pub has_api_key: bool,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ApiKey {
    pub fn to_response(&self) -> ApiKeyResponse {
        ApiKeyResponse {
            id: self.id,
            provider: self.provider.clone(),
            display_name: self.display_name.clone(),
            api_url: self.api_url.clone(),
            is_cloud_only: self.is_cloud_only,
            is_enabled: self.is_enabled,
            has_api_key: !self.api_key_enc.is_empty(),
            last_used_at: self.last_used_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// 新增 / 更新 API Key 请求（明文，后端加密后落盘）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertApiKeyRequest {
    pub provider: String,
    pub display_name: Option<String>,
    /// 明文 API Key（仅用于传输；落盘前由 cloud_api_router 调用 aes_gcm::encrypt）
    pub api_key_plain: String,
    pub api_url: Option<String>,
    pub is_enabled: Option<bool>,
}

impl UpsertApiKeyRequest {
    pub fn validate(&self) -> Result<(), String> {
        let provider = self.provider.trim().to_lowercase();
        if provider.is_empty() {
            return Err("provider 不能为空".into());
        }
        // 强制约束：仅允许云端 API provider，禁止本地底层智能模型
        if !CLOUD_API_PROVIDERS.contains(&provider.as_str()) {
            return Err(format!(
                "Yuan Code 编程 AI 仅允许云端 API provider（{}），禁止本地底层智能模型（如 ollama）",
                CLOUD_API_PROVIDERS.join(", ")
            ));
        }
        if self.api_key_plain.trim().is_empty() {
            return Err("api_key_plain 不能为空".into());
        }
        if let Some(ref url) = self.api_url {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(format!("api_url 必须以 http:// 或 https:// 开头: {}", url));
            }
        }
        Ok(())
    }
}

/// 允许的云端 API provider 白名单（强制约束：禁用本地底层智能模型用于编程生成）
///
/// 依据：项目核心设计意图 §三 / §七 — Yuan Code 编程 AI 必须走云端 API 模型
pub const CLOUD_API_PROVIDERS: &[&str] = &[
    "openai",
    "anthropic",
    "azure",
    "deepseek",
    "moonshot",
    "zhipu",
    "qwen",
    "custom",
];

/// 默认 provider API URL（与 ai_model::get_provider_default_url 对齐）
pub fn get_cloud_provider_default_url(provider: &str) -> Option<&'static str> {
    match provider {
        "openai" => Some("https://api.openai.com/v1"),
        "azure" => None,
        "anthropic" => Some("https://api.anthropic.com/v1"),
        "deepseek" => Some("https://api.deepseek.com/v1"),
        "moonshot" => Some("https://api.moonshot.cn/v1"),
        "zhipu" => Some("https://open.bigmodel.cn/api/paas/v4"),
        "qwen" => Some("https://dashscope.aliyuncs.com/compatible-mode/v1"),
        "custom" => None,
        _ => None,
    }
}

/// 默认推荐模型（每个 provider 的"编程友好"默认模型）
pub fn get_cloud_provider_default_model(provider: &str) -> Option<&'static str> {
    match provider {
        "openai" => Some("gpt-4o"),
        "anthropic" => Some("claude-sonnet-4-20250514"),
        "deepseek" => Some("deepseek-chat"),
        "moonshot" => Some("moonshot-v1-32k"),
        "zhipu" => Some("glm-4-plus"),
        "qwen" => Some("qwen-max"),
        _ => None,
    }
}
