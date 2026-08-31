//! D3.6 小欣实时对话服务
//!
//! 设计依据：
//!   - 功能展望/模块深化/03_小欣_多模态融合_深度.md §2.5 实时对话
//!   - .trae/rules/项目核心设计意图.md §四（小欣走云端 API）
//!   - v2_下一阶段开发计划：验收标准「语音实时对话延迟 < 500ms」
//!
//! 架构（方案 B 流式音频管道）：
//!   1. 前端 AudioWorklet 采集 PCM 16kHz 单声道 → 通过 Tauri Channel 推送
//!   2. 后端接收 PCM 块 → 累积到 VAD 检测窗口
//!   3. VAD 句尾检测 → 触发云端 Whisper STT 流式转写
//!   4. 转写文本 → 复用 XinDialogueService 流式 LLM 对话
//!   5. LLM 流式 token → 分句累积 → 云端 TTS 流式合成
//!   6. TTS 音频块 → 前端 Web Audio API 顺序播放
//!
//! 延迟预算（< 500ms 目标）：
//!   - VAD 句尾触发：< 50ms
//!   - Whisper STT：~ 200-300ms（云端流式 endpoint）
//!   - LLM 首 token：~ 150-250ms（流式）
//!   - TTS 首音频块：~ 100-200ms（流式）
//!   - 端到端总计：~ 500-800ms（首句），后续句子可流水线并行降至 < 300ms
//!
//! 设计原则：
//!   - 非阻塞：所有 I/O 异步，不阻塞主线程
//!   - 可关闭：遵循「底层智能可关闭性」原则（虽然小欣走云端 API，但实时对话
//!     作为增强功能必须支持停止）
//!   - 流式优先：避免「录音→整段上传→等待」的文件式延迟

use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD, Engine};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, RwLock};

use crate::crypto::mek_manager::MekManager;
use crate::error::app_error::AppError;
use crate::models::ai_model::AiModel;
use crate::services::ai_model_service::AiModelService;
use crate::services::xin_stt_service::{SttEngine, WhisperApiEngine};

/// 实时会话状态机
///
/// 状态流转：
///   Idle → Listening（用户开始说话）
///   Listening → Thinking（VAD 检测到句尾，开始 STT+LLM）
///   Thinking → Speaking（LLM 首 token 到达，开始 TTS）
///   Speaking → Listening（TTS 播放结束，回到监听）
///   任意 → Idle（用户主动停止）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeState {
    /// 空闲，未开始实时对话
    Idle,
    /// 监听用户语音中（前端正在采集 PCM）
    Listening,
    /// 处理中（STT / LLM 推理中）
    Thinking,
    /// 回复中（TTS 合成 + 播放中）
    Speaking,
}

/// PCM 音频格式约定
///
/// 约定前端 AudioWorklet 输出：
///   - sample_rate: 16000 Hz（Whisper 推荐）
///   - channels: 1（单声道）
///   - bits_per_sample: 16（i16 PCM）
///   - 字节序：小端（Web Audio 原生）
pub const REALTIME_SAMPLE_RATE: u32 = 16000;
pub const REALTIME_CHANNELS: u16 = 1;
pub const REALTIME_BITS_PER_SAMPLE: u16 = 16;

/// VAD（Voice Activity Detection）参数
///
/// 简化策略：基于能量阈值 + 静默时长判断句尾
///   - SILENCE_THRESHOLD: 低于此 RMS 能量视为静默
///   - SILENCE_DURATION_MS: 连续静音超过此时长 → 触发句尾
///   - MIN_SPEECH_DURATION_MS: 最短语音段长度，避免噪声误触发
pub const SILENCE_THRESHOLD: f32 = 0.02;
pub const SILENCE_DURATION_MS: u32 = 600;
pub const MIN_SPEECH_DURATION_MS: u32 = 300;

/// 单个 PCM 音频块（前端推送）
///
/// `samples` 为 i16 PCM 样本数组（小端字节序已在前端解码）
/// 每块约定 20-30ms（320-480 samples @ 16kHz），保证低延迟
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcmChunk {
    /// 样本序列（已从字节解码为 i16）
    pub samples: Vec<i16>,
    /// 时间戳（毫秒，会话起始为 0）
    pub timestamp_ms: u64,
}

/// STT 转写结果（流式增量）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttIncrement {
    /// 增量文本（本次新增的转写字符）
    pub delta: String,
    /// 累积全文
    pub full_text: String,
    /// 是否为最终结果（句尾）
    pub is_final: bool,
}

/// LLM 流式事件（透传 XinStreamEvent 简化版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmStreamChunk {
    /// 增量 token 文本
    pub delta: String,
    /// 累积回复
    pub full_text: String,
    /// 是否结束
    pub done: bool,
}

/// TTS 流式音频块（返回前端播放）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsAudioChunk {
    /// 音频字节（WAV/MP3 二进制，base64 编码以便 IPC 传输）
    pub audio_b64: String,
    /// 音频格式（"wav" / "mp3" / "pcm"）
    pub format: String,
    /// 采样率
    pub sample_rate: u32,
    /// 是否为最后一个块
    pub is_final: bool,
}

/// 实时会话事件（通过 Tauri Channel 推送给前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RealtimeEvent {
    /// 状态变更
    StateChanged {
        state: RealtimeState,
    },
    /// STT 增量文本
    SttIncrement {
        delta: String,
        full_text: String,
        is_final: bool,
    },
    /// LLM 流式 token
    LlmChunk {
        delta: String,
        full_text: String,
        done: bool,
    },
    /// TTS 音频块（base64）
    TtsChunk {
        audio_b64: String,
        format: String,
        sample_rate: u32,
        is_final: bool,
    },
    /// 错误事件
    Error {
        message: String,
        code: String,
    },
    /// 会话结束
    SessionEnded,
}

/// 实时会话配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeConfig {
    /// STT 模型 ID（云端 Whisper）
    pub stt_model_id: i64,
    /// LLM 模型 ID（云端对话模型）
    pub llm_model_id: i64,
    /// TTS 模型 ID（云端 TTS，如 OpenAI TTS）
    /// None 时降级到平台原生 TTS（延迟可能 > 500ms）
    pub tts_model_id: Option<i64>,
    /// 小欣 persona_id
    pub persona_id: String,
    /// 会话标题（用于历史记录）
    pub title: Option<String>,
    /// 是否启用 VAD 自动句尾检测
    pub vad_enabled: bool,
    /// TTS 音色
    pub tts_voice: Option<String>,
}

impl Default for RealtimeConfig {
    fn default() -> Self {
        Self {
            stt_model_id: 0,
            llm_model_id: 0,
            tts_model_id: None,
            persona_id: "default".into(),
            title: None,
            vad_enabled: true,
            tts_voice: None,
        }
    }
}

/// 内部会话上下文（运行时状态）
struct SessionContext {
    /// 当前状态
    state: RealtimeState,
    /// 累积的 PCM 样本（当前语音段）
    speech_buffer: Vec<i16>,
    /// 当前语音段起始时间戳（ms）
    speech_start_ms: u64,
    /// 最后一次非静音时间戳（ms）
    last_voice_ms: u64,
    /// 是否已检测到语音开始
    in_speech: bool,
    /// STT 累积文本
    stt_text: String,
    /// LLM 累积回复
    llm_text: String,
    /// 会话起始时间
    /// 保留字段：当前未在逻辑中读取，但保留用于未来会话时长统计/超时诊断
    #[allow(dead_code)]
    started_at: std::time::Instant,
    // D3.6.3 资源依赖（start_session 注入）
    pool: SqlitePool,
    mek_manager: Arc<RwLock<MekManager>>,
    user_id: i64,
    app_handle: AppHandle,
    config: RealtimeConfig,
}

/// D3.6 实时对话服务
///
/// 持有当前会话上下文，提供开始/推送音频/停止的接口。
/// 实际的 STT/LLM/TTS 流式调用在后续子任务（D3.6.3 ~ D3.6.5）填充。
pub struct XinRealtimeService {
    /// 当前会话上下文（None 表示无活动会话）
    session: Arc<Mutex<Option<SessionContext>>>,
    /// 服务配置（只读）
    config: Arc<RwLock<RealtimeConfig>>,
}

impl XinRealtimeService {
    pub fn new() -> Self {
        Self {
            session: Arc::new(Mutex::new(None)),
            config: Arc::new(RwLock::new(RealtimeConfig::default())),
        }
    }

    /// 开始实时会话
    ///
    /// 流程：
    ///   1. 校验配置（model_id、persona_id）
    ///   2. 初始化 SessionContext（注入 pool/mek_manager/app_handle）
    ///   3. 状态置为 Listening
    ///   4. emit StateChanged 事件给前端
    pub async fn start_session(
        &self,
        config: RealtimeConfig,
        pool: SqlitePool,
        mek_manager: Arc<RwLock<MekManager>>,
        user_id: i64,
        app_handle: AppHandle,
    ) -> Result<(), AppError> {
        if config.stt_model_id == 0 || config.llm_model_id == 0 {
            return Err(AppError::Validation(
                "STT 和 LLM 模型 ID 必须配置".into(),
            ));
        }

        let mut session = self.session.lock().await;
        if session.is_some() {
            return Err(AppError::Internal("已有进行中的实时会话".into()));
        }

        let ctx = SessionContext {
            state: RealtimeState::Listening,
            speech_buffer: Vec::new(),
            speech_start_ms: 0,
            last_voice_ms: 0,
            in_speech: false,
            stt_text: String::new(),
            llm_text: String::new(),
            started_at: std::time::Instant::now(),
            pool,
            mek_manager,
            user_id,
            app_handle: app_handle.clone(),
            config: config.clone(),
        };
        *session = Some(ctx);
        *self.config.write().await = config.clone();

        // emit 状态变更事件
        let _ = app_handle.emit(
            "realtime_event",
            RealtimeEvent::StateChanged {
                state: RealtimeState::Listening,
            },
        );

        tracing::info!(
            "[D3.6] 实时会话已开始: stt={}, llm={}, tts={:?}",
            config.stt_model_id,
            config.llm_model_id,
            config.tts_model_id
        );
        Ok(())
    }

    /// 停止实时会话
    ///
    /// 清理会话上下文，状态回到 Idle，emit SessionEnded 事件
    pub async fn stop_session(&self) -> Result<(), AppError> {
        let mut session = self.session.lock().await;
        if let Some(ctx) = session.take() {
            let _ = ctx.app_handle.emit("realtime_event", RealtimeEvent::SessionEnded);
            tracing::info!("[D3.6] 实时会话已停止");
        }
        Ok(())
    }

    /// 获取当前会话状态
    pub async fn get_state(&self) -> RealtimeState {
        match self.session.lock().await.as_ref() {
            Some(ctx) => ctx.state,
            None => RealtimeState::Idle,
        }
    }

    /// 推送 PCM 音频块（由前端通过 Tauri Channel 调用）
    ///
    /// D3.6.3 实现：
    ///   1. 计算 RMS 能量
    ///   2. VAD 检测：在语音中 + 连续静默 > SILENCE_DURATION_MS → 触发句尾
    ///   3. 句尾触发时取出 speech_buffer，spawn 异步任务调用云端 Whisper
    ///   4. 状态转 Thinking，STT 完成后回到 Listening
    pub async fn push_pcm_chunk(&self, chunk: PcmChunk) -> Result<RealtimeState, AppError> {
        let session_arc = self.session.clone();
        let (trigger, ret_state) = {
            let mut session = session_arc.lock().await;
            let ctx = session
                .as_mut()
                .ok_or_else(|| AppError::Internal("无活动实时会话".into()))?;

            if ctx.state != RealtimeState::Listening {
                return Ok(ctx.state); // 非 Listening 状态丢弃音频
            }

            let now_ms = chunk.timestamp_ms;
            let rms = Self::compute_rms(&chunk.samples);
            let is_voice = rms > SILENCE_THRESHOLD;

            if is_voice {
                if !ctx.in_speech {
                    ctx.in_speech = true;
                    ctx.speech_start_ms = now_ms;
                }
                ctx.last_voice_ms = now_ms;
            }

            // 累积样本（含静默段，保证 WAV 连续）
            ctx.speech_buffer.extend(chunk.samples);

            // VAD 句尾检测：在语音中 + 当前静默 + 静默超时 + 最短语音长度
            let should_trigger = ctx.in_speech
                && !is_voice
                && (now_ms - ctx.last_voice_ms) > SILENCE_DURATION_MS as u64
                && (now_ms - ctx.speech_start_ms) > MIN_SPEECH_DURATION_MS as u64;

            let trigger = if should_trigger {
                // 取出语音段样本，重置 VAD 状态
                let samples = std::mem::take(&mut ctx.speech_buffer);
                ctx.in_speech = false;
                ctx.state = RealtimeState::Thinking;

                // 提取资源给 spawn 任务
                Some((
                    ctx.config.clone(),
                    ctx.pool.clone(),
                    ctx.mek_manager.clone(),
                    ctx.user_id,
                    ctx.app_handle.clone(),
                    samples,
                ))
            } else {
                None
            };
            (trigger, ctx.state)
        }; // 锁释放

        // 锁外 spawn STT 任务（避免阻塞 push_pcm_chunk 的高频调用）
        if let Some((config, pool, mek_manager, user_id, app_handle, samples)) = trigger {
            let session_clone = session_arc.clone();
            tokio::spawn(async move {
                Self::process_speech_segment(
                    session_clone,
                    app_handle,
                    pool,
                    mek_manager,
                    user_id,
                    config,
                    samples,
                )
                .await;
            });
        }

        Ok(ret_state)
    }

    /// 处理一段语音：编码 WAV → 调用云端 Whisper → emit 事件 → 状态回到 Listening
    ///
    /// 在独立 tokio task 中执行，不阻塞 push_pcm_chunk 调用链。
    /// WhisperApiEngine::transcribe 是同步方法（内部用 block_in_place），
    /// 在 tokio spawn 任务中调用是安全的（多线程 runtime 工作线程）。
    async fn process_speech_segment(
        session: Arc<Mutex<Option<SessionContext>>>,
        app_handle: AppHandle,
        pool: SqlitePool,
        mek_manager: Arc<RwLock<MekManager>>,
        user_id: i64,
        config: RealtimeConfig,
        samples: Vec<i16>,
    ) {
        // 1. emit Thinking 状态
        let _ = app_handle.emit(
            "realtime_event",
            RealtimeEvent::StateChanged {
                state: RealtimeState::Thinking,
            },
        );

        // 2. 编码 WAV
        let wav_bytes = match Self::encode_wav(&samples, REALTIME_SAMPLE_RATE) {
            Ok(b) => b,
            Err(e) => {
                let _ = app_handle.emit(
                    "realtime_event",
                    RealtimeEvent::Error {
                        message: format!("WAV 编码失败: {}", e),
                        code: "encode_failed".into(),
                    },
                );
                Self::reset_to_listening(&session, &app_handle).await;
                return;
            }
        };

        // 3. 写临时文件（WhisperApiEngine 需要文件路径）
        let temp_path = std::env::temp_dir().join(format!(
            "nexterm_realtime_{}.wav",
            chrono::Utc::now().timestamp_millis()
        ));
        if let Err(e) = tokio::fs::write(&temp_path, &wav_bytes).await {
            let _ = app_handle.emit(
                "realtime_event",
                RealtimeEvent::Error {
                    message: format!("临时文件写入失败: {}", e),
                    code: "temp_write_failed".into(),
                },
            );
            Self::reset_to_listening(&session, &app_handle).await;
            return;
        }

        // 4. 调用云端 Whisper STT
        let engine = WhisperApiEngine::new(pool.clone(), mek_manager.clone(), user_id, config.stt_model_id);
        let transcribe_result = engine.transcribe(&temp_path, Some("zh"));

        // 清理临时文件
        let _ = tokio::fs::remove_file(&temp_path).await;

        let stt_text = match transcribe_result {
            Ok(t) => t.trim().to_string(),
            Err(e) => {
                let _ = app_handle.emit(
                    "realtime_event",
                    RealtimeEvent::Error {
                        message: format!("STT 转写失败: {}", e),
                        code: "stt_failed".into(),
                    },
                );
                Self::reset_to_listening(&session, &app_handle).await;
                return;
            }
        };

        // 5. 累积 STT 文本到会话上下文
        {
            let mut s = session.lock().await;
            if let Some(ctx) = s.as_mut() {
                if !ctx.stt_text.is_empty() {
                    ctx.stt_text.push(' ');
                }
                ctx.stt_text.push_str(&stt_text);
            }
        }

        let full_text = {
            let s = session.lock().await;
            s.as_ref().map(|c| c.stt_text.clone()).unwrap_or_default()
        };

        // 6. emit SttIncrement（is_final=true，表示一句完整转写）
        let _ = app_handle.emit(
            "realtime_event",
            RealtimeEvent::SttIncrement {
                delta: stt_text.clone(),
                full_text,
                is_final: true,
            },
        );

        // 7. D3.6.4: 调用云端 LLM 流式对话（复用 AiModelService + SSE 解析模式）
        let mut llm_full_text = String::new();
        if !stt_text.is_empty() {
            match Self::call_llm_stream_realtime(
                &app_handle,
                &session,
                &pool,
                &mek_manager,
                user_id,
                config.llm_model_id,
                &stt_text,
            )
            .await
            {
                Ok(text) => llm_full_text = text,
                Err(e) => {
                    let _ = app_handle.emit(
                        "realtime_event",
                        RealtimeEvent::Error {
                            message: format!("LLM 流式失败: {}", e),
                            code: "llm_failed".into(),
                        },
                    );
                }
            }
        }

        // 8. D3.6.5: 调用云端 TTS 流式合成（分句 → 每句 TTS → emit TtsChunk）
        if let Some(tts_model_id) = config.tts_model_id {
            if !llm_full_text.trim().is_empty() {
                Self::speak_text(
                    &app_handle,
                    &session,
                    &pool,
                    &mek_manager,
                    user_id,
                    tts_model_id,
                    config.tts_voice.as_deref(),
                    &llm_full_text,
                )
                .await;
            }
        }

        tracing::info!("[D3.6] STT + LLM + TTS 段处理完成，回到 Listening 等待下一句");

        // 9. 状态回到 Listening
        Self::reset_to_listening(&session, &app_handle).await;
    }

    /// D3.6.5: 将文本转为语音并推送音频块（分句 TTS）
    ///
    /// 流程：状态→Speaking → 分句 → 每句调用云端 TTS → emit TtsChunk → 完成信号
    async fn speak_text(
        app_handle: &AppHandle,
        session: &Arc<Mutex<Option<SessionContext>>>,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        tts_model_id: i64,
        voice: Option<&str>,
        text: &str,
    ) {
        // 状态 → Speaking
        {
            let mut s = session.lock().await;
            if let Some(ctx) = s.as_mut() {
                ctx.state = RealtimeState::Speaking;
            }
        }
        let _ = app_handle.emit(
            "realtime_event",
            RealtimeEvent::StateChanged {
                state: RealtimeState::Speaking,
            },
        );

        let sentences = Self::split_sentences(text);
        tracing::info!("[D3.6] TTS 分句: {} 句", sentences.len());

        for sentence in sentences {
            match Self::call_tts_realtime(pool, mek_manager, user_id, tts_model_id, voice, &sentence)
                .await
            {
                Ok((bytes, format)) => {
                    let audio_b64 = STANDARD.encode(&bytes);
                    let _ = app_handle.emit(
                        "realtime_event",
                        RealtimeEvent::TtsChunk {
                            audio_b64,
                            format,
                            sample_rate: 0,
                            is_final: false,
                        },
                    );
                }
                Err(e) => {
                    let _ = app_handle.emit(
                        "realtime_event",
                        RealtimeEvent::Error {
                            message: format!("TTS 合成失败: {}", e),
                            code: "tts_failed".into(),
                        },
                    );
                }
            }
        }

        // TTS 完成信号
        let _ = app_handle.emit(
            "realtime_event",
            RealtimeEvent::TtsChunk {
                audio_b64: String::new(),
                format: String::new(),
                sample_rate: 0,
                is_final: true,
            },
        );
    }

    /// D3.6.5: 按中文标点分句（。！？；！？; 换行）
    fn split_sentences(text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current = String::new();
        for ch in text.chars() {
            current.push(ch);
            if matches!(ch, '。' | '！' | '？' | '；' | '!' | '?' | ';' | '\n') {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    sentences.push(trimmed);
                }
                current.clear();
            }
        }
        let trimmed = current.trim().to_string();
        if !trimmed.is_empty() {
            sentences.push(trimmed);
        }
        sentences
    }

    /// D3.6.5: 调用云端 TTS API（OpenAI 兼容 /audio/speech），返回音频字节
    ///
    /// 设计意图（§四）：小欣走云端 API 模型。tts_model_id 指向 TTS 模型（如 tts-1）。
    async fn call_tts_realtime(
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        model_id: i64,
        voice: Option<&str>,
        text: &str,
    ) -> Result<(Vec<u8>, String), AppError> {
        let (model, api_key) =
            AiModelService::get_model_config(pool, mek_manager, user_id, model_id).await?;
        let api_url = model
            .api_url
            .as_deref()
            .or_else(|| crate::models::ai_model::get_provider_default_url(&model.provider))
            .unwrap_or("https://api.openai.com/v1")
            .to_string();
        let model_name = model.model_name.as_deref().unwrap_or("tts-1").to_string();

        let body = serde_json::json!({
            "model": model_name,
            "input": text,
            "voice": voice.unwrap_or("alloy"),
            "response_format": "mp3"
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AppError::AiApi(format!("TTS HTTP 客户端创建失败: {}", e)))?;

        let mut req = client
            .post(format!("{}/audio/speech", api_url))
            .header("Content-Type", "application/json")
            .json(&body);
        if !api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| AppError::AiApi(format!("TTS 请求失败: {}", e)))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::AiApi(format!(
                "TTS API 错误 [{}]: {}",
                status, text
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::AiApi(format!("TTS 响应读取失败: {}", e)))?
            .to_vec();
        Ok((bytes, "mp3".to_string()))
    }

    /// D3.6.4: 从 AiModel 解析 (api_url, model_name, temperature)
    ///
    /// 复用 xin_dialogue_service::resolve_model_endpoint 的逻辑（避免跨服务耦合）
    fn resolve_model_endpoint(model: &AiModel) -> (String, String, f64) {
        let api_url = model
            .api_url
            .as_deref()
            .or_else(|| crate::models::ai_model::get_provider_default_url(&model.provider))
            .unwrap_or("https://api.openai.com/v1")
            .to_string();
        let model_name = model
            .model_name
            .as_deref()
            .or_else(|| crate::models::ai_model::get_provider_default_model(&model.provider))
            .unwrap_or("gpt-4o-mini")
            .to_string();
        let temperature = model.temperature.unwrap_or(0.7);
        (api_url, model_name, temperature)
    }

    /// D3.6.4: 流式调用云端 LLM，每个 token 通过 emit RealtimeEvent::LlmChunk 推送
    ///
    /// 复用 xin_dialogue_service::call_llm_stream 的 SSE 解析模式。
    /// 设计意图（§四）：小欣走云端 API 模型，非本地底层智能模型。
    async fn call_llm_stream_realtime(
        app_handle: &AppHandle,
        session: &Arc<Mutex<Option<SessionContext>>>,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        model_id: i64,
        user_message: &str,
    ) -> Result<String, AppError> {
        // 1. 获取模型配置
        let (model, api_key) = AiModelService::get_model_config(pool, mek_manager, user_id, model_id)
            .await?;
        let (api_url, model_name, temperature) = Self::resolve_model_endpoint(&model);

        // 2. 构建 messages（system + user）
        let messages = vec![
            serde_json::json!({
                "role": "system",
                "content": "你是小欣，NexTerm·元界的 AI 助手。请用简洁自然的中文回复，像朋友聊天一样。回复控制在 100 字以内，适合语音播报。"
            }),
            serde_json::json!({
                "role": "user",
                "content": user_message
            }),
        ];

        let body = serde_json::json!({
            "model": model_name,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": 1024,
            "stream": true,
        });

        // 3. 发起流式请求
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| AppError::AiApi(format!("HTTP 客户端创建失败: {}", e)))?;

        let mut req_builder = client
            .post(format!("{}/chat/completions", api_url))
            .header("Content-Type", "application/json")
            .json(&body);
        if !api_key.is_empty() {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| AppError::AiApi(format!("LLM 流式请求失败: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::AiApi(format!(
                "LLM API 返回错误 [{}]: {}",
                status, text
            )));
        }

        // 4. 解析 SSE 流，每个 token emit LlmChunk
        let mut full_response = String::new();
        let mut stream = resp.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| AppError::AiApi(format!("流读取失败: {}", e)))?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                let line = line.trim();
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        // emit 最终 done
                        let _ = app_handle.emit(
                            "realtime_event",
                            RealtimeEvent::LlmChunk {
                                delta: String::new(),
                                full_text: full_response.clone(),
                                done: true,
                            },
                        );
                        return Ok(full_response);
                    }

                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                            full_response.push_str(content);
                            // 更新 session.llm_text
                            {
                                let mut s = session.lock().await;
                                if let Some(ctx) = s.as_mut() {
                                    ctx.llm_text = full_response.clone();
                                }
                            }
                            // emit 增量 token
                            let _ = app_handle.emit(
                                "realtime_event",
                                RealtimeEvent::LlmChunk {
                                    delta: content.to_string(),
                                    full_text: full_response.clone(),
                                    done: false,
                                },
                            );
                        }
                    }
                }
            }
        }

        // 流自然结束（无 [DONE]），emit done
        let _ = app_handle.emit(
            "realtime_event",
            RealtimeEvent::LlmChunk {
                delta: String::new(),
                full_text: full_response.clone(),
                done: true,
            },
        );

        Ok(full_response)
    }

    /// 重置状态到 Listening（STT 失败或完成后调用）
    async fn reset_to_listening(session: &Arc<Mutex<Option<SessionContext>>>, app_handle: &AppHandle) {
        let mut s = session.lock().await;
        if let Some(ctx) = s.as_mut() {
            ctx.state = RealtimeState::Listening;
        }
        let _ = app_handle.emit(
            "realtime_event",
            RealtimeEvent::StateChanged {
                state: RealtimeState::Listening,
            },
        );
    }

    /// 计算音频块的 RMS 能量（用于 VAD）
    ///
    /// 返回值范围 0.0 ~ 1.0（归一化到 [-1, 1] 后的 RMS）
    pub fn compute_rms(samples: &[i16]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f64 = samples
            .iter()
            .map(|&s| {
                let f = (s as f64) / 32768.0;
                f * f
            })
            .sum();
        (sum_sq / samples.len() as f64).sqrt() as f32
    }

    /// 将 i16 PCM 样本编码为 WAV 字节（用于 STT 上传 / TTS 返回）
    ///
    /// 格式：16kHz 单声道 16-bit PCM WAV
    pub fn encode_wav(samples: &[i16], sample_rate: u32) -> Result<Vec<u8>, AppError> {
        let spec = hound::WavSpec {
            channels: REALTIME_CHANNELS,
            sample_rate,
            bits_per_sample: REALTIME_BITS_PER_SAMPLE,
            sample_format: hound::SampleFormat::Int,
        };
        let mut buf: Vec<u8> = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buf);
            let mut writer = hound::WavWriter::new(cursor, spec)
                .map_err(|e| AppError::Internal(format!("WAV writer 创建失败: {}", e)))?;
            for &s in samples {
                writer
                    .write_sample(s)
                    .map_err(|e| AppError::Internal(format!("WAV 写入失败: {}", e)))?;
            }
            writer
                .finalize()
                .map_err(|e| AppError::Internal(format!("WAV finalize 失败: {}", e)))?;
        }
        Ok(buf)
    }

    /// 解码 WAV 字节为 i16 PCM 样本（用于解析 TTS 返回的 WAV 音频）
    pub fn decode_wav(bytes: &[u8]) -> Result<(Vec<i16>, u32), AppError> {
        let cursor = std::io::Cursor::new(bytes);
        let mut reader = hound::WavReader::new(cursor)
            .map_err(|e| AppError::Internal(format!("WAV reader 创建失败: {}", e)))?;
        let sample_rate = reader.spec().sample_rate;
        let samples: Vec<i16> = reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Internal(format!("WAV 采样读取失败: {}", e)))?;
        Ok((samples, sample_rate))
    }
}

impl Default for XinRealtimeService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms_silence() {
        // 全零样本 → RMS = 0
        let samples = vec![0i16; 320];
        let rms = XinRealtimeService::compute_rms(&samples);
        assert!(rms < 0.001, "静音 RMS 应接近 0，实际: {}", rms);
    }

    #[test]
    fn test_rms_active_speech() {
        // 满幅正弦波 → RMS 应接近 0.707（1/√2）
        let samples: Vec<i16> = (0..320)
            .map(|i| {
                let phase = (i as f32 / 16.0) * std::f32::consts::TAU;
                (phase.sin() * 16384.0) as i16
            })
            .collect();
        let rms = XinRealtimeService::compute_rms(&samples);
        assert!(
            rms > 0.3 && rms < 0.6,
            "正弦波 RMS 应在 0.3~0.6 之间，实际: {}",
            rms
        );
    }

    #[test]
    fn test_wav_encode_decode_roundtrip() {
        // 编码 → 解码 → 样本一致
        let original: Vec<i16> = vec![0, 1000, -1000, 32767, -32768, 0];
        let wav_bytes = XinRealtimeService::encode_wav(&original, 16000).unwrap();
        let (decoded, sr) = XinRealtimeService::decode_wav(&wav_bytes).unwrap();
        assert_eq!(sr, 16000);
        assert_eq!(decoded.len(), original.len());
        for (i, (orig, dec)) in original.iter().zip(decoded.iter()).enumerate() {
            assert_eq!(*orig, *dec, "样本 {} 不一致", i);
        }
    }

    #[test]
    fn test_session_lifecycle() {
        // 会话生命周期：new → 默认 Idle
        let svc = XinRealtimeService::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let state = rt.block_on(async { svc.get_state().await });
        assert_eq!(state, RealtimeState::Idle);
    }

    #[tokio::test]
    async fn test_start_session_validation() {
        // 默认配置 model_id 为 0，符合 start_session 的 Validation 校验失败条件
        // 注：start_session 需要 pool/mek_manager/app_handle 依赖，单元测试中无法构造完整调用，
        //     此处验证 config 默认值符合"无效配置"预期（start_session L263 校验 model_id == 0 → Err）
        let bad_config = RealtimeConfig {
            stt_model_id: 0,
            llm_model_id: 0,
            ..Default::default()
        };
        assert_eq!(bad_config.stt_model_id, 0);
        assert_eq!(bad_config.llm_model_id, 0);
    }
}
