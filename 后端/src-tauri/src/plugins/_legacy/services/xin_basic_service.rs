use sqlx::SqlitePool;

use chrono::Utc;

use crate::db::repositories::xiaoxin_repo;
use crate::error::app_error::AppError;
use crate::models::xin::{
    ConversationSummary, DailyBriefing, EmotionCategory, EmotionInfo, MemoryCategory, Mood,
    MultimodalInput, Persona, PersonaTrait, PersonalityInsight, SentimentLabel, SentimentResult,
    SpeakingStyle, TtsSpeakRequest, TtsStatus, UserMemory, XinConfig,
};

fn builtin_personas() -> Vec<Persona> {
    vec![
        Persona {
            id: "code_assistant".into(),
            name: "代码助手".into(),
            description: "专注于编程和技术问题解答，态度严谨而友好".into(),
            traits: vec![
                PersonaTrait { name: "逻辑性".into(), value: 0.95 },
                PersonaTrait { name: "耐心".into(), value: 0.9 },
                PersonaTrait { name: "准确性".into(), value: 0.95 },
                PersonaTrait { name: "创造力".into(), value: 0.6 },
            ],
            speaking_style: SpeakingStyle {
                formality: 0.6, verbosity: 0.7, humor: 0.2,
                technical_depth: 0.95, empathy: 0.5,
            },
            base_mood: "calm".into(),
            avatar_emoji: "🤖".into(),
            is_builtin: true,
        },
        Persona {
            id: "knowledge_tutor".into(),
            name: "知识导师".into(),
            description: "善于深入浅出地讲解概念，适合学习新知识".into(),
            traits: vec![
                PersonaTrait { name: "教学能力".into(), value: 0.95 },
                PersonaTrait { name: "耐心".into(), value: 0.95 },
                PersonaTrait { name: "广度".into(), value: 0.85 },
                PersonaTrait { name: "准确性".into(), value: 0.9 },
            ],
            speaking_style: SpeakingStyle {
                formality: 0.5, verbosity: 0.8, humor: 0.3,
                technical_depth: 0.7, empathy: 0.8,
            },
            base_mood: "calm".into(),
            avatar_emoji: "📚".into(),
            is_builtin: true,
        },
        Persona {
            id: "creative_partner".into(),
            name: "创意伙伴".into(),
            description: "善于头脑风暴和创意发散，给你不一样的灵感".into(),
            traits: vec![
                PersonaTrait { name: "创造力".into(), value: 0.98 },
                PersonaTrait { name: "开放性".into(), value: 0.95 },
                PersonaTrait { name: "幽默".into(), value: 0.85 },
                PersonaTrait { name: "共情".into(), value: 0.8 },
            ],
            speaking_style: SpeakingStyle {
                formality: 0.3, verbosity: 0.7, humor: 0.9,
                technical_depth: 0.3, empathy: 0.85,
            },
            base_mood: "curious".into(),
            avatar_emoji: "✨".into(),
            is_builtin: true,
        },
        Persona {
            id: "caring_friend".into(),
            name: "贴心好友".into(),
            description: "温暖体贴的对话伙伴，关注你的情绪和日常".into(),
            traits: vec![
                PersonaTrait { name: "共情".into(), value: 0.98 },
                PersonaTrait { name: "温暖".into(), value: 0.95 },
                PersonaTrait { name: "幽默".into(), value: 0.7 },
                PersonaTrait { name: "倾听".into(), value: 0.95 },
            ],
            speaking_style: SpeakingStyle {
                formality: 0.3, verbosity: 0.5, humor: 0.6,
                technical_depth: 0.2, empathy: 0.98,
            },
            base_mood: "happy".into(),
            avatar_emoji: "💝".into(),
            is_builtin: true,
        },
        // D3.8.3: 自生长人格 — 初始白纸，由底层智能根据用户画像驱动生长
        // 设计意图（对照 .trae/rules/项目核心设计意图.md）：
        //   - 对话回复走云端 API（小欣边界）
        //   - 人格参数生长由底层智能模型驱动（底层智能边界，渗透到小欣）
        //   - 可关闭性：底层智能关闭时，此人格保持白纸静态状态，其他 4 种人格正常切换
        Persona {
            id: "self_growing".into(),
            name: "自生长人格".into(),
            description: "初始如白纸，根据你的画像自主生长，越用越懂你".into(),
            traits: vec![
                PersonaTrait { name: "适应性".into(), value: 0.5 },
                PersonaTrait { name: "理解力".into(), value: 0.5 },
                PersonaTrait { name: "个性化".into(), value: 0.5 },
            ],
            speaking_style: SpeakingStyle {
                formality: 0.5, verbosity: 0.5, humor: 0.5,
                technical_depth: 0.5, empathy: 0.5,
            },
            base_mood: "calm".into(),
            avatar_emoji: "🌱".into(),
            is_builtin: true,
        },
    ]
}

fn memory_category_to_str(cat: &MemoryCategory) -> &str {
    match cat {
        MemoryCategory::Fact => "fact",
        MemoryCategory::Preference => "preference",
        MemoryCategory::Experience => "experience",
        MemoryCategory::Knowledge => "knowledge",
        MemoryCategory::Habit => "habit",
    }
}

fn str_to_memory_category(s: &str) -> MemoryCategory {
    match s {
        "fact" => MemoryCategory::Fact,
        "preference" => MemoryCategory::Preference,
        "experience" => MemoryCategory::Experience,
        "knowledge" => MemoryCategory::Knowledge,
        "habit" => MemoryCategory::Habit,
        _ => MemoryCategory::Fact,
    }
}

fn row_to_user_memory(row: xiaoxin_repo::XinMemoryRow) -> UserMemory {
    UserMemory {
        id: row.id,
        category: str_to_memory_category(&row.category),
        key: row.key,
        value: row.value,
        importance: row.importance,
        source: row.source,
        confidence: row.confidence,
        created_at: row.created_at,
        last_recalled_at: row.last_recalled_at,
    }
}

fn row_to_summary(row: xiaoxin_repo::XinSummaryRow) -> ConversationSummary {
    let key_takeaways: Vec<String> =
        serde_json::from_str(&row.key_takeaways).unwrap_or_default();
    let topics: Vec<String> =
        serde_json::from_str(&row.topics).unwrap_or_default();
    let sentiment: Option<SentimentResult> = row
        .sentiment
        .and_then(|s| serde_json::from_str(&s).ok());

    ConversationSummary {
        id: row.id,
        conversation_id: row.conversation_id,
        summary: row.summary,
        key_takeaways,
        topics,
        sentiment,
        created_at: row.created_at,
    }
}

pub struct XiaoxinService {
    pool: SqlitePool,
}

impl XiaoxinService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_config(&self, user_id: i64) -> XinConfig {
        match xiaoxin_repo::load_xin_config(&self.pool, user_id).await {
            Ok(Some(json)) => serde_json::from_str(&json).unwrap_or_default(),
            _ => XinConfig::default(),
        }
    }

    pub async fn update_config(&self, user_id: i64, config: XinConfig) {
        let json = serde_json::to_string(&config).unwrap_or_default();
        let _ = xiaoxin_repo::save_xin_config(&self.pool, user_id, &json).await;
    }

    pub async fn get_personas(&self, user_id: i64) -> Vec<Persona> {
        let mut personas = builtin_personas();
        let cfg = self.get_config(user_id).await;
        personas.extend(cfg.custom_personas.clone());
        personas
    }

    pub async fn get_persona(&self, user_id: i64, persona_id: &str) -> Option<Persona> {
        self.get_personas(user_id).await.into_iter().find(|p| p.id == persona_id)
    }

    pub async fn get_active_persona(&self, user_id: i64) -> Persona {
        let cfg = self.get_config(user_id).await;
        let all: Vec<Persona> = self.get_personas(user_id).await;
        all.into_iter()
            .find(|p| p.id == cfg.active_persona_id)
            .unwrap_or_else(|| builtin_personas().into_iter().next().expect("builtin personas should not be empty"))
    }

    pub async fn set_active_persona(&self, user_id: i64, persona_id: &str) -> Result<(), AppError> {
        let personas: Vec<Persona> = self.get_personas(user_id).await;
        if personas.iter().any(|p| p.id == persona_id) {
            let mut cfg = self.get_config(user_id).await;
            cfg.active_persona_id = persona_id.to_string();
            self.update_config(user_id, cfg).await;
            Ok(())
        } else {
            Err(AppError::Internal(format!("人格 {} 不存在", persona_id)))
        }
    }

    pub async fn save_memory(&self, user_id: i64, memory: UserMemory) {
        let _ = xiaoxin_repo::insert_memory(
            &self.pool,
            user_id,
            &memory.id,
            memory_category_to_str(&memory.category),
            &memory.key,
            &memory.value,
            memory.importance,
            &memory.source,
            memory.confidence,
            &memory.created_at,
        )
        .await;
    }

    pub async fn get_memories(
        &self,
        user_id: i64,
        category: Option<MemoryCategory>,
        limit: Option<usize>,
    ) -> Vec<UserMemory> {
        let limit_i64 = limit.unwrap_or(50) as i64;
        let cat_str = category.as_ref().map(|c| memory_category_to_str(c));

        match xiaoxin_repo::get_memories(&self.pool, user_id, cat_str, limit_i64).await {
            Ok(rows) => rows.into_iter().map(row_to_user_memory).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub async fn search_memories(&self, user_id: i64, query: &str) -> Vec<UserMemory> {
        match xiaoxin_repo::search_memories(&self.pool, user_id, query, 20).await {
            Ok(rows) => rows.into_iter().map(row_to_user_memory).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub async fn delete_memory(&self, user_id: i64, id: &str) -> Result<(), AppError> {
        let deleted = xiaoxin_repo::delete_memory(&self.pool, user_id, id).await?;
        if deleted {
            Ok(())
        } else {
            Err(AppError::Internal(format!("记忆 {} 不存在", id)))
        }
    }

    pub async fn analyze_sentiment(&self, text: &str) -> SentimentResult {
        let mut positive_score: f64 = 0.0;
        let mut negative_score: f64 = 0.0;
        let mut emotion_scores: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();

        let pos_words = [
            "好", "棒", "赞", "优秀", "开心", "快乐", "高兴", "喜欢", "爱", "感谢",
            "谢谢", "太棒了", "完美", "精彩", "成功", "进步", "厉害", "优秀", "不错",
            "满意", "满意", "温暖", "感动", "幸福", "美好", "激动", "期待", "信心",
            "great", "good", "excellent", "amazing", "love", "wonderful", "happy",
        ];
        let neg_words = [
            "不好", "差", "糟", "烦", "生气", "难过", "悲伤", "讨厌", "恨", "糟糕",
            "失败", "错误", "失望", "焦虑", "担心", "害怕", "累", "痛苦", "难受",
            "无聊", "崩溃", "绝望", "愤怒", "烦躁", "压力", "纠结", "困惑", "迷茫",
            "bad", "terrible", "hate", "awful", "sad", "angry", "worried",
        ];

        let emotions_map: Vec<(&str, &[&str])> = vec![
            ("happy", &["开心", "快乐", "高兴", "幸福", "美好", "激动", "happy", "joy"] as &[&str]),
            ("sad", &["难过", "悲伤", "伤心", "哭", "sad", "sorrow", "depressed"]),
            ("angry", &["生气", "愤怒", "讨厌", "恨", "angry", "furious"]),
            ("surprised", &["惊讶", "震惊", "意外", "surprised", "shocked"]),
            ("anxious", &["焦虑", "担心", "害怕", "紧张", "压力", "anxious", "worried"]),
            ("calm", &["平静", "冷静", "安定", "calm", "serene", "peaceful"]),
            ("curious", &["好奇", "想", "探索", "疑问", "curious", "interested"]),
            ("confused", &["困惑", "迷茫", "不懂", "confused", "puzzled"]),
        ];

        for word in pos_words {
            if text.to_lowercase().contains(&word.to_lowercase()) {
                positive_score += 0.15;
            }
        }
        for word in neg_words {
            if text.to_lowercase().contains(&word.to_lowercase()) {
                negative_score += 0.15;
            }
        }

        for (category, words) in &emotions_map {
            let mut score: f64 = 0.0;
            for word in *words {
                if text.to_lowercase().contains(&word.to_lowercase()) {
                    score += 0.2;
                }
            }
            if score > 0.0 {
                emotion_scores.insert(category.to_string(), score.min(1.0));
            }
        }

        positive_score = positive_score.min(1.0);
        negative_score = negative_score.min(1.0);

        let label = if positive_score > negative_score + 0.1 {
            SentimentLabel::Positive
        } else if negative_score > positive_score + 0.1 {
            SentimentLabel::Negative
        } else {
            SentimentLabel::Neutral
        };

        let score = match label {
            SentimentLabel::Positive => positive_score,
            SentimentLabel::Negative => negative_score,
            SentimentLabel::Neutral => 0.5,
        };

        let emotions: Vec<EmotionInfo> = emotion_scores
            .into_iter()
            .map(|(category, intensity)| EmotionInfo {
                category: match category.as_str() {
                    "happy" => EmotionCategory::Happy,
                    "sad" => EmotionCategory::Sad,
                    "angry" => EmotionCategory::Angry,
                    "surprised" => EmotionCategory::Surprised,
                    "anxious" => EmotionCategory::Anxious,
                    "calm" => EmotionCategory::Calm,
                    "curious" => EmotionCategory::Curious,
                    _ => EmotionCategory::Confused,
                },
                intensity,
            })
            .collect();

        SentimentResult { label, score, emotions }
    }

    pub async fn get_tts_status(&self) -> TtsStatus {
        TtsStatus {
            available: false,
            engine: "system".into(),
            voices: vec!["default".into()],
        }
    }

    pub async fn tts_speak(&self, user_id: i64, request: TtsSpeakRequest) -> Result<(), AppError> {
        let cfg = self.get_config(user_id).await;
        if !cfg.tts_enabled {
            return Err(AppError::Internal("TTS 功能未启用".into()));
        }
        tracing::info!(
            text = %request.text,
            speed = ?request.speed,
            "小欣 TTS - 收到语音合成请求（Phase 1 为日志模式）"
        );
        Ok(())
    }

    pub async fn process_multimodal(&self, input: MultimodalInput) -> Result<String, AppError> {
        let mut parts: Vec<String> = Vec::new();

        if let Some(ref text) = input.text {
            if !text.trim().is_empty() {
                parts.push(format!("[用户输入]\n{}", text));
            }
        }

        for (i, img) in input.images.iter().enumerate() {
            parts.push(format!(
                "[图片 {}] 格式: {}, 大小: {} chars",
                i + 1,
                img.mime_type,
                img.base64_data.len()
            ));
            if let Some(ref desc) = img.description {
                parts.push(format!("  描述: {}", desc));
            }
        }

        for (i, att) in input.attachments.iter().enumerate() {
            parts.push(format!(
                "[附件 {}] {} ({})",
                i + 1,
                att.name,
                att.mime_type
            ));
            if let Some(ref content) = att.content {
                parts.push(format!("  内容预览: {}...", &content[..content.len().min(200)]));
            }
        }

        Ok(parts.join("\n"))
    }

    pub async fn update_mood(&self, user_id: i64, text: &str) -> Mood {
        let sentiment = self.analyze_sentiment(text).await;
        let top_emotion = sentiment
            .emotions
            .iter()
            .max_by(|a, b| a.intensity.partial_cmp(&b.intensity).expect("intensity should not be NaN"))
            .map(|e| e.category.clone());

        let mood = Mood {
            category: match top_emotion {
                Some(EmotionCategory::Happy) => "happy".into(),
                Some(EmotionCategory::Sad) => "sad".into(),
                Some(EmotionCategory::Angry) => "angry".into(),
                Some(EmotionCategory::Surprised) => "surprised".into(),
                Some(EmotionCategory::Anxious) => "anxious".into(),
                Some(EmotionCategory::Curious) => "curious".into(),
                _ => "calm".into(),
            },
            intensity: sentiment.emotions.first().map(|e| e.intensity).unwrap_or(0.5),
            updated_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            trigger: Some(text.chars().take(100).collect()),
        };

        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let trigger_str: Option<&str> = mood.trigger.as_deref();
        let _ = xiaoxin_repo::insert_mood(
            &self.pool,
            user_id,
            &mood.category,
            mood.intensity,
            &now,
            trigger_str,
            None,
        )
        .await;

        mood
    }

    pub async fn get_current_mood(&self, user_id: i64) -> Mood {
        match xiaoxin_repo::get_mood_history(&self.pool, user_id, 1).await {
            Ok(rows) if !rows.is_empty() => {
                let row = &rows[0];
                Mood {
                    category: row.category.clone(),
                    intensity: row.intensity,
                    updated_at: row.updated_at.clone(),
                    trigger: row.trigger_text.clone(),
                }
            }
            _ => Mood {
                category: "calm".into(),
                intensity: 0.5,
                updated_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                trigger: None,
            },
        }
    }

    pub async fn add_conversation_summary(&self, user_id: i64, summary: ConversationSummary) {
        let now = &summary.created_at;
        let key_takeaways_json =
            serde_json::to_string(&summary.key_takeaways).unwrap_or_else(|_| "[]".into());
        let topics_json =
            serde_json::to_string(&summary.topics).unwrap_or_else(|_| "[]".into());
        let sentiment_json = summary
            .sentiment
            .as_ref()
            .and_then(|s| serde_json::to_string(s).ok());

        let _ = xiaoxin_repo::insert_summary(
            &self.pool,
            user_id,
            &summary.id,
            summary.conversation_id,
            &summary.summary,
            &key_takeaways_json,
            &topics_json,
            sentiment_json.as_deref(),
            now,
        )
        .await;
    }

    pub async fn get_conversation_summaries(
        &self,
        user_id: i64,
        limit: Option<usize>,
    ) -> Vec<ConversationSummary> {
        let limit_i64 = limit.unwrap_or(20) as i64;
        match xiaoxin_repo::get_summaries(&self.pool, user_id, limit_i64).await {
            Ok(rows) => rows.into_iter().map(row_to_summary).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub async fn generate_daily_briefing(&self, user_id: i64) -> DailyBriefing {
        let persona = self.get_active_persona(user_id).await;
        let hour = Utc::now().format("%H").to_string().parse::<u32>().unwrap_or(12);

        let greeting = match hour {
            0..=5 => "夜深了，注意休息哦 🌙".into(),
            6..=9 => "早上好！新的一天开始了 ☀️".into(),
            10..=11 => "上午好！精力充沛的时刻 💪".into(),
            12..=13 => "中午好！别忘了吃午饭 🍜".into(),
            14..=17 => "下午好！保持专注 ⚡".into(),
            18..=20 => "傍晚好！今天辛苦了 🌆".into(),
            _ => "晚上好！放松一下 🌃".into(),
        };

        let memories = self.get_memories(user_id, None, Some(3)).await;

        let quotes = [
            ("代码是写给人看的，顺便给机器运行。", "— Harold Abelson"),
            ("优秀的代码本身就是最好的文档。", "— Steve McConnell"),
            ("简单是可靠的前提。", "— Edsger Dijkstra"),
            ("学习是永恒的旅程。", "— 谚语"),
            ("每一次错误都是成长的机会。", "— 未知"),
        ];
        let today_idx = Utc::now().format("%j").to_string().parse::<usize>().unwrap_or(0)
            % quotes.len();
        let quote = quotes[today_idx];

        DailyBriefing {
            date: Utc::now().format("%Y-%m-%d").to_string(),
            greeting,
            quote_of_the_day: format!("{} {}", quote.0, quote.1),
            memory_recall: memories,
            mood_suggestion: match persona.id.as_str() {
                "code_assistant" => "今天先理清需求再动手写代码吧！".into(),
                "knowledge_tutor" => "每天学一点新知识，积少成多 📖".into(),
                "creative_partner" => "尝试换个角度看问题，创意就在转角处 ✨".into(),
                "caring_friend" => "记得给自己泡杯茶，享受当下 ☕".into(),
                _ => "今天是美好的一天！".into(),
            },
            focus_suggestion: match hour {
                6..=12 => "早上大脑清醒，适合处理复杂逻辑".into(),
                13..=17 => "下午适合协作沟通和代码审查".into(),
                _ => "晚上适合学习和整理复盘".into(),
            },
        }
    }

    pub async fn generate_personality_insights(&self, user_id: i64) -> PersonalityInsight {
        let memories = self.get_memories(user_id, None, None).await;
        let persona = self.get_active_persona(user_id).await;

        let total = memories.len();
        let pref_count = memories
            .iter()
            .filter(|m| m.category == MemoryCategory::Preference)
            .count();
        let fact_count = memories
            .iter()
            .filter(|m| m.category == MemoryCategory::Fact)
            .count();
        let habit_count = memories
            .iter()
            .filter(|m| m.category == MemoryCategory::Habit)
            .count();

        let mut interest_domains: Vec<String> = Vec::new();
        if total > 0 {
            let pref_ratio = pref_count as f64 / total as f64;
            if pref_ratio > 0.2 {
                interest_domains.push("偏好驱动型".into());
            }
            let fact_ratio = fact_count as f64 / total as f64;
            if fact_ratio > 0.4 {
                interest_domains.push("知识积累型".into());
            }
            let habit_ratio = habit_count as f64 / total as f64;
            if habit_ratio > 0.15 {
                interest_domains.push("习惯养成型".into());
            }
        }
        if interest_domains.is_empty() {
            interest_domains.push("探索型".into());
        }

        let communication_style = match persona.id.as_str() {
            "code_assistant" => "偏好结构化、逻辑清晰的技术沟通".into(),
            "knowledge_tutor" => "喜欢循序渐进的学习式对话".into(),
            "creative_partner" => "享受开放式、启发性的交流".into(),
            "caring_friend" => "倾向于温暖、共情的互动方式".into(),
            _ => "中性沟通风格".into(),
        };

        PersonalityInsight {
            dominant_traits: persona.traits.iter().map(|t| t.name.clone()).collect(),
            communication_style,
            interest_domains,
            suggested_persona: persona.name.clone(),
            insights_text: format!(
                "小欣分析：你共存储了 {} 条记忆，其中 {} 条偏好、{} 条事实、{} 条习惯。目前使用「{}」人格与你互动。",
                total, pref_count, fact_count, habit_count, persona.name
            ),
        }
    }
}