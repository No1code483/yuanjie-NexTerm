//! D3.7 小欣情感系统服务
//!
//! 设计依据：
//!   - v2_下一阶段开发计划：D3.7 验收标准「按用户情绪调整风格」
//!   - .trae/rules/项目核心设计意图.md §四（小欣走云端 API，情感风格调整通过 system prompt 注入）
//!   - 功能展望/模块深化/03_小欣_多模态融合_深度.md（情感系统规划）
//!
//! 核心功能：
//!   1. 用户情绪识别（复用 XinPostProcessor::analyze_sentiment 关键词匹配）
//!   2. 情绪→回复风格映射（get_style_guide）
//!   3. 情绪感知入口（get_emotion_guide：分析消息→生成风格指引）
//!   4. 小欣情绪共鸣（empathic_mood：用户情绪→小欣自身情绪）
//!
//! 设计原则：
//!   - 非侵入：情绪风格作为 system prompt 的可选注入，关闭后不影响对话
//!   - 简洁优先：复用现有 analyze_sentiment，不重复造轮子
//!   - 可关闭：情绪强度低于阈值时不注入风格指引（保持自然）

use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::xin::{EmotionCategory, SentimentResult};
use crate::services::xin_post_process_service::XinPostProcessor;

/// 情绪强度阈值：低于此值不注入风格指引（避免过度干预自然对话）
const EMOTION_GUIDE_THRESHOLD: f64 = 0.4;

/// 情绪历史趋势数据点
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmotionTrendPoint {
    /// 情绪类别（happy/sad/angry/...）
    pub category: String,
    /// 情绪强度（0.0 ~ 1.0）
    pub intensity: f64,
    /// 记录时间
    pub updated_at: String,
    /// 触发文本（截断）
    pub trigger: Option<String>,
}

/// D3.7 小欣情感系统服务
///
/// 纯逻辑服务（无状态），核心职责是「根据用户情绪生成回复风格指引」，
/// 供 XinPromptBuilder 注入到 system prompt 中，实现「按用户情绪调整风格」。
pub struct XinEmotionService;

impl XinEmotionService {
    /// 情绪感知入口：分析用户消息 → 生成回复风格指引
    ///
    /// 返回 None 的场景：
    ///   - 情绪强度低于 EMOTION_GUIDE_THRESHOLD（自然对话，无需特殊风格）
    ///   - 主导情绪为 Calm（平静状态，不需要调整）
    ///
    /// 调用点：xin_dialogue_service.rs::send_message 构建 system prompt 前
    pub fn get_emotion_guide(text: &str) -> Option<String> {
        let sentiment = XinPostProcessor::analyze_sentiment(text);

        // 取情绪强度最高的一项作为主导情绪
        let top_emotion = sentiment
            .emotions
            .iter()
            .max_by(|a, b| {
                a.intensity
                    .partial_cmp(&b.intensity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })?;

        // 低强度情绪不注入风格指引，保持自然对话
        if top_emotion.intensity < EMOTION_GUIDE_THRESHOLD {
            return None;
        }

        Self::get_style_guide(&top_emotion.category, top_emotion.intensity)
    }

    /// 情绪→回复风格指引映射（核心）
    ///
    /// 根据用户当前情绪类别，返回对应的回复风格指引文本。
    /// 该文本将被注入 system prompt，引导云端 LLM 以合适的风格回复。
    pub fn get_style_guide(emotion: &EmotionCategory, intensity: f64) -> Option<String> {
        let guide: &str = match emotion {
            EmotionCategory::Happy => {
                "用户心情愉悦。请以活泼热情的语气回应，可以分享喜悦，\
                 适当使用积极词汇，保持轻快节奏。"
            }
            EmotionCategory::Sad => {
                "用户情绪低落。请以温暖共情的语气回应，给予安慰和支持，\
                 语速放缓，避免过于欢快或说教。"
            }
            EmotionCategory::Angry => {
                "用户情绪激动。请以冷静理性的语气回应，先认可其情绪，\
                 避免争辩或刺激，帮助平复后再解决问题。"
            }
            EmotionCategory::Anxious => {
                "用户感到焦虑。请以温柔安抚的语气回应，给予安全感，\
                 用确定的语气，避免增加压力的信息。"
            }
            EmotionCategory::Curious => {
                "用户充满好奇。请详细解答并引导探索，\
                 可以扩展相关知识，鼓励提问。"
            }
            EmotionCategory::Confused => {
                "用户感到困惑。请耐心清晰地分步解释，\
                 避免信息过载，用简单类比帮助理解。"
            }
            EmotionCategory::Surprised => {
                "用户感到惊讶。请以同理心回应，肯定其感受，\
                 提供更多背景信息。"
            }
            EmotionCategory::Calm => {
                // 平静状态不需要特殊风格指引
                return None;
            }
        };

        // 强情绪标注（强度 > 0.7 时提示 LLM 加重风格）
        let intensity_tag = if intensity > 0.7 {
            "（用户情绪强烈，请加重风格调整）"
        } else {
            ""
        };

        Some(format!("【用户情绪感知{}】{}", intensity_tag, guide))
    }

    /// 小欣情绪共鸣：根据用户情绪调整小欣自身情绪
    ///
    /// 设计：小欣不是简单镜像用户情绪，而是以「共情 + 适当互补」回应：
    ///   - 用户开心 → 小欣也开心（共情）
    ///   - 用户悲伤 → 小欣共情体谅（empathetic）
    ///   - 用户愤怒/焦虑 → 小欣冷静安抚（calm，互补）
    ///   - 用户好奇 → 小欣好奇配合（curious）
    pub fn empathic_mood(user_emotion: &EmotionCategory) -> &'static str {
        match user_emotion {
            EmotionCategory::Happy => "happy",
            EmotionCategory::Sad => "empathetic",
            EmotionCategory::Angry => "calm",
            EmotionCategory::Anxious => "calm",
            EmotionCategory::Curious => "curious",
            EmotionCategory::Confused => "patient",
            EmotionCategory::Surprised => "surprised",
            EmotionCategory::Calm => "calm",
        }
    }

    /// 情绪历史趋势分析（基于 xin_moods 表）
    ///
    /// 查询最近的情绪记录，用于前端情绪时间线展示。
    /// 多用户隔离：仅查询当前用户的情绪记录。
    pub async fn get_emotion_trend(
        pool: &SqlitePool,
        user_id: i64,
        limit: i64,
    ) -> Result<Vec<EmotionTrendPoint>, AppError> {
        let rows = sqlx::query_as::<
            _,
            (String, f64, String, Option<String>),
        >(
            "SELECT category, intensity, updated_at, trigger_text \
             FROM xin_moods WHERE user_id = ? ORDER BY updated_at DESC LIMIT ?",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows
            .into_iter()
            .map(|(category, intensity, updated_at, trigger)| EmotionTrendPoint {
                category,
                intensity,
                updated_at,
                trigger,
            })
            .collect())
    }

    /// 情绪分析（对外暴露，复用 XinPostProcessor）
    ///
    /// 供 IPC 命令调用，前端可查询某段文本的情绪分析结果。
    pub fn analyze_emotion(text: &str) -> SentimentResult {
        XinPostProcessor::analyze_sentiment(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_happy_emotion_guide() {
        let guide = XinEmotionService::get_emotion_guide("太开心了！今天真的很棒，非常高兴！");
        assert!(guide.is_some(), "开心消息应生成风格指引");
        let g = guide.unwrap();
        assert!(g.contains("活泼热情"), "开心风格应包含活泼热情");
    }

    #[test]
    fn test_sad_emotion_guide() {
        let guide = XinEmotionService::get_emotion_guide("太难过了，我很伤心，感觉很难受");
        assert!(guide.is_some());
        assert!(guide.unwrap().contains("温暖共情"));
    }

    #[test]
    fn test_calm_no_guide() {
        // 平静消息不应生成风格指引
        let guide = XinEmotionService::get_emotion_guide("好的");
        assert!(guide.is_none(), "平静消息不应生成风格指引");
    }

    #[test]
    fn test_style_guide_calm_returns_none() {
        let guide =
            XinEmotionService::get_style_guide(&EmotionCategory::Calm, 0.8);
        assert!(guide.is_none(), "Calm 情绪应返回 None");
    }

    #[test]
    fn test_empathic_mood_mapping() {
        assert_eq!(XinEmotionService::empathic_mood(&EmotionCategory::Happy), "happy");
        assert_eq!(XinEmotionService::empathic_mood(&EmotionCategory::Angry), "calm");
        assert_eq!(XinEmotionService::empathic_mood(&EmotionCategory::Sad), "empathetic");
    }

    #[test]
    fn test_strong_emotion_intensity_tag() {
        let guide =
            XinEmotionService::get_style_guide(&EmotionCategory::Angry, 0.9);
        assert!(guide.is_some());
        assert!(guide.unwrap().contains("情绪强烈"), "高强度情绪应标注强烈");
    }

    #[test]
    fn test_analyze_emotion_returns_result() {
        let result = XinEmotionService::analyze_emotion("我很开心");
        assert!(!result.emotions.is_empty());
    }
}
