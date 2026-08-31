use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AiModel {
    pub id: i64,
    pub name: String,
    pub provider: String,
    pub api_url: Option<String>,
    pub api_key_enc: Option<Vec<u8>>,
    pub api_key_nonce: Option<Vec<u8>>,
    pub api_format: Option<String>,
    pub model_name: Option<String>,
    pub display_name: Option<String>,
    pub is_local: bool,
    pub multimodal: bool,
    pub system_prompt: Option<String>,
    pub context_window: Option<i64>,
    pub temperature: Option<f64>,
    pub created_at: i64,
    pub updated_at: i64,
    // spec ai-chat-enhancement Phase 1 §1.1: 模型健康检测字段（v114 迁移新增）
    pub status: String,
    pub last_health_check: Option<i64>,
    pub latency_ms: Option<i64>,
    // 安全审计修复（多用户隔离批次 1）：所有者用户 ID（v116 迁移新增）
    pub user_id: i64,
}

impl AiModel {
    pub fn to_response(&self) -> AiModelResponse {
        AiModelResponse {
            id: self.id,
            name: self.name.clone(),
            provider: self.provider.clone(),
            api_endpoint: self.api_url.clone(),
            api_format: self.api_format.clone().unwrap_or_else(|| {
                match self.provider.as_str() {
                    "anthropic" => "anthropic-messages".to_string(),
                    "ollama" => "ollama".to_string(),
                    _ => "openai-chat".to_string(),
                }
            }),
            model_name: self.model_name.clone().unwrap_or_default(),
            display_name: self.display_name.clone().unwrap_or_else(|| self.name.clone()),
            model_type: if self.is_local { "local".to_string() } else { "api".to_string() },
            multimodal: self.multimodal,
            system_prompt: self.system_prompt.clone(),
            context_window: self.context_window.unwrap_or(8192),
            temperature: self.temperature,
            status: self.status.clone(),
            latency_ms: self.latency_ms,
            last_health_check: self.last_health_check,
            has_api_key: self.api_key_enc.is_some(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModelResponse {
    pub id: i64,
    pub name: String,
    pub provider: String,
    pub api_endpoint: Option<String>,
    pub api_format: String,
    pub model_name: String,
    pub display_name: String,
    pub model_type: String,
    pub multimodal: bool,
    pub system_prompt: Option<String>,
    pub context_window: i64,
    pub temperature: Option<f64>,
    pub status: String,
    // spec ai-chat-enhancement Phase 1 §1.1: 健康检测元数据
    pub latency_ms: Option<i64>,
    pub last_health_check: Option<i64>,
    pub has_api_key: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AiAgent {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    // 安全审计修复（多用户隔离批次 1）：所有者用户 ID（v116 迁移新增）
    pub user_id: i64,
}

pub const VALID_PROVIDERS: &[&str] = &[
    "ollama", "openai", "azure", "anthropic", "deepseek",
    "moonshot", "zhipu", "qwen", "custom",
];

pub fn get_provider_default_url(provider: &str) -> Option<&'static str> {
    match provider {
        "ollama" => Some("http://localhost:11434"),
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

pub fn get_provider_default_model(provider: &str) -> Option<&'static str> {
    match provider {
        "ollama" => Some("llama3.2"),
        "openai" => Some("gpt-4o-mini"),
        "anthropic" => Some("claude-sonnet-4-20250514"),
        "deepseek" => Some("deepseek-chat"),
        "moonshot" => Some("moonshot-v1-8k"),
        "zhipu" => Some("glm-4-flash"),
        "qwen" => Some("qwen-turbo"),
        _ => None,
    }
}

pub fn get_provider_available_models(provider: &str) -> &'static [&'static str] {
    match provider {
        "ollama" => &[
            "llama3.2", "llama3.1", "llama3", "codellama", "mistral",
            "gemma2", "phi3", "qwen2.5", "deepseek-r1", "yi",
        ],
        "openai" => &[
            "gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-4",
            "o1-preview", "o1-mini", "o3-mini", "o4-mini",
        ],
        "anthropic" => &[
            "claude-opus-4-20250514", "claude-sonnet-4-20250514",
            "claude-haiku-4-20250514", "claude-3-5-sonnet-latest",
            "claude-3-5-haiku-latest", "claude-3-opus-20240229",
        ],
        "deepseek" => &[
            "deepseek-chat", "deepseek-reasoner",
            "deepseek-v3", "deepseek-v3-0324",
            "deepseek-v4-flash", "deepseek-v4-pro",
        ],
        "moonshot" => &[
            "moonshot-v1-8k", "moonshot-v1-32k", "moonshot-v1-128k",
        ],
        "zhipu" => &[
            "glm-4-plus", "glm-4-air", "glm-4-flash", "glm-4-long",
            "glm-4v-plus", "glm-4v-air", "glm-4v-flash",
        ],
        "qwen" => &[
            "qwen-max", "qwen-plus", "qwen-turbo", "qwen-long",
            "qwen-vl-max", "qwen-vl-plus", "qwen2.5-72b-instruct",
            "qwen3-235b-a22b", "qwen3-32b", "qwen3-8b",
        ],
        _ => &[],
    }
}

pub fn validate_provider(provider: &str) -> Result<(), String> {
    let lower = provider.to_lowercase();
    if VALID_PROVIDERS.contains(&lower.as_str()) {
        Ok(())
    } else {
        Err(format!(
            "不支持的模型供应商 '{}'，支持的供应商: {}",
            provider,
            VALID_PROVIDERS.join(", ")
        ))
    }
}

pub fn validate_model_url(url: &str) -> Result<(), String> {
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(())
    } else {
        Err(format!("API URL 必须以 http:// 或 https:// 开头: {}", url))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddModelRequest {
    pub name: String,
    pub provider: String,
    pub api_url: Option<String>,
    pub api_key: Option<String>,
    pub api_format: Option<String>,
    pub model_name: Option<String>,
    pub display_name: Option<String>,
    pub is_local: bool,
    pub multimodal: bool,
    pub system_prompt: Option<String>,
    pub context_window: Option<i64>,
    pub temperature: Option<f64>,
}

impl AddModelRequest {
    pub fn validate(&self) -> Result<(), String> {
        validate_provider(&self.provider)?;
        if self.name.trim().is_empty() {
            return Err("模型名称不能为空".into());
        }
        if let Some(ref fmt) = self.api_format {
            if !["openai-chat", "anthropic-messages", "ollama"].contains(&fmt.as_str()) {
                return Err(format!("不支持的 API 格式 '{}'，支持: openai-chat, anthropic-messages, ollama", fmt));
            }
        }
        if !self.is_local {
            if let Some(ref url) = self.api_url {
                validate_model_url(url)?;
            }
            if self.model_name.as_ref().map_or(true, |m| m.trim().is_empty()) {
                return Err("云端API模型必须填写模型 ID（model_name）".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateModelRequest {
    pub id: i64,
    pub name: Option<String>,
    pub provider: Option<String>,
    pub api_url: Option<String>,
    pub api_key: Option<String>,
    pub api_format: Option<String>,
    pub model_name: Option<String>,
    pub display_name: Option<String>,
    pub is_local: Option<bool>,
    pub multimodal: Option<bool>,
    pub system_prompt: Option<String>,
    pub context_window: Option<i64>,
    pub temperature: Option<f64>,
}

impl UpdateModelRequest {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(ref provider) = self.provider {
            validate_provider(provider)?;
        }
        if let Some(ref url) = self.api_url {
            validate_model_url(url)?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddAgentRequest {
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAgentRequest {
    pub id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model_id: Option<i64>,
}

// spec ai-chat-enhancement Phase 2 §2.1: 会话列表带最后消息预览的复合结构
//
// 与 models::chat::Conversation 的差异：
// - 去掉 dissolve_at（列表展示不需要）
// - 增加 unread_count / sort_order（v115 迁移新增字段）
// - 增加 last_message_preview / last_message_sender_type / last_message_at（联表查询最后一条消息）
//
// 字段顺序与 chat_repo::get_conversations_with_preview 的 SELECT 列顺序一致，
// 配合 sqlx::FromRow 按列名匹配（实际按列名而非位置，但保持一致便于审查）。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConversationWithPreview {
    pub id: i64,
    pub user_id: i64,
    pub title: Option<String>,
    pub r#type: String,
    pub is_temp: bool,
    pub token_budget: i64,
    pub starred: Option<bool>,
    pub created_at: i64,
    pub updated_at: i64,
    pub unread_count: i64,
    pub sort_order: i64,
    // 最后一条消息预览（联表查询，NULL 表示会话无消息）
    pub last_message_preview: Option<String>,
    pub last_message_sender_type: Option<String>,
    pub last_message_at: Option<i64>,
}

// spec ai-chat-enhancement Phase 2 §3.3: 会话拖拽排序请求项
// 由前端 reorder_conversations 命令传入 Vec<ReorderItem>，
// 经服务层转发到 chat_repo::reorder_conversations 在事务内批量 UPDATE。
#[derive(Debug, Serialize, Deserialize)]
pub struct ReorderItem {
    pub id: i64,
    pub sort_order: i64,
}

// spec ai-chat-enhancement Phase 1 §1.3: 单个模型最新健康状态
//
// 由 get_model_health_status 命令返回，与 ModelHealthStatus（chat 模块）的差异：
// - ModelHealthStatus 用于"刚检测完"的结果（含 is_online bool + name/provider）
// - ModelHealthInfo 用于"从 DB 查询"的最新状态（含 status 字符串 + last_health_check）
//
// 字段对齐 `ai-model-health-changed` 事件 payload，便于前端复用同一渲染逻辑。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHealthInfo {
    pub model_id: i64,
    pub name: String,
    pub status: String,
    pub latency_ms: Option<i64>,
    pub last_health_check: Option<i64>,
}