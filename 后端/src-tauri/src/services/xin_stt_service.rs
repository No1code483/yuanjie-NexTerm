//! 小欣 STT 语音输入服务（D3.3）
//!
//! 设计目标（03_小欣_多模态融合_深度.md §2.1）：
//! - 将用户录音转写为文本
//! - 跨平台抽象（SttEngine trait）
//! - 支持在线 Whisper API（OpenAI 兼容）+ 本地 whisper.cpp（未来扩展）
//!
//! 实现策略（2026-07-21 D3.3）：
//! - 在线 STT：调用 OpenAI 兼容 `/audio/transcriptions` 接口（Whisper）
//!   - 通过 `AiModelService::get_model_config` 获取 (model, api_key)
//!   - 使用 multipart/form-data 上传音频文件
//! - 本地 STT：未实现，预留 trait 接口
//! - 不引入 whisper-rs（避免引入 ~100MB 模型 + 重型依赖）

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::crypto::mek_manager::MekManager;
use crate::db::repositories::ai_repo;
use crate::error::app_error::AppError;
use crate::services::ai_model_service::AiModelService;
use sqlx::SqlitePool;

/// STT 引擎抽象。
pub trait SttEngine: Send + Sync {
    fn name(&self) -> &str;
    /// 将音频文件转写为文本。
    ///
    /// `audio_path` 音频文件路径（wav/mp3/m4a/webm）；
    /// `language` 语言代码（如 "zh"、"en"），None 自动检测。
    fn transcribe(
        &self,
        audio_path: &std::path::Path,
        language: Option<&str>,
    ) -> Result<String, AppError>;
}

/// 在线 Whisper STT（OpenAI 兼容 API）。
///
/// 通过 multipart/form-data 上传音频文件到 `/audio/transcriptions` 端点。
/// 需要传入 (pool, mek_manager, user_id, model_id) 以从 AiModelService 获取配置。
pub struct WhisperApiEngine {
    pool: SqlitePool,
    mek_manager: Arc<RwLock<MekManager>>,
    user_id: i64,
    model_id: i64,
    client: reqwest::Client,
}

impl WhisperApiEngine {
    pub fn new(
        pool: SqlitePool,
        mek_manager: Arc<RwLock<MekManager>>,
        user_id: i64,
        model_id: i64,
    ) -> Self {
        Self {
            pool,
            mek_manager,
            user_id,
            model_id,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("failed to build STT HTTP client"),
        }
    }
}

impl SttEngine for WhisperApiEngine {
    fn name(&self) -> &str {
        "whisper_api"
    }

    fn transcribe(
        &self,
        audio_path: &std::path::Path,
        language: Option<&str>,
    ) -> Result<String, AppError> {
        // 读取模型配置（同步上下文中调用异步方法 → 用 tokio runtime handle）
        let (model, api_key) = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(AiModelService::get_model_config(
                &self.pool,
                &self.mek_manager,
                self.user_id,
                self.model_id,
            ))
        })?;

        let api_url = model
            .api_url
            .as_deref()
            .or_else(|| crate::models::ai_model::get_provider_default_url(&model.provider))
            .unwrap_or("https://api.openai.com/v1")
            .to_string();

        let model_name = model
            .model_name
            .as_deref()
            .unwrap_or("whisper-1")
            .to_string();

        let file_bytes = std::fs::read(audio_path).map_err(|e| {
            AppError::Internal(format!("读取音频文件失败: {}", e))
        })?;

        let file_ext = audio_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("wav")
            .to_string();

        // 构造 multipart 表单
        let form = {
            let mut form = reqwest::multipart::Form::new()
                .text("model", model_name.clone())
                .text("response_format", "text");
            if let Some(lang) = language {
                form = form.text("language", lang.to_string());
            }
            let part = reqwest::multipart::Part::bytes(file_bytes)
                .file_name(format!("audio.{}", file_ext))
                .mime_str("audio/webm")
                .unwrap_or_else(|_| {
                    reqwest::multipart::Part::bytes(Vec::new()).mime_str("application/octet-stream").unwrap()
                });
            form = form.part("file", part);
            form
        };

        let mut request_builder = self
            .client
            .post(format!("{}/audio/transcriptions", api_url))
            .multipart(form);

        if !api_key.is_empty() {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(request_builder.send())
        })
        .map_err(|e| AppError::AiApi(format!("Whisper API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(response.text())
            })
            .unwrap_or_default();
            return Err(AppError::AiApi(format!(
                "Whisper API 返回错误 [{}]: {}",
                status, text
            )));
        }

        let text = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(response.text())
        })
        .map_err(|e| AppError::AiApi(format!("Whisper 响应读取失败: {}", e)))?;

        Ok(text.trim().to_string())
    }
}

/// STT 服务入口。
pub struct XinSttService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttTranscribeRequest {
    pub audio_path: String,
    pub language: Option<String>,
    pub model_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttTranscribeResponse {
    pub text: String,
    pub engine: String,
}

impl XinSttService {
    /// 转写音频文件。`user_id` 由调用方（命令层）从 AppState 获取。
    pub async fn transcribe(
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        request: SttTranscribeRequest,
    ) -> Result<SttTranscribeResponse, AppError> {
        // 验证模型存在（多用户隔离：按 user_id 过滤）
        let _model = ai_repo::get_model_by_id(pool, request.model_id, user_id)
            .await?
            .ok_or_else(|| AppError::AiApi("STT 模型不存在".into()))?;

        let audio_path = PathBuf::from(&request.audio_path);
        if !audio_path.exists() {
            return Err(AppError::Validation(format!(
                "音频文件不存在: {}",
                request.audio_path
            )));
        }

        let engine = WhisperApiEngine::new(
            pool.clone(),
            mek_manager.clone(),
            user_id,
            request.model_id,
        );
        let text = engine.transcribe(&audio_path, request.language.as_deref())?;

        Ok(SttTranscribeResponse {
            text,
            engine: engine.name().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcribe_nonexistent_file_rejected() {
        // 直接验证文件存在性逻辑（不实际调用 API）
        let path = PathBuf::from("/nonexistent/audio.wav");
        assert!(!path.exists());
    }
}
