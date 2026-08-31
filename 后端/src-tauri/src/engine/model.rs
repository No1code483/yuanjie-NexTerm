// 模型客户端抽象 — 对标 Codex-rs 的 ModelClient
// 提供多提供商统一的 AI 模型调用接口

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::error::app_error::AppError;

/// 模型提供商信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProviderInfo {
    /// 提供商名称
    pub name: String,
    /// 提供商显示名称
    pub display_name: String,
    /// API 基础 URL
    pub base_url: String,
    /// API 密钥 (环境变量名)
    pub api_key_env: String,
    /// 支持的模型列表
    pub models: Vec<ModelInfo>,
    /// 是否支持流式
    pub supports_streaming: bool,
    /// 是否支持函数调用
    pub supports_tools: bool,
}

/// 模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// 模型 ID
    pub id: String,
    /// 显示名称
    pub display_name: String,
    /// 最大上下文长度
    pub max_context_tokens: u64,
    /// 最大输出长度
    pub max_output_tokens: u64,
    /// 是否默认
    pub is_default: bool,
}

/// 模型请求
#[derive(Debug, Clone)]
pub struct ModelRequest {
    /// 模型 ID
    pub model: String,
    /// 消息列表
    pub messages: Vec<ModelMessage>,
    /// 温度
    pub temperature: f32,
    /// 最大输出 Token
    pub max_tokens: u64,
    /// Top P
    pub top_p: f32,
    /// 是否流式
    pub stream: bool,
    /// 工具定义
    pub tools: Option<Vec<ToolDefinition>>,
    /// 系统提示
    pub system_prompt: Option<String>,
    /// 超时
    pub timeout: Duration,
}

/// 模型消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

/// 函数调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

/// 函数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// 模型响应
#[derive(Debug, Clone)]
pub struct ModelResponse {
    /// 完整响应文本
    pub content: String,
    /// 工具调用
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Token 用量
    pub usage: Option<ModelUsage>,
    /// 完成原因
    pub finish_reason: Option<String>,
    /// 模型 ID
    pub model: String,
}

/// 流式响应块
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// 文本增量
    pub delta: String,
    /// 工具调用增量
    pub tool_call_delta: Option<ToolCallDelta>,
    /// 完成原因
    pub finish_reason: Option<String>,
    /// 序列号
    pub sequence: u64,
}

/// 工具调用增量
#[derive(Debug, Clone)]
pub struct ToolCallDelta {
    pub index: usize,
    pub id: Option<String>,
    pub function_name: Option<String>,
    pub function_arguments: Option<String>,
}

/// 模型用量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

/// 模型管理器
pub struct ModelManager {
    /// 提供商注册表
    providers: HashMap<String, ModelProviderInfo>,
    /// 默认提供商
    default_provider: String,
    /// 默认模型
    default_model: String,
}

impl ModelManager {
    pub fn new() -> Self {
        let mut providers = HashMap::new();

        // 注册 OpenAI
        providers.insert(
            "openai".into(),
            ModelProviderInfo {
                name: "openai".into(),
                display_name: "OpenAI".into(),
                base_url: "https://api.openai.com/v1".into(),
                api_key_env: "OPENAI_API_KEY".into(),
                models: vec![
                    ModelInfo {
                        id: "gpt-4o".into(),
                        display_name: "GPT-4o".into(),
                        max_context_tokens: 128_000,
                        max_output_tokens: 16_384,
                        is_default: true,
                    },
                    ModelInfo {
                        id: "gpt-4o-mini".into(),
                        display_name: "GPT-4o Mini".into(),
                        max_context_tokens: 128_000,
                        max_output_tokens: 16_384,
                        is_default: false,
                    },
                    ModelInfo {
                        id: "gpt-4.1".into(),
                        display_name: "GPT-4.1".into(),
                        max_context_tokens: 1_000_000,
                        max_output_tokens: 32_768,
                        is_default: false,
                    },
                ],
                supports_streaming: true,
                supports_tools: true,
            },
        );

        // 注册 Anthropic
        providers.insert(
            "anthropic".into(),
            ModelProviderInfo {
                name: "anthropic".into(),
                display_name: "Anthropic".into(),
                base_url: "https://api.anthropic.com/v1".into(),
                api_key_env: "ANTHROPIC_API_KEY".into(),
                models: vec![
                    ModelInfo {
                        id: "claude-sonnet-4-20250514".into(),
                        display_name: "Claude Sonnet 4".into(),
                        max_context_tokens: 200_000,
                        max_output_tokens: 8_192,
                        is_default: true,
                    },
                    ModelInfo {
                        id: "claude-opus-4-20250514".into(),
                        display_name: "Claude Opus 4".into(),
                        max_context_tokens: 200_000,
                        max_output_tokens: 8_192,
                        is_default: false,
                    },
                ],
                supports_streaming: true,
                supports_tools: true,
            },
        );

        Self {
            providers,
            default_provider: "openai".into(),
            default_model: "gpt-4o".into(),
        }
    }

    /// 获取所有提供商
    pub fn providers(&self) -> Vec<&ModelProviderInfo> {
        self.providers.values().collect()
    }

    /// 获取指定提供商
    pub fn get_provider(&self, name: &str) -> Option<&ModelProviderInfo> {
        self.providers.get(name)
    }

    /// 获取默认提供商
    pub fn default_provider(&self) -> &str {
        &self.default_provider
    }

    /// 获取默认模型
    pub fn default_model(&self) -> &str {
        &self.default_model
    }

    /// 设置默认提供商
    pub fn set_default_provider(&mut self, provider: &str) -> Result<(), AppError> {
        if !self.providers.contains_key(provider) {
            return Err(AppError::Internal(format!("未知提供商: {}", provider)));
        }
        self.default_provider = provider.into();
        Ok(())
    }

    /// 设置默认模型
    pub fn set_default_model(&mut self, model: &str) -> Result<(), AppError> {
        let provider = self.providers.get(&self.default_provider).ok_or_else(|| {
            AppError::Internal("默认提供商未设置".into())
        })?;

        if !provider.models.iter().any(|m| m.id == model) {
            return Err(AppError::Internal(format!("未知模型: {}", model)));
        }
        self.default_model = model.into();
        Ok(())
    }

    /// 注册自定义提供商
    pub fn register_provider(&mut self, provider: ModelProviderInfo) {
        self.providers.insert(provider.name.clone(), provider);
    }
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}