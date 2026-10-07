//! 小欣 TTS 服务（D3.2）
//!
//! 设计目标（03_小欣_多模态融合_深度.md §2.2）：
//! - 将小欣回复合成语音播放
//! - 支持语速 / 音调 / 音色配置
//! - 跨平台抽象（TtsEngine trait）
//!
//! 实现策略（2026-07-21 D3.2）：
//! - Windows：通过 PowerShell 调用 SAPI.SPVoice 合成 WAV 文件
//! - macOS/Linux：返回日志模式占位（待后续阶段接入 say / espeak）
//! - 不引入重型 Rust TTS crate（如 kokoro-rs / sherpa-onnx），保持编译时间可控
//! - 后续阶段可扩展 Edge TTS（在线神经 TTS）作为高质量备选

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::error::app_error::AppError;

/// TTS 引擎抽象。
///
/// 不同平台 / 实现需实现此 trait。
/// 失败时返回 `AppError`，调用方决定是否降级。
pub trait TtsEngine: Send + Sync {
    /// 引擎名称（如 "windows_sapi" / "macos_say" / "edge_tts"）
    fn name(&self) -> &str;

    /// 列出可用音色（用于 UI 选择）
    fn list_voices(&self) -> Vec<String>;

    /// 合成语音到文件，返回 WAV/MP3 文件路径。
    ///
    /// `text` 待合成文本；`voice` 音色名（None 用默认）；`speed` 语速倍率（0.5~2.0）；`pitch` 音调倍率（0.5~2.0）。
    fn synthesize(
        &self,
        text: &str,
        voice: Option<&str>,
        speed: f64,
        pitch: f64,
        out_path: &std::path::Path,
    ) -> Result<(), AppError>;
}

/// Windows SAPI 实现（通过 PowerShell 调用）。
///
/// 利用系统自带的 SAPI.SpVoice COM 对象，无需引入额外 Rust 依赖。
/// 输出格式：PCM 22kHz 16bit 单声道 WAV。
pub struct WindowsSapiEngine;

impl WindowsSapiEngine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsSapiEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TtsEngine for WindowsSapiEngine {
    fn name(&self) -> &str {
        "windows_sapi"
    }

    fn list_voices(&self) -> Vec<String> {
        // 通过 PowerShell 列出已安装的 SAPI 音色
        // 失败时返回默认占位，不阻塞流程
        let ps_script = "(New-Object -ComObject SAPI.SpVoice).GetVoices() | ForEach-Object { $_.GetDescription() }";
        match run_powershell(ps_script) {
            Ok(out) => out
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            Err(_) => vec!["Microsoft Huihui Desktop".into(), "Microsoft Zira Desktop".into()],
        }
    }

    fn synthesize(
        &self,
        text: &str,
        voice: Option<&str>,
        speed: f64,
        pitch: f64,
        out_path: &std::path::Path,
    ) -> Result<(), AppError> {
        // 转义 PowerShell 字符串中的特殊字符
        let escaped_text = text
            .replace('\'', "''")
            .replace('\r', " ")
            .replace('\n', " ");
        let escaped_path = out_path.to_string_lossy().replace('\'', "''");

        // SAPI 语速 Rate 范围 -10 ~ +10（0 为正常）；speed 倍率映射
        let rate = ((speed - 1.0) * 10.0).round() as i32;
        // SAPI 音调在 SetVoice 之外需要 XML 标记，此处暂不实现 pitch（仅 speed + voice）
        let _ = pitch; // 暂未使用，保留参数兼容

        let voice_clause = match voice {
            Some(v) if !v.is_empty() => {
                let escaped_voice = v.replace('\'', "''");
                format!(
                    "$voice = $sp.GetVoices().Item(0); foreach ($v in $sp.GetVoices()) {{ if ($v.GetDescription() -like '*{escaped_voice}*') {{ $voice = $v; break }} }}; $sp.Voice = $voice;"
                )
            }
            _ => String::new(),
        };

        let ps_script = format!(
            "$sp = New-Object -ComObject SAPI.SpVoice;\
             {voice_clause}\
             $sp.Rate = {rate};\
             $fs = New-Object -ComObject SAPI.SpFileStream;\
             $fs.Open('{escaped_path}', 3, $false);\
             $sp.AudioOutputStream = $fs;\
             $sp.Speak('{escaped_text}', 8);\
             $fs.Close();",
            voice_clause = voice_clause,
            rate = rate,
            escaped_path = escaped_path,
            escaped_text = escaped_text
        );

        run_powershell(&ps_script).map_err(|e| {
            AppError::Internal(format!("Windows SAPI 合成失败: {}", e))
        })?;
        Ok(())
    }
}

/// macOS `say` 命令实现。
pub struct MacosSayEngine;

impl TtsEngine for MacosSayEngine {
    fn name(&self) -> &str {
        "macos_say"
    }

    fn list_voices(&self) -> Vec<String> {
        std::process::Command::new("say")
            .arg("-v")
            .arg("?")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| {
                s.lines()
                    .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(|| vec!["Tingting".into(), "Sinji".into()])
    }

    fn synthesize(
        &self,
        text: &str,
        voice: Option<&str>,
        speed: f64,
        pitch: f64,
        out_path: &std::path::Path,
    ) -> Result<(), AppError> {
        let mut cmd = std::process::Command::new("say");
        if let Some(v) = voice {
            cmd.arg("-v").arg(v);
        }
        // say 语速 -r 单位为字/分钟（默认 175），speed 倍率
        cmd.arg("-r").arg(format!("{}", (175.0 * speed) as u32));
        // say 没有直接音调参数，pitch 通过 [[pbas 0]] embedded command 实现，此处保留兼容
        let _ = pitch;
        cmd.arg("-o")
            .arg(out_path)
            .arg("--data-format=LEF32@22050")
            .arg("--file-format=WAVE")
            .arg(text);
        cmd.status()
            .map_err(|e| AppError::Internal(format!("macOS say 调用失败: {}", e)))?;
        Ok(())
    }
}

/// TTS 服务入口：按平台选择引擎，提供 synthesize_to_temp_file 便捷接口。
pub struct XinTtsService {
    engine: Box<dyn TtsEngine>,
    cache_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsSynthesizeRequest {
    pub text: String,
    pub voice: Option<String>,
    pub speed: Option<f64>,
    pub pitch: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsSynthesizeResponse {
    pub audio_path: String,
    pub engine: String,
    pub voices: Vec<String>,
}

impl XinTtsService {
    pub fn new(cache_dir: PathBuf) -> Self {
        let engine: Box<dyn TtsEngine> = if cfg!(target_os = "windows") {
            Box::new(WindowsSapiEngine::new())
        } else if cfg!(target_os = "macos") {
            Box::new(MacosSayEngine)
        } else {
            // Linux / 其他平台：占位日志引擎
            Box::new(NullTtsEngine)
        };
        Self { engine, cache_dir }
    }

    pub fn engine_name(&self) -> &str {
        self.engine.name()
    }

    pub fn list_voices(&self) -> Vec<String> {
        self.engine.list_voices()
    }

    /// 合成语音到临时文件，返回文件路径。
    pub fn synthesize(&self, request: TtsSynthesizeRequest) -> Result<TtsSynthesizeResponse, AppError> {
        if request.text.trim().is_empty() {
            return Err(AppError::Validation("TTS 文本不能为空".into()));
        }

        // 确保缓存目录存在
        std::fs::create_dir_all(&self.cache_dir).map_err(|e| {
            AppError::Internal(format!("创建 TTS 缓存目录失败: {}", e))
        })?;

        // 用文本哈希作为文件名，相同文本复用缓存
        let mut hasher = sha2::Sha256::new();
        hasher.update(request.text.as_bytes());
        if let Some(v) = &request.voice {
            hasher.update(v.as_bytes());
        }
        let speed = request.speed.unwrap_or(1.0);
        let pitch = request.pitch.unwrap_or(1.0);
        hasher.update(&[speed.to_be_bytes(), pitch.to_be_bytes()].concat());
        let hash = hasher.finalize();
        let hash_hex: String = hash.iter().take(8).map(|b| format!("{:02x}", b)).collect();

        let out_path = self.cache_dir.join(format!("tts_{}.wav", hash_hex));

        // 缓存命中：直接返回
        if out_path.exists() {
            return Ok(TtsSynthesizeResponse {
                audio_path: out_path.to_string_lossy().to_string(),
                engine: self.engine.name().to_string(),
                voices: self.list_voices(),
            });
        }

        self.engine.synthesize(
            &request.text,
            request.voice.as_deref(),
            speed,
            pitch,
            &out_path,
        )?;

        Ok(TtsSynthesizeResponse {
            audio_path: out_path.to_string_lossy().to_string(),
            engine: self.engine.name().to_string(),
            voices: self.list_voices(),
        })
    }
}

/// 空实现（Linux / 不支持平台）— 仅记录日志，不生成实际音频。
struct NullTtsEngine;

impl TtsEngine for NullTtsEngine {
    fn name(&self) -> &str {
        "null"
    }

    fn list_voices(&self) -> Vec<String> {
        vec!["default".into()]
    }

    fn synthesize(
        &self,
        text: &str,
        _voice: Option<&str>,
        _speed: f64,
        _pitch: f64,
        _out_path: &std::path::Path,
    ) -> Result<(), AppError> {
        tracing::info!(text = %text, "NullTtsEngine - 当前平台未实现 TTS，仅记录日志");
        Err(AppError::Internal("当前平台未实现 TTS".into()))
    }
}

/// 执行 PowerShell 脚本，返回 stdout。
fn run_powershell(script: &str) -> Result<String, String> {
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|e| format!("PowerShell 启动失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("PowerShell 执行失败: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let svc = XinTtsService::new(PathBuf::from("/tmp/tts_cache"));
        // 引擎名不应为空
        assert!(!svc.engine_name().is_empty());
        // 音色列表至少有 1 个
        assert!(!svc.list_voices().is_empty());
    }

    #[test]
    fn test_synthesize_empty_text_rejected() {
        let svc = XinTtsService::new(PathBuf::from("/tmp/tts_cache"));
        let req = TtsSynthesizeRequest {
            text: "   ".into(),
            voice: None,
            speed: None,
            pitch: None,
        };
        let result = svc.synthesize(req);
        assert!(result.is_err());
    }
}
