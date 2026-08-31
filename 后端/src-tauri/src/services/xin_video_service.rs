//! D3.5 小欣视频输入多模态
//!
//! 设计依据：
//!   - 功能展望/模块深化/03_小欣_多模态融合_深度.md §2.4 视频输入
//!   - .trae/rules/项目核心设计意图.md §四（小欣走云端 API）
//!
//! 视频理解流程：
//!   1. 通过系统 ffmpeg 抽取关键帧（短视频每 2s/帧，长视频每 5s/帧，最多 20 帧）
//!   2. 关键帧转为 base64 图片
//!   3. 帧列表作为图片附件走云端多模态 API（GPT-4o 等）
//!   4. PoC 不提取音频，Phase 后续接入 STT
//!
//! 依赖：系统 ffmpeg（用户需自行安装并确保在 PATH 中）

use std::path::Path;
use std::process::Command;

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};

use crate::error::app_error::AppError;

/// 单个视频关键帧
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFrame {
    /// 帧在视频中的时间戳（秒）
    pub timestamp_secs: f64,
    /// 帧 JPEG base64 编码
    pub image_b64: String,
    pub mime_type: String,
}

/// 视频理解结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoUnderstandingResult {
    pub frames: Vec<VideoFrame>,
    pub frame_count: usize,
    pub duration_secs: Option<f64>,
    /// PoC 不提取音频，恒为 None
    pub audio_transcript: Option<String>,
}

/// D3.5 视频摘要结果（抽帧 + LLM 总结）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSummaryResult {
    /// LLM 生成的视频内容摘要
    pub summary: String,
    /// LLM 提取的关键要点
    pub key_points: Vec<String>,
    /// 抽取的关键帧（前端可作为附件二次询问）
    pub frames: Vec<VideoFrame>,
    pub frame_count: usize,
    pub duration_secs: Option<f64>,
    /// 使用的模型名称
    pub model_used: String,
}

pub struct XinVideoService;

const DEFAULT_MAX_FRAMES: usize = 10;
const MAX_FRAMES_CAP: usize = 20;
const SHORT_VIDEO_THRESHOLD_SECS: f64 = 60.0;
const SHORT_VIDEO_INTERVAL: f64 = 2.0;
const LONG_VIDEO_INTERVAL: f64 = 5.0;

impl XinVideoService {
    /// 检查系统是否安装 ffmpeg
    pub fn check_ffmpeg_available() -> bool {
        Command::new("ffmpeg")
            .arg("-version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// 获取视频时长（秒），失败返回 None
    fn get_duration(video_path: &str) -> Option<f64> {
        let output = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                video_path,
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let dur_str = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        dur_str.parse::<f64>().ok()
    }

    /// 抽取关键帧到指定目录，返回 (帧文件路径, 时间戳) 列表
    fn extract_key_frames(
        video_path: &str,
        output_dir: &str,
        max_frames: usize,
    ) -> Result<Vec<(String, f64)>, AppError> {
        std::fs::create_dir_all(output_dir).map_err(|e| AppError::FileSystem(e))?;

        let duration = Self::get_duration(video_path).unwrap_or(SHORT_VIDEO_THRESHOLD_SECS);
        let interval = if duration > SHORT_VIDEO_THRESHOLD_SECS {
            LONG_VIDEO_INTERVAL
        } else {
            SHORT_VIDEO_INTERVAL
        };
        let frame_count = ((duration / interval) as usize).clamp(1, max_frames);

        let sep = if output_dir.contains('\\') && !output_dir.contains('/') {
            '\\'
        } else {
            '/'
        };
        let frame_pattern = format!(
            "{}{}frame_%03d.jpg",
            output_dir.trim_end_matches(sep),
            sep
        );

        let output = Command::new("ffmpeg")
            .args([
                "-i",
                video_path,
                "-vf",
                &format!("fps=1/{}", interval),
                "-frames:v",
                &frame_count.to_string(),
                "-y",
                &frame_pattern,
            ])
            .output()
            .map_err(|e| AppError::Internal(format!("ffmpeg 执行失败: {}", e)))?;

        // ffmpeg 即使成功也可能在 stderr 输出信息，以实际生成的帧文件为准
        let mut frames: Vec<(String, f64)> = Vec::new();
        for entry in std::fs::read_dir(output_dir).map_err(|e| AppError::FileSystem(e))? {
            let entry = entry.map_err(|e| AppError::FileSystem(e))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("jpg") {
                let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                // frame_001 → idx=1 → 时间戳 = (1-1)*interval = 0
                let idx: f64 = name
                    .rsplit("frame_")
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let ts = if idx > 0.0 {
                    (idx - 1.0) * interval
                } else {
                    0.0
                };
                frames.push((path.to_string_lossy().to_string(), ts));
            }
        }
        frames.sort_by(|a, b| a.0.cmp(&b.0));

        if frames.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Internal(format!(
                "ffmpeg 抽帧失败（未生成帧）: {}",
                stderr
            )));
        }
        Ok(frames)
    }

    /// 完整视频理解流程：抽帧 → 转 base64 → 返回多模态帧
    pub fn analyze_video(
        video_path: &str,
        max_frames: Option<usize>,
    ) -> Result<VideoUnderstandingResult, AppError> {
        if !Path::new(video_path).exists() {
            return Err(AppError::Validation(format!(
                "视频文件不存在: {}",
                video_path
            )));
        }
        if !Self::check_ffmpeg_available() {
            return Err(AppError::Internal(
                "视频理解需要 ffmpeg，请先安装 ffmpeg 并确保在 PATH 中".into(),
            ));
        }

        let max_frames = max_frames.unwrap_or(DEFAULT_MAX_FRAMES).min(MAX_FRAMES_CAP);
        let temp_dir = std::env::temp_dir().join(format!(
            "nexterm_video_{}",
            chrono::Utc::now().timestamp_millis()
        ));
        let temp_dir_str = temp_dir.to_string_lossy().to_string();

        let duration = Self::get_duration(video_path);
        let frame_paths = Self::extract_key_frames(video_path, &temp_dir_str, max_frames)?;

        let mut frames = Vec::new();
        for (path, ts) in &frame_paths {
            let bytes = std::fs::read(path).map_err(|e| AppError::FileSystem(e))?;
            let b64 = STANDARD.encode(&bytes);
            frames.push(VideoFrame {
                timestamp_secs: *ts,
                image_b64: b64,
                mime_type: "image/jpeg".into(),
            });
        }

        // 清理临时文件
        let _ = std::fs::remove_dir_all(&temp_dir);

        let frame_count = frames.len();
        tracing::info!(
            "[D3.5] 视频分析完成: {} 帧, 时长 {:?}s",
            frame_count,
            duration
        );

        Ok(VideoUnderstandingResult {
            frames,
            frame_count,
            duration_secs: duration,
            audio_transcript: None,
        })
    }

    /// D3.5 视频摘要：抽帧 → 调用云端多模态 LLM 生成文字摘要
    ///
    /// 设计意图（§四）：小欣走云端 API 多模态模型（GPT-4o 等），非本地底层智能模型。
    /// 流程：
    ///   1. 调用 analyze_video 抽取关键帧（base64）
    ///   2. 将帧作为图片附件构建多模态 OpenAI 兼容请求
    ///   3. 调用云端 LLM 生成视频内容摘要 + 关键要点
    ///   4. 返回摘要 + 帧（帧可供前端二次询问）
    pub async fn summarize_video(
        video_path: &str,
        max_frames: Option<usize>,
        api_url: &str,
        api_key: &str,
        model_name: &str,
    ) -> Result<VideoSummaryResult, AppError> {
        // 步骤 1：抽帧（复用 analyze_video 逻辑）
        let analysis = Self::analyze_video(video_path, max_frames)?;

        if analysis.frames.is_empty() {
            return Err(AppError::Internal("视频抽帧失败：未获取到任何帧".into()));
        }

        // 步骤 2：构建多模态消息（系统提示 + 用户消息含图片）
        let system_prompt = "你是一个专业的视频内容分析助手。请根据用户提供的视频关键帧，生成视频内容摘要。\n\
            \n输出格式要求（严格遵守）：\n\
            第一行：直接写摘要正文（100-300字），不要加【摘要】等标题前缀\n\
            接下来：用 - 开头列出3个关键要点，每行一个";

        let mut content_parts: Vec<serde_json::Value> = Vec::new();
        content_parts.push(serde_json::json!({
            "type": "text",
            "text": format!("请根据以下 {} 个视频关键帧，总结视频内容。视频时长约 {:.1} 秒。",
                analysis.frame_count,
                analysis.duration_secs.unwrap_or(0.0))
        }));

        // 限制发送给 LLM 的帧数（避免 token 超限）
        let max_llm_frames = analysis.frames.len().min(8);
        for frame in analysis.frames.iter().take(max_llm_frames) {
            content_parts.push(serde_json::json!({
                "type": "image_url",
                "image_url": {
                    "url": format!("data:{};base64,{}", frame.mime_type, frame.image_b64),
                    "detail": "low"
                }
            }));
        }

        let body = serde_json::json!({
            "model": model_name,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": content_parts }
            ],
            "temperature": 0.3,
            "max_tokens": 1024,
            "stream": false
        });

        // 步骤 3：调用云端 LLM
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| AppError::AiApi(format!("HTTP 客户端创建失败: {}", e)))?;

        let url = format!("{}/chat/completions", api_url.trim_end_matches('/'));
        let mut req_builder = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body);
        if !api_key.is_empty() {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| AppError::AiApi(format!("视频摘要 API 请求失败: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::AiApi(format!(
                "视频摘要 API 返回错误 [{}]: {}",
                status, text
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::AiApi(format!("视频摘要响应解析失败: {}", e)))?;

        let result_text = json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::AiApi("视频摘要响应格式异常".into()))?;

        // 步骤 4：解析摘要 + 关键要点
        let (summary, key_points) = Self::parse_summary_response(&result_text);

        tracing::info!(
            "[D3.5] 视频摘要生成完成: {} 帧, 摘要 {} 字",
            analysis.frame_count,
            summary.chars().count()
        );

        Ok(VideoSummaryResult {
            summary,
            key_points,
            frames: analysis.frames,
            frame_count: analysis.frame_count,
            duration_secs: analysis.duration_secs,
            model_used: model_name.to_string(),
        })
    }

    /// 解析 LLM 返回的摘要文本，分离摘要正文和关键要点
    fn parse_summary_response(text: &str) -> (String, Vec<String>) {
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        let mut content_lines: Vec<String> = Vec::new();
        let mut point_lines: Vec<String> = Vec::new();

        for line in &lines {
            let trimmed = line.trim();
            let is_point = trimmed.starts_with('-')
                || trimmed.starts_with('·')
                || trimmed.starts_with('*')
                || trimmed.starts_with("要点")
                || trimmed.starts_with("关键")
                || trimmed.starts_with("1.")
                || trimmed.starts_with("2.")
                || trimmed.starts_with("3.");

            if is_point {
                let cleaned = trimmed
                    .trim_matches(&['-', '·', '*', ' ', '1', '2', '3', '.'] as &[_])
                    .trim()
                    .to_string();
                if !cleaned.is_empty() {
                    point_lines.push(cleaned);
                }
            } else {
                // 跳过标题行
                if trimmed.starts_with("【") && trimmed.ends_with("】") {
                    continue;
                }
                if trimmed == "摘要" || trimmed == "总结" || trimmed == "Summary" {
                    continue;
                }
                content_lines.push(trimmed.to_string());
            }
        }

        let summary = if content_lines.is_empty() {
            text.chars().take(300).collect()
        } else {
            content_lines.join("\n")
        };

        let key_points = if point_lines.is_empty() {
            // 兜底：从摘要拆分句子
            let sentences: Vec<&str> = summary
                .split(['。', '！', '？', '\n'])
                .filter(|s| s.trim().len() > 6)
                .take(3)
                .collect();
            if sentences.len() >= 2 {
                sentences.iter().map(|s| s.trim().to_string()).collect()
            } else {
                vec!["AI 已完成视频摘要".to_string()]
            }
        } else {
            point_lines.into_iter().take(3).collect()
        };

        (summary, key_points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_summary_response_with_points() {
        let text = "这是一个关于编程教学的视频。\n- 介绍了 Rust 语言基础\n- 演示了所有权机制\n- 展示了借用检查器";
        let (summary, points) = XinVideoService::parse_summary_response(text);
        assert!(summary.contains("编程教学"));
        assert_eq!(points.len(), 3);
        assert!(points[0].contains("Rust"));
    }

    #[test]
    fn test_parse_summary_response_no_points() {
        let text = "视频展示了一个完整的操作流程。用户从开始到结束完成了所有步骤。";
        let (summary, points) = XinVideoService::parse_summary_response(text);
        assert!(!summary.is_empty());
        // 无要点行时从摘要拆分
        assert!(!points.is_empty());
    }

    #[test]
    fn test_parse_summary_response_with_title() {
        let text = "【摘要】\n视频内容是关于AI的介绍\n- AI 的定义\n- AI 的应用";
        let (summary, points) = XinVideoService::parse_summary_response(text);
        assert!(summary.contains("AI"));
        assert!(!summary.contains("【摘要】"));
        assert_eq!(points.len(), 2);
    }
}
