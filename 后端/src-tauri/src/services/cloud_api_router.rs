//! Yuan Code v3.1 Task 3.5.3 — 云端 API 路由层
//!
//! 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.5.3
//!       + 项目核心设计意图 §三 / §八（强制规则 8.1.1）
//!
//! 核心约束（项目核心设计意图 §三 / §八）：
//! - Yuan Code 编程 AI 必须走云端 API 模型（OpenAI/Claude/GPT-4o/DeepSeek/...）
//! - 禁止本地底层智能模型（ollama qwen3:8b 等）用于编程生成
//! - 本路由层是"编程任务"的唯一出口，所有 Agent 编程调用必须经此路由
//!
//! 设计：
//! 1. `route_programming_request()` 入口：根据 provider + model 从 api_keys 表加载密钥
//! 2. `validate_cloud_only()` 守卫：拒绝本地 provider（ollama 等）的编程请求
//! 3. 复用 `AiModelService` 完成实际 HTTP 调用（不重复造轮子）
//! 4. 调用成功后 `touch_last_used` 刷新 last_used_at
//!
//! 与底层智能的边界：
//! - 底层智能（D2）是「智能优化层」，不替代编程 AI 调用
//! - 本路由层只服务 Yuan Code 编程任务，不路由到底层智能本地模型

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::SqlitePool;
use tauri::Emitter;
use tokio::sync::RwLock;

use crate::crypto::aes_gcm;
use crate::crypto::mek_manager::MekManager;
use crate::db::repositories::api_key_repo;
use crate::error::app_error::AppError;
use crate::models::api_key::{CLOUD_API_PROVIDERS, get_cloud_provider_default_url};
use crate::services::ai_model_service::AiModelService;

/// 编程任务路由请求
#[derive(Debug, Clone)]
pub struct ProgrammingRequest {
    /// 云端 API provider（openai/anthropic/deepseek/...）
    pub provider: String,
    /// 具体模型名（gpt-4o / claude-sonnet-4-20250514 / ...）
    pub model_name: String,
    /// 用户 prompt
    pub prompt: String,
    /// 系统提示词
    pub system_prompt: Option<String>,
    /// 温度（None = 用 provider 默认）
    pub temperature: Option<f64>,
    /// 最大 token（None = 用 provider 默认）
    pub max_tokens: Option<i32>,
    /// 是否流式（流式通过 app_handle emit "agent-cloud-stream" 事件）
    pub stream: bool,
    /// 会话标识（用于流式事件路由）
    pub conversation_id: Option<i64>,
    /// 触发 Agent 的 agent_id（用于审计/监测）
    pub agent_id: Option<String>,
}

impl ProgrammingRequest {
    /// 校验：provider 必须在云端白名单内（强制约束）
    pub fn validate_cloud_only(&self) -> Result<(), AppError> {
        let provider_lower = self.provider.to_lowercase();
        if !CLOUD_API_PROVIDERS.contains(&provider_lower.as_str()) {
            return Err(AppError::Validation(format!(
                "Yuan Code 编程 AI 必须走云端 API provider（{}），\
                 禁止使用本地底层智能模型（如 ollama）\
                 [项目核心设计意图 §三/§八 强制规则]",
                CLOUD_API_PROVIDERS.join(", ")
            )));
        }
        if self.model_name.trim().is_empty() {
            return Err(AppError::Validation(
                "云端 API 模型名（model_name）不能为空".into(),
            ));
        }
        if self.prompt.trim().is_empty() {
            return Err(AppError::Validation("编程 prompt 不能为空".into()));
        }
        Ok(())
    }
}

/// 路由结果
#[derive(Debug, Clone)]
pub struct ProgrammingResponse {
    pub provider: String,
    pub model_used: String,
    pub content: String,
    pub tokens_used: Option<i64>,
    pub agent_id: Option<String>,
}

/// 云端 API 路由器 — 编程任务唯一出口
pub struct CloudApiRouter {
    /// 复用 AiModelService 完成 HTTP 调用
    /// 注：当前实现直接走 call_cloud_api 私有方法，此字段保留以便未来复用 AiModelService 的高级能力（如重试、指标）
    #[allow(dead_code)]
    ai_service: Arc<AiModelService>,
}

impl CloudApiRouter {
    pub fn new(ai_service: Arc<AiModelService>) -> Self {
        Self { ai_service }
    }

    /// 路由编程请求到云端 API（非流式）
    ///
    /// 强制规则：本方法是 Yuan Code 编程任务（含 7 种 Agent 调用）的唯一出口。
    /// 任何本地底层智能模型（ollama 等）的编程请求都会被 `validate_cloud_only()` 拒绝。
    pub async fn route_programming_request(
        &self,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        req: ProgrammingRequest,
    ) -> Result<ProgrammingResponse, AppError> {
        // 1. 强制约束校验：拒绝本地底层智能模型
        req.validate_cloud_only()?;

        // 2. 从 api_keys 表加载密钥（按 provider + user_id，多用户隔离）
        let api_key_row = api_key_repo::get_by_provider(pool, user_id, &req.provider)
            .await?
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "云端 API provider '{}' 未配置 API Key，\
                     请先在 ModelSelector UI 中配置（api_keys 表无记录）",
                    req.provider
                ))
            })?;

        if !api_key_row.is_enabled {
            return Err(AppError::Validation(format!(
                "provider '{}' 的 API Key 已禁用，请在设置中启用",
                req.provider
            )));
        }

        // 3. 解密 API Key（复用 crypto::aes_gcm + MEK，按 user_id 取 MEK）
        let api_key_str = decrypt_api_key_for_provider(
            &api_key_row.api_key_enc,
            &api_key_row.api_key_nonce,
            mek_manager,
            user_id,
        )
        .await?;

        // 4. 构造临时 AiModel 行，复用 AiModelService 的 HTTP 调用逻辑
        //    （复用而非重新实现：遵循"精准修改"原则，不重复造轮子）
        let api_url = api_key_row
            .api_url
            .clone()
            .or_else(|| get_cloud_provider_default_url(&req.provider).map(String::from))
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "provider '{}' 未配置 api_url 且无默认值",
                    req.provider
                ))
            })?;

        // 直接调用底层 HTTP（绕过 ai_repo，因为密钥来自 api_keys 表）
        let temperature = req.temperature.unwrap_or(0.2);
        let content = self
            .call_cloud_api(
                &req.provider,
                &api_url,
                &api_key_str,
                &req.model_name,
                &req.prompt,
                req.system_prompt.as_deref(),
                temperature,
                req.max_tokens,
            )
            .await?;

        // 5. 刷新 last_used_at（审计）
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let _ = api_key_repo::touch_last_used(pool, user_id, &req.provider, now).await;

        tracing::info!(
            "[cloud_api_router] 编程任务路由成功: provider={}, model={}, agent={:?}, prompt_len={}",
            req.provider,
            req.model_name,
            req.agent_id,
            req.prompt.len()
        );

        Ok(ProgrammingResponse {
            provider: req.provider.clone(),
            model_used: req.model_name.clone(),
            content,
            tokens_used: None,
            agent_id: req.agent_id.clone(),
        })
    }

    /// 路由编程请求到云端 API（流式，通过 app_handle emit 事件）
    pub async fn route_programming_request_stream(
        &self,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        req: ProgrammingRequest,
        app_handle: &tauri::AppHandle,
    ) -> Result<ProgrammingResponse, AppError> {
        req.validate_cloud_only()?;

        let api_key_row = api_key_repo::get_by_provider(pool, user_id, &req.provider)
            .await?
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "云端 API provider '{}' 未配置 API Key",
                    req.provider
                ))
            })?;

        if !api_key_row.is_enabled {
            return Err(AppError::Validation(format!(
                "provider '{}' 的 API Key 已禁用",
                req.provider
            )));
        }

        let api_key_str = decrypt_api_key_for_provider(
            &api_key_row.api_key_enc,
            &api_key_row.api_key_nonce,
            mek_manager,
            user_id,
        )
        .await?;

        let api_url = api_key_row
            .api_url
            .clone()
            .or_else(|| get_cloud_provider_default_url(&req.provider).map(String::from))
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "provider '{}' 未配置 api_url",
                    req.provider
                ))
            })?;

        let temperature = req.temperature.unwrap_or(0.2);
        // 流式调用：emit "agent-cloud-stream" 事件
        let content = self
            .call_cloud_api_stream(
                &req.provider,
                &api_url,
                &api_key_str,
                &req.model_name,
                &req.prompt,
                req.system_prompt.as_deref(),
                temperature,
                req.max_tokens,
                app_handle,
                req.conversation_id,
                req.agent_id.as_deref(),
            )
            .await?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let _ = api_key_repo::touch_last_used(pool, user_id, &req.provider, now).await;

        Ok(ProgrammingResponse {
            provider: req.provider.clone(),
            model_used: req.model_name.clone(),
            content,
            tokens_used: None,
            agent_id: req.agent_id.clone(),
        })
    }

    /// 内部 HTTP 调用 — OpenAI 兼容 / Anthropic Messages API
    ///
    /// 复用 reqwest::Client；不依赖 ai_repo/AiModelService 内部状态，
    /// 因为密钥来自 api_keys 表（独立于 ai_models）。
    async fn call_cloud_api(
        &self,
        provider: &str,
        api_url: &str,
        api_key: &str,
        model: &str,
        prompt: &str,
        system_prompt: Option<&str>,
        temperature: f64,
        max_tokens: Option<i32>,
    ) -> Result<String, AppError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| AppError::AiApi(format!("HTTP client 构建失败: {}", e)))?;

        match provider.to_lowercase().as_str() {
            "anthropic" => {
                self.call_anthropic(
                    &client,
                    api_url,
                    api_key,
                    model,
                    prompt,
                    system_prompt,
                    temperature,
                    max_tokens,
                )
                .await
            }
            // openai / azure / deepseek / moonshot / zhipu / qwen / custom 全部走 OpenAI 兼容协议
            _ => {
                self.call_openai_compatible(
                    &client,
                    api_url,
                    api_key,
                    model,
                    prompt,
                    system_prompt,
                    temperature,
                    max_tokens,
                )
                .await
            }
        }
    }

    async fn call_openai_compatible(
        &self,
        client: &reqwest::Client,
        api_url: &str,
        api_key: &str,
        model: &str,
        prompt: &str,
        system_prompt: Option<&str>,
        temperature: f64,
        max_tokens: Option<i32>,
    ) -> Result<String, AppError> {
        let mut messages = Vec::with_capacity(2);
        if let Some(sys) = system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys,
            }));
        }
        messages.push(serde_json::json!({
            "role": "user",
            "content": prompt,
        }));

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": temperature,
            "stream": false,
        });
        if let Some(max) = max_tokens {
            body["max_tokens"] = serde_json::json!(max);
        }

        let url = format!("{}/chat/completions", api_url.trim_end_matches('/'));
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::AiApi(format!("OpenAI 兼容请求失败: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::AiApi(format!(
                "OpenAI 兼容 API 错误 [{}]: {}",
                status, text
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::AiApi(format!("响应 JSON 解析失败: {}", e)))?;

        let content = json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::AiApi("响应缺少 choices[0].message.content".into()))?
            .to_string();

        Ok(content)
    }

    async fn call_anthropic(
        &self,
        client: &reqwest::Client,
        api_url: &str,
        api_key: &str,
        model: &str,
        prompt: &str,
        system_prompt: Option<&str>,
        temperature: f64,
        max_tokens: Option<i32>,
    ) -> Result<String, AppError> {
        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": max_tokens.unwrap_or(4096),
            "temperature": temperature,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        });
        if let Some(sys) = system_prompt {
            body["system"] = serde_json::json!(sys);
        }

        let url = format!("{}/messages", api_url.trim_end_matches('/'));
        let resp = client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::AiApi(format!("Anthropic 请求失败: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::AiApi(format!(
                "Anthropic API 错误 [{}]: {}",
                status, text
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::AiApi(format!("Anthropic 响应 JSON 解析失败: {}", e)))?;

        let content = json
            .get("content")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("text"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::AiApi("Anthropic 响应缺少 content[0].text".into()))?
            .to_string();

        Ok(content)
    }

    /// 流式调用：emit `agent-cloud-stream` 事件
    ///
    /// 与 ai_model_service::call_model_stream 对齐（OpenAI 兼容流式）
    async fn call_cloud_api_stream(
        &self,
        provider: &str,
        api_url: &str,
        api_key: &str,
        model: &str,
        prompt: &str,
        system_prompt: Option<&str>,
        temperature: f64,
        max_tokens: Option<i32>,
        app_handle: &tauri::AppHandle,
        conversation_id: Option<i64>,
        agent_id: Option<&str>,
    ) -> Result<String, AppError> {
        // 简化实现：调用非流式 + 一次性 emit（保留流式接口，便于未来切换为 SSE）
        let content = self
            .call_cloud_api(
                provider,
                api_url,
                api_key,
                model,
                prompt,
                system_prompt,
                temperature,
                max_tokens,
            )
            .await?;

        let mut payload = serde_json::json!({
            "conversation_id": conversation_id.unwrap_or(-1),
            "chunk": content,
            "event": "chunk",
            "provider": provider,
            "model": model,
        });
        if let Some(aid) = agent_id {
            payload["agent_id"] = serde_json::json!(aid);
        }
        let _ = app_handle.emit("agent-cloud-stream", payload);

        Ok(content)
    }
}

impl ProgrammingRequest {
    /// 构造简单编程请求（便捷构造器）
    pub fn new_simple(
        provider: impl Into<String>,
        model_name: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            provider: provider.into(),
            model_name: model_name.into(),
            prompt: prompt.into(),
            system_prompt: None,
            temperature: Some(0.2),
            max_tokens: None,
            stream: false,
            conversation_id: None,
            agent_id: None,
        }
    }
}

/// 解密 api_keys 表中的密钥（复用 crypto::aes_gcm + MEK）
///
/// 与 ai_model_service::decrypt_api_key 的区别：本函数从 api_keys 表的 BLOB 字段读取，
/// 而 ai_model_service 版本从 ai_models 表的 AiModel.api_key_enc 读取。
async fn decrypt_api_key_for_provider(
    enc: &[u8],
    nonce: &[u8],
    mek_manager: &Arc<RwLock<MekManager>>,
    user_id: i64,
) -> Result<String, AppError> {
    let mgr = mek_manager.read().await;
    let mek = mgr
        .get_mek(user_id)
        .ok_or_else(|| AppError::MekDecryption("MEK 未在内存中".into()))?;
    let nonce_arr: [u8; 12] = nonce
        .try_into()
        .map_err(|_| AppError::Crypto("api_key_nonce 长度错误".into()))?;
    let key_bytes = aes_gcm::decrypt_bytes(enc, mek, &nonce_arr)?;
    String::from_utf8(key_bytes).map_err(|e| AppError::Crypto(format!("API Key UTF-8 解码失败: {}", e)))
}

#[cfg(test)]
mod tests {
    //! CloudApiRouter 单元测试（v1.52 测试体系 Phase 2）
    //! 参见：03_测试体系_单元与集成测试.md §2.2.1

    use super::*;

    #[test]
    fn test_validate_cloud_only_rejects_ollama() {
        let req = ProgrammingRequest::new_simple("ollama", "qwen3:8b", "implement a function");
        let err = req.validate_cloud_only().unwrap_err();
        let msg = match err {
            AppError::Validation(m) => m,
            _ => "unexpected error type".to_string(),
        };
        assert!(msg.contains("云端 API"), "应拒绝 ollama: {}", msg);
        assert!(msg.contains("禁止"), "应提示禁止本地模型: {}", msg);
    }

    #[test]
    fn test_validate_cloud_only_accepts_openai() {
        let req = ProgrammingRequest::new_simple("openai", "gpt-4o", "implement a function");
        req.validate_cloud_only().expect("openai 应通过校验");
    }

    #[test]
    fn test_validate_cloud_only_accepts_anthropic() {
        let req =
            ProgrammingRequest::new_simple("anthropic", "claude-sonnet-4-20250514", "refactor");
        req.validate_cloud_only().expect("anthropic 应通过校验");
    }

    #[test]
    fn test_validate_cloud_only_accepts_deepseek() {
        let req = ProgrammingRequest::new_simple("deepseek", "deepseek-chat", "test");
        req.validate_cloud_only().expect("deepseek 应通过校验");
    }

    #[test]
    fn test_validate_cloud_only_rejects_empty_model() {
        let req = ProgrammingRequest::new_simple("openai", "", "implement");
        let err = req.validate_cloud_only().unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn test_validate_cloud_only_rejects_empty_prompt() {
        let req = ProgrammingRequest::new_simple("openai", "gpt-4o", "   ");
        assert!(req.validate_cloud_only().is_err());
    }

    #[test]
    fn test_validate_cloud_only_accepts_case_insensitive_provider() {
        let req = ProgrammingRequest::new_simple("OpenAI", "gpt-4o", "implement");
        req.validate_cloud_only().expect("OpenAI 大小写应通过校验");
    }
}
