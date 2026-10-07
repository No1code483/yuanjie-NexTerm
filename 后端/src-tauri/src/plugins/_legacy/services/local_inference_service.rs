//! 恐龙双脑 Phase 0：本地推理框架 PoC
//!
//! 设计依据：
//!   - 功能展望/v2_核心战略/01_恐龙双脑智能架构_实施路线图_补充.md §2.1 Phase 0
//!   - .trae/rules/项目核心设计意图.md §8.1（底层智能走本地模型）
//!
//! Phase 0 任务：推理框架 PoC（接口骨架）
//!   - V4：走 ollama qwen3:8b（本地模型）
//!   - V5：替换为恐龙双脑定制模型（GGUF 加载，Phase 1+ 实现）
//!
//! 本模块是推理框架的接口骨架，Phase 0 暂用 ollama 作为推理后端占位。
//! 遵循项目核心设计意图 §8.1：底层智能的 AI 调用走本地模型（非云端 API）。

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::app_error::AppError;

/// 本地推理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalInferenceConfig {
    /// Ollama 服务地址
    pub ollama_url: String,
    /// 模型名称（V4: qwen3:8b / V5: 恐龙双脑定制模型名）
    pub model_name: String,
    /// 采样温度
    pub temperature: f64,
    /// 上下文窗口大小（token 数）
    pub context_size: u32,
}

impl Default for LocalInferenceConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".into(),
            model_name: "qwen3:8b".into(),
            temperature: 0.7,
            context_size: 4096,
        }
    }
}

/// 本地推理服务（Phase 0 PoC）
///
/// V5 将替换为恐龙双脑定制模型推理（GGUF 加载 + 量化），
/// 当前 Phase 0 通过 ollama 验证推理管线可用性。
pub struct LocalInferenceService {
    config: LocalInferenceConfig,
    client: reqwest::Client,
}

impl LocalInferenceService {
    pub fn new(config: LocalInferenceConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { config, client }
    }

    /// 使用默认配置创建（ollama qwen3:8b）
    pub fn with_defaults() -> Self {
        Self::new(LocalInferenceConfig::default())
    }

    /// 检查本地推理后端是否可用
    pub async fn check_available(&self) -> Result<bool, AppError> {
        let url = format!("{}/api/tags", self.config.ollama_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("无法连接 ollama: {}", e)))?;
        Ok(resp.status().is_success())
    }

    /// 本地推理
    ///
    /// Phase 0 PoC：调用 ollama `/api/generate`。
    /// V5 将替换为恐龙双脑定制模型推理（GGUF 加载），Phase 1+ 实现。
    /// 遵循项目核心设计意图 §8.1：底层智能的 AI 调用走本地模型。
    pub async fn infer(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
    ) -> Result<String, AppError> {
        let full_prompt = match system_prompt {
            Some(sys) => format!("System: {}\n\nUser: {}", sys, prompt),
            None => prompt.to_string(),
        };

        let body = serde_json::json!({
            "model": self.config.model_name,
            "prompt": full_prompt,
            "stream": false,
            "options": {
                "temperature": self.config.temperature,
                "num_ctx": self.config.context_size,
            }
        });

        let url = format!("{}/api/generate", self.config.ollama_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::AiApi(format!("ollama 推理请求失败: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::AiApi(format!(
                "ollama 推理失败 [{}]: {}",
                status, text
            )));
        }

        let parsed: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::AiApi(format!("ollama 推理响应解析失败: {}", e)))?;

        let output = parsed
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        tracing::info!(
            "[D2-Phase0] 本地推理完成（模型: {}, 输出 {} 字符）",
            self.config.model_name,
            output.len()
        );

        Ok(output)
    }

    pub fn config(&self) -> &LocalInferenceConfig {
        &self.config
    }
}
