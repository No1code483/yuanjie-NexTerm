use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::Emitter;

use crate::crypto::aes_gcm;
use crate::crypto::mek_manager::MekManager;
use crate::db::connection::AppState;
use crate::db::repositories::ai_repo;
use crate::error::app_error::AppError;
use crate::models::ai_model::{AiModel, get_provider_default_model, get_provider_default_url};
use std::sync::Arc;
use tokio::sync::RwLock;
use sqlx::SqlitePool;

#[derive(Debug, Serialize, Deserialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: Option<OllamaOptions>,
}

// ============================================================================
// ImageSize 枚举（spec 阶段1 Task 1.2）
// ============================================================================

/// 文生图尺寸枚举（spec 阶段1 Task 1.2）。
///
/// 取值参考 trae-api-cn 图片生成 API 与 OpenAI DALL-E 3 标准 size 的映射：
/// - `SquareHd` → "1024x1024"（高质量方形）
/// - `Square` → "1024x1024"（标准方形，等同 SquareHd，与 trae-api-cn 命名对齐）
/// - `Portrait4_3` → "896x1152"（纵向 4:3）
/// - `Landscape4_3` → "1152x896"（横向 4:3）
/// - `Portrait16_9` → "768x1408"（宽屏纵向 16:9）
/// - `Landscape16_9` → "1408x768"（宽屏横向 16:9）
///
/// 设计依据：.trae/rules/项目核心设计意图.md §五（游戏考验生成走云端 API 模型）。
/// provider 路由：仅 openai/anthropic/custom 路由到 /images/generations；
/// ollama 等本地 provider 不支持文生图，调用方需降级到 placeholder://。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageSize {
    SquareHd,
    Square,
    Portrait4_3,
    Landscape4_3,
    Portrait16_9,
    Landscape16_9,
}

impl ImageSize {
    /// 转 OpenAI `/v1/images/generations` API 的 `size` 字段字符串。
    pub fn as_openai_size(&self) -> &'static str {
        match self {
            Self::SquareHd | Self::Square => "1024x1024",
            Self::Portrait4_3 => "896x1152",
            Self::Landscape4_3 => "1152x896",
            Self::Portrait16_9 => "768x1408",
            Self::Landscape16_9 => "1408x768",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaOptions {
    temperature: f64,
    top_p: f64,
    num_predict: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaResponse {
    response: String,
    done: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f64,
    max_tokens: Option<i32>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessageContent,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessageContent {
    content: String,
}

pub struct AiModelService {
    client: Client,
}

impl AiModelService {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .no_gzip()
                .no_deflate()
                .no_brotli()
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    /// A5 Phase 3 Task 2: 云端 AI 调用前的在线守卫
    ///
    /// 设计依据：
    /// - .trae/rules/项目核心设计意图.md §二/§三/§四/§五/§六（云端 AI 服务）
    /// - .trae/rules/项目核心设计意图.md §八 8.1.1（云端 AI 调用必须走云端 API，离线不切换本地 ollama）
    /// - .trae/rules/项目核心设计意图.md §八 8.2.3（测试用例必须包含"底层智能关闭"场景）
    ///
    /// 调用方：小欣 / Yuan Code / 游戏 等云端 AI 入口在调用 `call_model*` /
    /// `route_programming_request*` 前调用此方法。
    ///
    /// 边界：
    /// - 仅云端 AI 服务调用此守卫；底层智能（D2）继续使用本地 ollama，不调用此方法
    /// - 离线时返回 `AppError::Offline`（前端据此显示「AI 服务不可用，请连接网络」）
    /// - 在线时返回 `Ok(())`，调用方继续走原云端 API 流程
    pub async fn check_online_before_call(state: &AppState) -> Result<(), AppError> {
        let status = state.connectivity_checker.get_status().await;
        if !status.is_online {
            tracing::info!(
                "[ai_offline_guard] 云端 AI 调用被拦截：网络离线 (last_checked={})",
                status.last_checked
            );
            return Err(AppError::Offline(
                "AI 服务不可用，请连接网络后重试".into(),
            ));
        }
        Ok(())
    }

    pub async fn call_model(
        &self,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        model_id: i64,
        prompt: &str,
        system_prompt: Option<&str>,
    ) -> Result<String, AppError> {
        let model = ai_repo::get_model_by_id(pool, model_id, user_id)
            .await?
            .ok_or_else(|| AppError::AiApi("模型不存在".into()))?;

        let api_key = Self::decrypt_api_key(&model, mek_manager, user_id).await?;
        let temperature = model.temperature.unwrap_or(0.7);

        match model.provider.to_lowercase().as_str() {
            "ollama" => self.call_ollama(&model, prompt, system_prompt).await,
            "anthropic" => self.call_anthropic(&model, &api_key, prompt, system_prompt, temperature).await,
            _ => self.call_openai_compatible(&model, &api_key, prompt, system_prompt).await,
        }
    }

    pub async fn call_model_stream(
        &self,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        model_id: i64,
        prompt: &str,
        system_prompt: Option<&str>,
        app_handle: &tauri::AppHandle,
        conversation_id: i64,
        sender_id: Option<i64>,
    ) -> Result<String, AppError> {
        let model = ai_repo::get_model_by_id(pool, model_id, user_id)
            .await?
            .ok_or_else(|| AppError::AiApi("模型不存在".into()))?;

        let api_key = Self::decrypt_api_key(&model, mek_manager, user_id).await?;
        let temperature = model.temperature.unwrap_or(0.7);

        match model.provider.to_lowercase().as_str() {
            "ollama" => {
                self.call_ollama_stream(&model, prompt, temperature, app_handle, conversation_id, sender_id)
                    .await
            }
            "anthropic" => {
                let result = self.call_anthropic(&model, &api_key, prompt, system_prompt, temperature).await?;
                let mut payload = serde_json::json!({
                    "conversation_id": conversation_id,
                    "chunk": result,
                    "event": "chunk",
                });
                if let Some(sid) = sender_id {
                    payload["sender_id"] = serde_json::json!(sid);
                }
                let _ = app_handle.emit("ai-stream", payload);
                Ok(result)
            }
            _ => {
                self.call_openai_stream(
                    &model, &api_key, prompt, system_prompt, temperature, app_handle, conversation_id, sender_id,
                )
                .await
            }
        }
    }

    async fn decrypt_api_key(
        model: &AiModel,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
    ) -> Result<String, AppError> {
        if let (Some(enc), Some(nonce)) = (&model.api_key_enc, &model.api_key_nonce) {
            let mgr = mek_manager.read().await;
            let mek = mgr
                .get_mek(user_id)
                .ok_or_else(|| AppError::MekDecryption("MEK 未在内存中".into()))?;
            let nonce_arr: [u8; 12] = nonce.as_slice()
                .try_into()
                .map_err(|_| AppError::Crypto("API Key nonce 长度错误".into()))?;
            let key_bytes = aes_gcm::decrypt_bytes(enc, mek, &nonce_arr)?;
            String::from_utf8(key_bytes)
                .map_err(|e| AppError::Crypto(format!("API Key UTF-8 解码失败: {}", e)))
        } else {
            Ok(String::new())
        }
    }

    async fn call_ollama_stream(
        &self,
        model: &AiModel,
        prompt: &str,
        temperature: f64,
        app_handle: &tauri::AppHandle,
        conversation_id: i64,
        sender_id: Option<i64>,
    ) -> Result<String, AppError> {
        let api_url = model.api_url.as_deref().unwrap_or("http://localhost:11434");
        let model_name = model.model_name.clone().unwrap_or_else(|| "llama3.2".to_string());

        // 使用非流式模式，一次性获取完整响应，彻底避免流式传输的解码问题
        let request = OllamaRequest {
            model: model_name,
            prompt: prompt.to_string(),
            stream: false,
            options: Some(OllamaOptions {
                temperature,
                top_p: 0.9,
                num_predict: Some(2048),
            }),
        };

        let response = self
            .client
            .post(&format!("{}/api/generate", api_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::AiApi(format!("Ollama 请求失败: {}", e)))?;

        let parsed: OllamaResponse = response
            .json()
            .await
            .map_err(|e| AppError::AiApi(format!("Ollama 响应解析失败: {}", e)))?;

        let full_response = parsed.response;

        if !full_response.is_empty() {
            let mut payload = serde_json::json!({
                "conversation_id": conversation_id,
                "chunk": full_response,
                "event": "chunk",
            });
            if let Some(sid) = sender_id {
                payload["sender_id"] = serde_json::json!(sid);
            }
            let _ = app_handle.emit("ai-stream", payload);
        }

        Ok(full_response)
    }

    async fn call_openai_stream(
        &self,
        model: &AiModel,
        api_key: &str,
        prompt: &str,
        system_prompt: Option<&str>,
        temperature: f64,
        app_handle: &tauri::AppHandle,
        conversation_id: i64,
        sender_id: Option<i64>,
    ) -> Result<String, AppError> {
        let api_url = model.api_url.as_deref()
            .or_else(|| get_provider_default_url(&model.provider))
            .unwrap_or("https://api.openai.com/v1");
        let model_name = model.model_name.as_deref()
            .or_else(|| get_provider_default_model(&model.provider))
            .unwrap_or("gpt-4o-mini");

        let mut messages = vec![];
        if let Some(sys_prompt) = system_prompt {
            messages.push(OpenAIMessage {
                role: "system".to_string(),
                content: sys_prompt.to_string(),
            });
        }
        messages.push(OpenAIMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        });

        // 使用非流式模式，一次性获取完整响应
        let request = OpenAIRequest {
            model: model_name.to_string(),
            messages,
            temperature,
            max_tokens: Some(2048),
            stream: false,
        };

        let mut request_builder = self
            .client
            .post(&format!("{}/chat/completions", api_url))
            .json(&request);

        if !api_key.is_empty() {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request_builder.send().await.map_err(|e| {
            AppError::AiApi(format!("OpenAI 请求失败: {}", e))
        })?;

        let parsed: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| AppError::AiApi(format!("OpenAI 响应解析失败: {}", e)))?;

        let full_response = parsed.choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        if !full_response.is_empty() {
            let mut payload = serde_json::json!({
                "conversation_id": conversation_id,
                "chunk": full_response,
                "event": "chunk",
            });
            if let Some(sid) = sender_id {
                payload["sender_id"] = serde_json::json!(sid);
            }
            let _ = app_handle.emit("ai-stream", payload);
        }

        Ok(full_response)
    }

    /// D3.1 小欣接入统一模型管理：获取模型配置 + 解密 api_key（供流式调用复用）
    ///
    /// 返回 (model, api_key)，调用方自行构造流式请求
    pub async fn get_model_config(
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        model_id: i64,
    ) -> Result<(AiModel, String), AppError> {
        let model = ai_repo::get_model_by_id(pool, model_id, user_id)
            .await?
            .ok_or_else(|| AppError::AiApi("模型不存在".into()))?;
        let api_key = Self::decrypt_api_key(&model, mek_manager, user_id).await?;
        Ok((model, api_key))
    }

    /// D3.1 小欣接入统一模型管理：多轮对话非流式调用
    ///
    /// 接受 messages 列表（role, content），支持 system/user/assistant 多轮对话
    pub async fn call_model_messages(
        &self,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        model_id: i64,
        messages: Vec<(String, String)>,
        system_prompt: Option<&str>,
    ) -> Result<String, AppError> {
        let model = ai_repo::get_model_by_id(pool, model_id, user_id)
            .await?
            .ok_or_else(|| AppError::AiApi("模型不存在".into()))?;

        let api_key = Self::decrypt_api_key(&model, mek_manager, user_id).await?;
        let temperature = model.temperature.unwrap_or(0.7);

        match model.provider.to_lowercase().as_str() {
            "ollama" => {
                // Ollama: 拼接为单个 prompt
                let mut full_prompt = String::new();
                if let Some(sys) = system_prompt {
                    full_prompt.push_str(&format!("System: {}\n\n", sys));
                }
                for (role, content) in &messages {
                    full_prompt.push_str(&format!("{}: {}\n", role, content));
                }
                self.call_ollama(&model, &full_prompt, None).await
            }
            "anthropic" => {
                // Anthropic: 把 messages 拼接为 prompt（system 分离）
                let mut user_messages = Vec::new();
                for (role, content) in &messages {
                    if role != "system" {
                        user_messages.push(content.clone());
                    }
                }
                let prompt = user_messages.join("\n\n");
                self.call_anthropic(&model, &api_key, &prompt, system_prompt, temperature).await
            }
            _ => {
                // OpenAI 兼容：构造完整 messages 列表
                self.call_openai_messages(&model, &api_key, messages, system_prompt, temperature).await
            }
        }
    }

    /// D3.4 图像理解：多模态模型调用
    ///
    /// 接受 OpenAI 兼容的 messages 数组（serde_json::Value），content 可为字符串或
    /// content blocks 数组（text + image_url），用于 vision 模型的图片理解场景。
    ///
    /// 设计依据：
    /// - .trae/rules/项目核心设计意图.md §四（小欣走云端 API，不是底层智能模型）
    /// - xin_multimodal_service.rs 已构建 OpenAI Vision content blocks，此方法负责发送
    ///
    /// provider 路由：
    ///   - ollama     降级为纯文本拼接（本地模型不支持 vision content 数组）
    ///   - anthropic  转换为 Anthropic Messages API 格式（image_url → image source）
    ///   - 其他       走 OpenAI 兼容 /chat/completions（原生支持 content 数组）
    pub async fn call_model_multimodal(
        &self,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        model_id: i64,
        messages: Vec<serde_json::Value>,
        temperature: Option<f64>,
        max_tokens: Option<i32>,
    ) -> Result<String, AppError> {
        let model = ai_repo::get_model_by_id(pool, model_id, user_id)
            .await?
            .ok_or_else(|| AppError::AiApi("模型不存在".into()))?;
        let api_key = Self::decrypt_api_key(&model, mek_manager, user_id).await?;
        let temp = temperature.unwrap_or_else(|| model.temperature.unwrap_or(0.7));
        let tokens = max_tokens.unwrap_or(2048);

        match model.provider.to_lowercase().as_str() {
            "anthropic" => {
                self.call_anthropic_multimodal(&model, &api_key, messages, temp, tokens)
                    .await
            }
            "ollama" => {
                // Bug 修复 v1.52.19.1：原逻辑把 image_url 块拼接成文本（仅提取 URL 字符串），
                // 静默丢失图片导致模型生成质量极差的"伪图片理解"。
                // 按 .trae/rules/项目核心设计意图.md §五：游戏考验生成走云端 API 模型，不应使用本地 Ollama。
                // 此处返回错误，让调用方走降级路径（如跳过图片素材、改用文本出题）。
                tracing::warn!(
                    model = %model.name,
                    "[ai_model_service] Ollama provider 不支持多模态视觉调用，请使用支持 vision 的云端模型"
                );
                Err(AppError::AiApi(
                    "Ollama provider 不支持多模态视觉调用，请使用支持 vision 的云端模型（如 glm-4v-plus / qwen-vl-max）".into()
                ))
            }
            _ => {
                self.call_openai_multimodal(&model, &api_key, messages, temp, tokens)
                    .await
            }
        }
    }

    /// D3.4 OpenAI 兼容多模态调用（内部方法）
    ///
    /// 直接用 serde_json::Value 构造请求体，绕过 OpenAIMessage.content: String 的限制，
    /// 使 content 可为 Vec<ContentBlock>（text + image_url）。
    async fn call_openai_multimodal(
        &self,
        model: &AiModel,
        api_key: &str,
        messages: Vec<serde_json::Value>,
        temperature: f64,
        max_tokens: i32,
    ) -> Result<String, AppError> {
        let api_url = model
            .api_url
            .as_deref()
            .or_else(|| get_provider_default_url(&model.provider))
            .unwrap_or("https://api.openai.com/v1");
        let model_name = model
            .model_name
            .as_deref()
            .or_else(|| get_provider_default_model(&model.provider))
            .unwrap_or("gpt-4o-mini");

        let request_body = serde_json::json!({
            "model": model_name,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
            "stream": false,
        });

        tracing::info!(
            model = %model.name,
            provider = &model.provider,
            api_url = %api_url,
            model_name = %model_name,
            "D3.4 多模态调用（OpenAI 兼容）"
        );

        let mut request_builder = self
            .client
            .post(&format!("{}/chat/completions", api_url))
            .json(&request_body);

        if !api_key.is_empty() {
            request_builder =
                request_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request_builder.send().await.map_err(|e| {
            AppError::AiApi(format!("多模态 API 请求失败: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::AiApi(format!(
                "{} 多模态 API 返回错误 [{}]: {}",
                model.provider, status, error_text
            )));
        }

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| AppError::AiApi(format!("多模态 API 响应解析失败: {}", e)))?;

        Ok(openai_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    /// D3.4 Anthropic 多模态调用（内部方法）
    ///
    /// 将 OpenAI 格式的 content blocks 转换为 Anthropic Messages API 格式：
    ///   - image_url: {url: "data:image/png;base64,xxx"} → image: {source: {type:"base64",...}}
    ///   - text 块保持不变
    ///   - system 消息分离到顶层 system 字段
    async fn call_anthropic_multimodal(
        &self,
        model: &AiModel,
        api_key: &str,
        messages: Vec<serde_json::Value>,
        temperature: f64,
        max_tokens: i32,
    ) -> Result<String, AppError> {
        let api_url = model
            .api_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com");
        let model_name = model
            .model_name
            .as_deref()
            .unwrap_or("claude-3-5-sonnet-20241022");

        let mut system_text = String::new();
        let mut anthropic_messages: Vec<serde_json::Value> = Vec::new();

        for msg in &messages {
            let role = msg["role"].as_str().unwrap_or("user");
            let content = &msg["content"];

            if role == "system" {
                // system 消息：提取文本到顶层
                if let Some(s) = content.as_str() {
                    system_text.push_str(s);
                } else if let Some(arr) = content.as_array() {
                    for block in arr {
                        if let Some(t) = block["text"].as_str() {
                            system_text.push_str(t);
                        }
                    }
                }
                continue;
            }

            // 转换 content 为 Anthropic 格式
            let anthropic_content: Vec<serde_json::Value> = if let Some(text) = content.as_str() {
                vec![serde_json::json!({"type": "text", "text": text})]
            } else if let Some(arr) = content.as_array() {
                arr.iter()
                    .map(|block| {
                        let block_type = block["type"].as_str().unwrap_or("text");
                        match block_type {
                            "text" => serde_json::json!({
                                "type": "text",
                                "text": block["text"].as_str().unwrap_or("")
                            }),
                            "image_url" => Self::convert_openai_image_to_anthropic(block),
                            _ => serde_json::json!({"type": "text", "text": "[未知块类型]"}),
                        }
                    })
                    .collect()
            } else {
                vec![serde_json::json!({"type": "text", "text": "(空内容)"})]
            };

            anthropic_messages.push(serde_json::json!({
                "role": if role == "assistant" { "assistant" } else { "user" },
                "content": anthropic_content,
            }));
        }

        let mut request_body = serde_json::json!({
            "model": model_name,
            "messages": anthropic_messages,
            "max_tokens": max_tokens,
            "temperature": temperature,
        });
        if !system_text.is_empty() {
            request_body["system"] = serde_json::json!(system_text);
        }

        tracing::info!(
            model = %model.name,
            provider = "anthropic",
            model_name = %model_name,
            "D3.4 多模态调用（Anthropic）"
        );

        let response = self
            .client
            .post(&format!("{}/v1/messages", api_url))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::AiApi(format!("Anthropic 多模态请求失败: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::AiApi(format!(
                "Anthropic 多模态 API 返回错误 [{}]: {}",
                status, error_text
            )));
        }

        let resp_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::AiApi(format!("Anthropic 多模态响应解析失败: {}", e)))?;

        // Anthropic 响应格式：{"content":[{"type":"text","text":"..."}]}
        Ok(resp_json["content"]
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .find_map(|b| b["text"].as_str().map(String::from))
            })
            .unwrap_or_default())
    }

    /// D3.4 将 OpenAI image_url 块转换为 Anthropic image source 格式
    fn convert_openai_image_to_anthropic(block: &serde_json::Value) -> serde_json::Value {
        let url = block["image_url"]["url"].as_str().unwrap_or("");
        // 解析 data URL: data:image/png;base64,xxxxx
        if let Some(stripped) = url.strip_prefix("data:") {
            if let Some(semicolon_pos) = stripped.find(';') {
                let media_type = &stripped[..semicolon_pos];
                if let Some(b64) = stripped[semicolon_pos..].strip_prefix(";base64,") {
                    return serde_json::json!({
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": media_type,
                            "data": b64
                        }
                    });
                }
            }
        }
        serde_json::json!({"type": "text", "text": "[图片格式不支持]"})
    }

    /// D3.1 OpenAI 兼容多轮对话（内部方法）
    async fn call_openai_messages(
        &self,
        model: &AiModel,
        api_key: &str,
        messages: Vec<(String, String)>,
        system_prompt: Option<&str>,
        temperature: f64,
    ) -> Result<String, AppError> {
        let api_url = model.api_url.as_deref()
            .or_else(|| get_provider_default_url(&model.provider))
            .unwrap_or("https://api.openai.com/v1");
        let model_name = model.model_name.as_deref()
            .or_else(|| get_provider_default_model(&model.provider))
            .unwrap_or("gpt-4o-mini");

        let mut msgs = vec![];
        if let Some(sys) = system_prompt {
            msgs.push(OpenAIMessage { role: "system".into(), content: sys.to_string() });
        }
        for (role, content) in messages {
            msgs.push(OpenAIMessage { role, content });
        }

        let request = OpenAIRequest {
            model: model_name.to_string(),
            messages: msgs,
            temperature,
            max_tokens: Some(2048),
            stream: false,
        };

        let mut request_builder = self
            .client
            .post(&format!("{}/chat/completions", api_url))
            .json(&request);

        if !api_key.is_empty() {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request_builder
            .send()
            .await
            .map_err(|e| AppError::AiApi(format!("OpenAI 兼容 API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::AiApi(format!("{} API 返回错误 [{}]: {}", model.provider, status, error_text)));
        }

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| AppError::AiApi(format!("{} API 响应解析失败: {}", model.provider, e)))?;

        let content = openai_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(content)
    }

    async fn call_ollama(
        &self,
        model: &AiModel,
        prompt: &str,
        _system_prompt: Option<&str>,
    ) -> Result<String, AppError> {
        let api_url = model.api_url.as_deref().unwrap_or("http://localhost:11434");
        let temperature = model.temperature.unwrap_or(0.7);

        let request = OllamaRequest {
            model: model.model_name.clone().unwrap_or_else(|| "llama3.2".to_string()),
            prompt: prompt.to_string(),
            stream: false,
            options: Some(OllamaOptions {
                temperature,
                top_p: 0.9,
                num_predict: Some(2048),
            }),
        };

        tracing::info!(model = %model.name, provider = "Ollama", "正在调用 AI 模型");

        let response = self
            .client
            .post(&format!("{}/api/generate", api_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::AiApi(format!("Ollama 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::AiApi(format!(
                "Ollama 返回错误 [{}]: {}",
                status, error_text
            )));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| AppError::AiApi(format!("Ollama 响应解析失败: {}", e)))?;

        tracing::info!(response_len = ollama_response.response.len(), "Ollama 调用成功");

        Ok(ollama_response.response)
    }

    async fn call_openai_compatible(
        &self,
        model: &AiModel,
        api_key: &str,
        prompt: &str,
        system_prompt: Option<&str>,
    ) -> Result<String, AppError> {
        let api_url = model.api_url.as_deref()
            .or_else(|| get_provider_default_url(&model.provider))
            .unwrap_or("https://api.openai.com/v1");
        let model_name = model.model_name.as_deref()
            .or_else(|| get_provider_default_model(&model.provider))
            .unwrap_or("gpt-4o-mini");
        let temperature = model.temperature.unwrap_or(0.7);

        let mut messages = vec![];

        if let Some(sys_prompt) = system_prompt {
            messages.push(OpenAIMessage {
                role: "system".to_string(),
                content: sys_prompt.to_string(),
            });
        }

        messages.push(OpenAIMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        });

        let request = OpenAIRequest {
            model: model_name.to_string(),
            messages,
            temperature,
            max_tokens: Some(2048),
            stream: false,
        };

        tracing::info!(model = %model.name, provider = &model.provider, api_url = %api_url, model_name = %model_name, "正在调用 AI 模型");

        let mut request_builder = self
            .client
            .post(&format!("{}/chat/completions", api_url))
            .json(&request);

        if !api_key.is_empty() {
            request_builder = request_builder.header(
                "Authorization",
                format!("Bearer {}", api_key),
            );
        }

        let response = request_builder
            .send()
            .await
            .map_err(|e| AppError::AiApi(format!("OpenAI 兼容 API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::AiApi(format!(
                "{} API 返回错误 [{}]: {}",
                model.provider, status, error_text
            )));
        }

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| AppError::AiApi(format!("{} API 响应解析失败: {}", model.provider, e)))?;

        let content = openai_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        tracing::info!(response_len = content.len(), provider = %model.provider, "AI 调用成功");

        Ok(content)
    }

    async fn call_anthropic(
        &self,
        model: &AiModel,
        api_key: &str,
        prompt: &str,
        system_prompt: Option<&str>,
        _temperature: f64,
    ) -> Result<String, AppError> {
        let api_url = model.api_url.as_deref()
            .or_else(|| get_provider_default_url(&model.provider))
            .unwrap_or("https://api.anthropic.com/v1");
        let model_name = model.model_name.as_deref()
            .or_else(|| get_provider_default_model(&model.provider))
            .unwrap_or("claude-sonnet-4-20250514");

        #[derive(Debug, Serialize)]
        struct AnthropicRequest {
            model: String,
            max_tokens: i32,
            system: Option<String>,
            messages: Vec<AnthropicMessage>,
        }

        #[derive(Debug, Serialize)]
        struct AnthropicMessage {
            role: String,
            content: String,
        }

        #[derive(Debug, Deserialize)]
        struct AnthropicResponse {
            content: Vec<AnthropicContent>,
        }

        #[derive(Debug, Deserialize)]
        struct AnthropicContent {
            text: String,
        }

        let request = AnthropicRequest {
            model: model_name.to_string(),
            max_tokens: 2048,
            system: system_prompt.map(|s| s.to_string()),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        tracing::info!(model = %model.name, provider = "Anthropic", "正在调用 AI 模型");

        let response = self
            .client
            .post(&format!("{}/messages", api_url))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::AiApi(format!("Anthropic API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::AiApi(format!(
                "Anthropic API 返回错误 [{}]: {}",
                status, error_text
            )));
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| AppError::AiApi(format!("Anthropic API 响应解析失败: {}", e)))?;

        let content = anthropic_response
            .content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        tracing::info!(response_len = content.len(), "Anthropic 调用成功");

        Ok(content)
    }

    // ========================================================================
    // spec 阶段1 Task 1.2：文生图 / 图片理解抽象接口
    // ========================================================================

    /// 文生图：调用云端 API（OpenAI DALL-E 3 / gpt-image-1）生成图片。
    ///
    /// 设计依据：
    /// - .trae/rules/项目核心设计意图.md §五（游戏考验生成走云端 API 模型）
    /// - .trae/rules/项目核心设计意图.md §八.1（AI 调用必须走云端 API，不接入底层智能）
    ///
    /// provider 路由：
    ///   - ollama     返回 Err（本地模型不支持文生图，调用方需降级到 placeholder://）
    ///   - openai     走 `/v1/images/generations`（DALL-E 3 / gpt-image-1）
    ///   - anthropic  返回 Err（Anthropic 暂无公开文生图 API，调用方需降级）
    ///   - 其他       尝试 OpenAI 兼容路径（custom / deepseek 等若提供文生图）
    ///
    /// 参数：
    /// - `prompt`：图片描述（建议含画面元素、风格、构图等）
    /// - `size`：目标尺寸
    ///
    /// 返回：图片 URL（http(s)://）。失败时返回 `AppError::AiApi`，由调用方降级处理。
    pub async fn generate_image(
        &self,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        model_id: i64,
        prompt: &str,
        size: ImageSize,
    ) -> Result<String, AppError> {
        let model = ai_repo::get_model_by_id(pool, model_id, user_id)
            .await?
            .ok_or_else(|| AppError::AiApi("模型不存在".into()))?;
        let api_key = Self::decrypt_api_key(&model, mek_manager, user_id).await?;

        match model.provider.to_lowercase().as_str() {
            "ollama" => Err(AppError::AiApi(
                "Ollama 本地模型不支持文生图，请降级到 placeholder://".into(),
            )),
            "anthropic" => Err(AppError::AiApi(
                "Anthropic 暂无公开文生图 API，请降级到 placeholder://".into(),
            )),
            _ => self.call_openai_image_generation(&model, &api_key, prompt, size).await,
        }
    }

    /// OpenAI 兼容文生图内部方法（DALL-E 3 / gpt-image-1）。
    ///
    /// 调用 `POST {api_url}/images/generations`：
    /// - model: 默认 `dall-e-3`（可由 AiModel.model_name 覆盖为 `gpt-image-1`）
    /// - size: 由 `ImageSize::as_openai_size()` 转换
    /// - response_format: "url"（直接返回 URL，避免大 base64 撑爆响应）
    /// - n: 1
    async fn call_openai_image_generation(
        &self,
        model: &AiModel,
        api_key: &str,
        prompt: &str,
        size: ImageSize,
    ) -> Result<String, AppError> {
        let api_url = model
            .api_url
            .as_deref()
            .or_else(|| get_provider_default_url(&model.provider))
            .unwrap_or("https://api.openai.com/v1");
        let model_name = model
            .model_name
            .as_deref()
            .unwrap_or("dall-e-3");

        #[derive(Debug, Serialize)]
        struct ImageGenRequest<'a> {
            model: &'a str,
            prompt: &'a str,
            n: u8,
            size: &'a str,
            response_format: &'a str,
        }

        #[derive(Debug, Deserialize)]
        struct ImageGenResponse {
            data: Vec<ImageGenItem>,
        }

        #[derive(Debug, Deserialize)]
        struct ImageGenItem {
            url: Option<String>,
            b64_json: Option<String>,
        }

        let request = ImageGenRequest {
            model: model_name,
            prompt,
            n: 1,
            size: size.as_openai_size(),
            response_format: "url",
        };

        tracing::info!(
            model = %model.name,
            provider = &model.provider,
            api_url = %api_url,
            model_name = %model_name,
            size = %size.as_openai_size(),
            "spec Task 1.2: 调用文生图 API"
        );

        let mut request_builder = self
            .client
            .post(&format!("{}/images/generations", api_url))
            .json(&request);

        if !api_key.is_empty() {
            request_builder =
                request_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request_builder.send().await.map_err(|e| {
            AppError::AiApi(format!("文生图 API 请求失败: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::AiApi(format!(
                "文生图 API 返回错误 [{}]: {}",
                status, error_text
            )));
        }

        let parsed: ImageGenResponse = response
            .json()
            .await
            .map_err(|e| AppError::AiApi(format!("文生图 API 响应解析失败: {}", e)))?;

        let item = parsed
            .data
            .into_iter()
            .next()
            .ok_or_else(|| AppError::AiApi("文生图 API 响应缺少 data 字段".into()))?;

        // 优先返回 URL；若 API 返回 b64_json（部分 provider 不支持 response_format=url），降级为 data URL
        if let Some(url) = item.url {
            tracing::info!(url_len = url.len(), "文生图调用成功（URL 模式）");
            Ok(url)
        } else if let Some(b64) = item.b64_json {
            tracing::info!(b64_len = b64.len(), "文生图调用成功（b64_json 模式，转换为 data URL）");
            Ok(format!("data:image/png;base64,{}", b64))
        } else {
            Err(AppError::AiApi(
                "文生图 API 响应既无 url 也无 b64_json".into(),
            ))
        }
    }

    /// 图片理解：调用多模态 vision API（GPT-4V / Claude Vision / glm-4v）理解图片内容。
    ///
    /// 设计依据：
    /// - .trae/rules/项目核心设计意图.md §五（游戏考验生成走云端 API 模型）
    /// - 复用 D3.4 已实现的 `call_model_multimodal`（OpenAI Vision content blocks + Anthropic 转换 + Ollama 降级）
    ///
    /// 参数：
    /// - `image_url`：图片 URL 或 data URL（`http(s)://...` 或 `data:image/...;base64,...`）
    /// - `prompt`：用户提问（如"基于这张图片出 1 道选择题，考察知识点 X"）
    ///
    /// 返回：模型生成的文本响应（通常为 JSON 或自然语言描述）。
    pub async fn understand_image(
        &self,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        model_id: i64,
        image_url: &str,
        prompt: &str,
    ) -> Result<String, AppError> {
        // 构造 OpenAI Vision content blocks：[image_url, text]
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": [
                {
                    "type": "image_url",
                    "image_url": { "url": image_url }
                },
                {
                    "type": "text",
                    "text": prompt
                }
            ]
        })];

        tracing::info!(
            model_id,
            image_url_len = image_url.len(),
            prompt_len = prompt.len(),
            "spec Task 1.2: 调用图片理解 API（vision input）"
        );

        self.call_model_multimodal(
            pool,
            mek_manager,
            user_id,
            model_id,
            messages,
            Some(0.7),
            Some(2048),
        )
        .await
    }
}
