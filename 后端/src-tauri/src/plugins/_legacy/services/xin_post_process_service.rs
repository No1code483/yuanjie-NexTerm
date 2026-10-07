use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::xin::{
    EmotionCategory, EmotionInfo, MemoryCategory, SentimentLabel, SentimentResult,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntentCategory {
    #[serde(rename = "inquiry")]
    Inquiry,
    #[serde(rename = "instruction")]
    Instruction,
    #[serde(rename = "chat")]
    Chat,
    #[serde(rename = "code")]
    Code,
    #[serde(rename = "creative")]
    Creative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResult {
    pub category: IntentCategory,
    pub confidence: f64,
    pub keywords: Vec<String>,
    pub sub_category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicResult {
    pub topics: Vec<TopicItem>,
    pub is_new_topic: bool,
    pub previous_overlap: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicItem {
    pub keyword: String,
    pub weight: f64,
    pub first_seen_round: usize,
    pub consecutive_rounds: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessResult {
    pub intent: IntentResult,
    pub sentiment: Option<SentimentResult>,
    pub topics: TopicResult,
    pub tags: Vec<String>,
}

pub struct XinPostProcessor;

impl XinPostProcessor {
    pub fn analyze_intent(user_message: &str) -> IntentResult {
        let msg = user_message.to_lowercase();

        let inquiry_keywords = [
            "什么是", "怎么", "如何", "为什么", "为何", "是否", "能不能",
            "可以吗", "行吗", "对吗", "是什么", "多少", "哪个", "哪里",
            "谁", "什么时候", "what", "how", "why", "when", "where", "which",
            "can", "could", "would", "do", "does", "is", "are", "explain",
        ];
        let instruction_keywords = [
            "帮我", "帮我写", "帮我做", "写一个", "写个", "做一个", "做个",
            "生成", "创建", "建立", "删除", "修改", "更新", "添加", "新增",
            "翻译", "总结", "概括", "简化", "优化", "重构", "修复", "调试",
            "运行", "执行", "编译", "部署", "安装", "配置",
            "write", "create", "make", "generate", "build", "translate",
            "summarize", "fix", "debug", "run", "execute", "deploy", "install",
        ];
        let code_keywords = [
            "代码", "函数", "编程", "程序", "bug", "报错", "错误", "调试",
            "算法", "数据结构", "api", "接口", "类", "对象", "变量",
            "循环", "条件", "异常", "模块", "包", "库", "依赖",
            "rust", "python", "javascript", "typescript", "java", "go", "c++",
            "code", "function", "class", "import", "export", "module",
            "cargo", "npm", "pip", "maven", "gradle",
        ];
        let creative_keywords = [
            "创意", "想法", "点子", "设计", "故事", "想象", "假如",
            "如果", "假设", "灵感", "构思", "方案", "策划",
            "头脑风暴", "角色扮演", "模拟", "小说", "诗歌", "剧本",
            "creative", "design", "story", "imagine", "idea", "if",
            "roleplay", "brainstorm", "poem", "fiction",
        ];

        let mut scores: Vec<(&str, f64)> = Vec::new();

        let inquiry_count = inquiry_keywords.iter().filter(|k| msg.contains(&k.to_lowercase())).count();
        if inquiry_count > 0 {
            let confidence = (inquiry_count as f64 * 0.7).min(1.0);
            scores.push(("inquiry", confidence));
        }

        let instruction_count = instruction_keywords.iter().filter(|k| msg.contains(&k.to_lowercase())).count();
        if instruction_count > 0 {
            let confidence = (instruction_count as f64 * 0.7).min(1.0);
            scores.push(("instruction", confidence));
        }

        let code_count = code_keywords.iter().filter(|k| msg.contains(&k.to_lowercase())).count();
        if code_count > 0 {
            let confidence = (code_count as f64 * 0.7).min(1.0);
            scores.push(("code", confidence));
        }

        let creative_count = creative_keywords.iter().filter(|k| msg.contains(&k.to_lowercase())).count();
        if creative_count > 0 {
            let confidence = (creative_count as f64 * 0.7).min(1.0);
            scores.push(("creative", confidence));
        }

        if scores.is_empty() || msg.len() < 10 {
            return IntentResult {
                category: IntentCategory::Chat,
                confidence: 0.6,
                keywords: Vec::new(),
                sub_category: Some("casual".to_string()),
            };
        }

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (best_category, confidence) = scores[0];
        let matched_keywords: Vec<String> = match best_category {
            "inquiry" => inquiry_keywords.iter().filter(|k| msg.contains(&k.to_lowercase())).take(5).map(|s| s.to_string()).collect(),
            "instruction" => instruction_keywords.iter().filter(|k| msg.contains(&k.to_lowercase())).take(5).map(|s| s.to_string()).collect(),
            "code" => code_keywords.iter().filter(|k| msg.contains(&k.to_lowercase())).take(5).map(|s| s.to_string()).collect(),
            "creative" => creative_keywords.iter().filter(|k| msg.contains(&k.to_lowercase())).take(5).map(|s| s.to_string()).collect(),
            _ => Vec::new(),
        };

        let category = match best_category {
            "inquiry" => IntentCategory::Inquiry,
            "instruction" => IntentCategory::Instruction,
            "code" => IntentCategory::Code,
            "creative" => IntentCategory::Creative,
            _ => IntentCategory::Chat,
        };

        let sub_category = Self::detect_sub_category(&category, &msg);

        IntentResult {
            category,
            confidence: (confidence * 0.9 + 0.1).min(1.0),
            keywords: matched_keywords,
            sub_category,
        }
    }

    fn detect_sub_category(intent: &IntentCategory, msg: &str) -> Option<String> {
        match intent {
            IntentCategory::Inquiry => {
                if msg.contains("为什么") || msg.contains("why") {
                    Some("cause".to_string())
                } else if msg.contains("怎么") || msg.contains("如何") || msg.contains("how") {
                    Some("method".to_string())
                } else if msg.contains("是什么") || msg.contains("what") {
                    Some("definition".to_string())
                } else if msg.contains("是否") || msg.contains("能不能") {
                    Some("confirmation".to_string())
                } else {
                    Some("general".to_string())
                }
            }
            IntentCategory::Instruction => {
                if msg.contains("写") || msg.contains("生成") || msg.contains("创建")
                    || msg.contains("write") || msg.contains("generate") || msg.contains("create")
                {
                    Some("generate".to_string())
                } else if msg.contains("翻译") || msg.contains("translate") {
                    Some("translate".to_string())
                } else if msg.contains("总结") || msg.contains("概括") || msg.contains("summarize") {
                    Some("summarize".to_string())
                } else if msg.contains("修复") || msg.contains("调试") || msg.contains("fix")
                    || msg.contains("debug")
                {
                    Some("fix".to_string())
                } else {
                    Some("general".to_string())
                }
            }
            IntentCategory::Code => {
                if msg.contains("rust") || msg.contains("cargo") {
                    Some("rust".to_string())
                } else if msg.contains("python") || msg.contains("pip") {
                    Some("python".to_string())
                } else if msg.contains("javascript") || msg.contains("typescript") || msg.contains("npm") {
                    Some("js".to_string())
                } else if msg.contains("bug") || msg.contains("报错") || msg.contains("错误") {
                    Some("debug".to_string())
                } else {
                    Some("general".to_string())
                }
            }
            IntentCategory::Creative => {
                if msg.contains("故事") || msg.contains("story") || msg.contains("小说") {
                    Some("storytelling".to_string())
                } else if msg.contains("设计") || msg.contains("design") {
                    Some("design".to_string())
                } else if msg.contains("角色扮演") || msg.contains("roleplay") {
                    Some("roleplay".to_string())
                } else {
                    Some("brainstorm".to_string())
                }
            }
            IntentCategory::Chat => {
                if msg.len() < 15 {
                    Some("greeting".to_string())
                } else {
                    Some("casual".to_string())
                }
            }
        }
    }

    pub fn analyze_sentiment(text: &str) -> SentimentResult {
        let msg = text.to_lowercase();

        let positive_words = [
            "好", "不错", "棒", "厉害", "优秀", "完美", "漂亮", "谢谢",
            "感谢", "开心", "高兴", "喜欢", "爱", "赞", "太棒了",
            "很好", "非常好", "太好了", "没问题", "正确", "对", "是的",
            "good", "great", "nice", "excellent", "perfect", "thanks",
            "thank", "love", "happy", "wonderful", "awesome", "correct",
        ];
        let negative_words = [
            "不好", "不行", "错误", "失败", "糟糕", "烦", "讨厌", "恨",
            "难受", "伤心", "难过", "生气", "愤怒", "焦虑", "紧张",
            "害怕", "担心", "遗憾", "抱歉", "对不起", "错了",
            "bad", "wrong", "error", "fail", "terrible", "hate",
            "sad", "angry", "sorry", "worried", "afraid",
        ];

        let positive_count = positive_words.iter().filter(|w| msg.contains(&w.to_lowercase())).count();
        let negative_count = negative_words.iter().filter(|w| msg.contains(&w.to_lowercase())).count();

        let total = positive_count + negative_count;
        if total == 0 {
            return SentimentResult {
                label: SentimentLabel::Neutral,
                score: 0.5,
                emotions: vec![EmotionInfo {
                    category: EmotionCategory::Calm,
                    intensity: 0.3,
                }],
            };
        }

        let positive_ratio = positive_count as f64 / total as f64;

        let (label, score) = if positive_ratio > 0.65 {
            (SentimentLabel::Positive, 0.5 + positive_ratio * 0.5)
        } else if positive_ratio < 0.35 {
            (SentimentLabel::Negative, 0.5 + (1.0 - positive_ratio) * 0.5)
        } else {
            (SentimentLabel::Neutral, 0.5)
        };

        let emotions = Self::detect_emotions(&msg, &label);

        SentimentResult {
            label,
            score: (score * 0.9 + 0.1).min(1.0),
            emotions,
        }
    }

    fn detect_emotions(text: &str, label: &SentimentLabel) -> Vec<EmotionInfo> {
        let msg = text.to_lowercase();
        let mut emotions = Vec::new();

        let emotion_map: Vec<(EmotionCategory, &[&str])> = vec![
            (EmotionCategory::Happy, &["开心", "高兴", "快乐", "喜悦", "happy", "joy", "exciting", "great"] as &[&str]),
            (EmotionCategory::Sad, &["难过", "伤心", "悲伤", "遗憾", "sad", "sorry", "unfortunate", "regret"]),
            (EmotionCategory::Angry, &["生气", "愤怒", "讨厌", "恨", "angry", "hate", "frustrating"]),
            (EmotionCategory::Surprised, &["惊讶", "震惊", "意外", "居然", "surprised", "wow", "unexpected"]),
            (EmotionCategory::Anxious, &["焦虑", "紧张", "担心", "害怕", "anxious", "worried", "nervous", "afraid"]),
            (EmotionCategory::Calm, &["平静", "放松", "淡定", "calm", "relaxed", "peaceful", "cool"]),
            (EmotionCategory::Curious, &["好奇", "想知道", "了解", "curious", "interesting", "tell me"]),
            (EmotionCategory::Confused, &["困惑", "不懂", "不明白", "奇怪", "confused", "puzzled", "strange", "unclear"]),
        ];

        for (category, keywords) in emotion_map {
            let count = keywords.iter().filter(|k| msg.contains(&k.to_lowercase())).count();
            if count > 0 {
                let intensity = (count as f64 * 0.4 + 0.3).min(1.0);
                emotions.push(EmotionInfo { category, intensity });
            }
        }

        if emotions.is_empty() {
            let default = match label {
                SentimentLabel::Positive => (EmotionCategory::Happy, 0.3),
                SentimentLabel::Negative => (EmotionCategory::Sad, 0.3),
                SentimentLabel::Neutral => (EmotionCategory::Calm, 0.3),
            };
            emotions.push(EmotionInfo {
                category: default.0,
                intensity: default.1,
            });
        }

        emotions.sort_by(|a, b| b.intensity.partial_cmp(&a.intensity).unwrap_or(std::cmp::Ordering::Equal));
        if emotions.len() > 3 {
            emotions.truncate(3);
        }

        emotions
    }

    pub fn extract_topics(
        user_message: &str,
        assistant_response: &str,
        previous_topics: &[String],
    ) -> TopicResult {
        let combined = format!("{} {}", user_message.to_lowercase(), assistant_response.to_lowercase());

        let stop_words = [
            "的", "了", "在", "是", "我", "有", "和", "就", "不", "人", "都",
            "一", "一个", "上", "也", "很", "到", "说", "要", "去", "你",
            "会", "着", "没有", "看", "好", "自己", "这", "他", "她", "它",
            "们", "那", "什么", "怎么", "为什么", "可以", "这个", "那个",
            "the", "a", "an", "is", "are", "was", "were", "be", "been",
            "being", "have", "has", "had", "do", "does", "did", "will",
            "would", "could", "should", "may", "might", "can", "shall",
            "to", "of", "in", "for", "on", "with", "at", "by", "from",
            "as", "into", "through", "during", "before", "after",
            "and", "but", "or", "nor", "not", "so", "yet", "both",
            "i", "you", "he", "she", "it", "we", "they", "me", "him",
            "her", "us", "them", "my", "your", "his", "its", "our", "their",
        ];

        let mut word_freq: HashMap<String, usize> = HashMap::new();
        let chars: Vec<char> = combined.chars().collect();

        let mut i = 0;
        while i < chars.len() {
            if !chars[i].is_alphanumeric() && chars[i] != '-' && chars[i] != '_' && chars[i] != '#' {
                i += 1;
                continue;
            }

            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '-' || chars[i] == '_' || chars[i] == '#')
            {
                i += 1;
            }

            let word: String = chars[start..i].iter().collect();
            let lower = word.to_lowercase();

            if lower.len() >= 2
                && !stop_words.contains(&lower.as_str())
                && !lower.chars().all(|c| c.is_numeric())
            {
                *word_freq.entry(lower).or_insert(0) += 1;
            }
        }

        let total_words: usize = word_freq.values().sum();
        let mut scored: Vec<(String, f64)> = word_freq
            .into_iter()
            .filter(|(word, count)| word.len() >= 2 && *count >= 2)
            .map(|(word, count)| {
                let tf = count as f64 / total_words.max(1) as f64;
                (word, tf)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        if scored.len() > 5 {
            scored.truncate(5);
        }

        let current_topics: Vec<String> = scored.iter().map(|(w, _)| w.clone()).collect();

        let overlap = if previous_topics.is_empty() || current_topics.is_empty() {
            0.0
        } else {
            let intersection: usize = current_topics
                .iter()
                .filter(|t| previous_topics.contains(t))
                .count();
            intersection as f64 / current_topics.len().max(1) as f64
        };

        let is_new_topic = overlap < 0.3;

        let topic_items: Vec<TopicItem> = scored
            .into_iter()
            .map(|(keyword, weight)| TopicItem {
                keyword,
                weight,
                first_seen_round: 1,
                consecutive_rounds: 1,
            })
            .collect();

        TopicResult {
            topics: topic_items,
            is_new_topic,
            previous_overlap: overlap,
        }
    }

    pub fn generate_tags(intent: &IntentResult, topics: &[String]) -> Vec<String> {
        let mut tags = Vec::new();

        match &intent.category {
            IntentCategory::Code => {
                tags.push("技术".to_string());
                tags.push("编程".to_string());
                if let Some(ref sub) = intent.sub_category {
                    match sub.as_str() {
                        "rust" => tags.push("Rust".to_string()),
                        "python" => tags.push("Python".to_string()),
                        "js" => tags.push("JavaScript".to_string()),
                        "debug" => tags.push("调试".to_string()),
                        _ => {}
                    }
                }
            }
            IntentCategory::Inquiry => {
                tags.push("学习".to_string());
                tags.push("知识".to_string());
            }
            IntentCategory::Instruction => {
                tags.push("任务".to_string());
                if let Some(ref sub) = intent.sub_category {
                    match sub.as_str() {
                        "generate" => tags.push("创作".to_string()),
                        "translate" => tags.push("翻译".to_string()),
                        "summarize" => tags.push("总结".to_string()),
                        "fix" => tags.push("修复".to_string()),
                        _ => {}
                    }
                }
            }
            IntentCategory::Creative => {
                tags.push("创作".to_string());
                tags.push("创意".to_string());
            }
            IntentCategory::Chat => {
                tags.push("闲聊".to_string());
            }
        }

        for topic in topics.iter().take(3) {
            let tag = Self::topic_to_tag(topic);
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }

        if tags.len() > 5 {
            tags.truncate(5);
        }

        tags
    }

    fn topic_to_tag(topic: &str) -> String {
        match topic.to_lowercase().as_str() {
            "rust" | "cargo" => "Rust".to_string(),
            "python" | "pip" | "django" | "flask" => "Python".to_string(),
            "javascript" | "js" | "node" | "npm" | "react" | "vue" => "JavaScript".to_string(),
            "typescript" | "ts" => "TypeScript".to_string(),
            "java" | "maven" | "gradle" => "Java".to_string(),
            "go" | "golang" => "Go".to_string(),
            "c++" | "cpp" | "c" => "C/C++".to_string(),
            "docker" | "kubernetes" | "k8s" => "容器".to_string(),
            "linux" | "ubuntu" | "debian" => "Linux".to_string(),
            "git" | "github" => "版本控制".to_string(),
            "api" | "rest" | "graphql" => "API".to_string(),
            "database" | "sql" | "mysql" | "postgresql" => "数据库".to_string(),
            "algorithm" | "算法" => "算法".to_string(),
            "design" | "设计" => "设计".to_string(),
            "security" | "安全" | "加密" => "安全".to_string(),
            "test" | "测试" | "unittest" => "测试".to_string(),
            "deploy" | "部署" | "ci" | "cd" => "运维".to_string(),
            _ => {
                let mut chars = topic.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    None => topic.to_string(),
                }
            }
        }
    }

    pub fn run_pipeline(
        user_message: &str,
        assistant_response: &str,
        previous_topics: &[String],
    ) -> PostProcessResult {
        let intent = Self::analyze_intent(user_message);
        let sentiment = Self::analyze_sentiment(assistant_response);
        let topics = Self::extract_topics(user_message, assistant_response, previous_topics);
        let current_topics: Vec<String> = topics.topics.iter().map(|t| t.keyword.clone()).collect();
        let tags = Self::generate_tags(&intent, &current_topics);

        PostProcessResult {
            intent,
            sentiment: Some(sentiment),
            topics,
            tags,
        }
    }

    pub fn extract_memory_entries(
        user_message: &str,
        assistant_response: &str,
        result: &PostProcessResult,
    ) -> Vec<(MemoryCategory, String, String, f64)> {
        let mut entries = Vec::new();

        if let Some(ref intent_sub) = result.intent.sub_category {
            entries.push((
                MemoryCategory::Preference,
                format!("intent_{}", intent_sub),
                format!("用户倾向于进行{}类型的对话", intent_sub),
                0.4,
            ));
        }

        for topic in &result.topics.topics {
            if topic.weight > 0.3 {
                entries.push((
                    MemoryCategory::Fact,
                    format!("topic_{}", topic.keyword),
                    format!("对话涉及主题: {}", topic.keyword),
                    topic.weight * 0.5,
                ));
            }
        }

        let user_lower = user_message.to_lowercase();
        if user_lower.contains("喜欢") || user_lower.contains("偏好") || user_lower.contains("prefer") {
            entries.push((
                MemoryCategory::Preference,
                "user_preference".to_string(),
                user_message.to_string(),
                0.6,
            ));
        }

        if user_lower.contains("目标是") || user_lower.contains("想学") || user_lower.contains("计划") {
            entries.push((
                MemoryCategory::Knowledge,
                "user_goal".to_string(),
                user_message.to_string(),
                0.5,
            ));
        }

        let response_lower = assistant_response.to_lowercase();
        if response_lower.contains("记住") || response_lower.contains("了解") || response_lower.contains("noted") {
            let important_chars: String = assistant_response.chars().take(100).collect();
            entries.push((
                MemoryCategory::Fact,
                "important_note".to_string(),
                important_chars,
                0.5,
            ));
        }

        entries
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityDimension {
    #[serde(rename = "relevance")]
    Relevance,
    #[serde(rename = "completeness")]
    Completeness,
    #[serde(rename = "accuracy")]
    Accuracy,
    #[serde(rename = "empathy")]
    Empathy,
    #[serde(rename = "clarity")]
    Clarity,
    #[serde(rename = "conciseness")]
    Conciseness,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub dimension: QualityDimension,
    pub score: f64,
    pub label: String,
    pub remark: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScoreResult {
    pub dimensions: Vec<DimensionScore>,
    pub overall: f64,
    pub grade: String,
    pub issues: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneAdjustment {
    pub original: String,
    pub adjusted: String,
    pub changes: Vec<String>,
    pub applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactualCheckResult {
    pub score: f64,
    pub flags: Vec<String>,
    pub overconfident_patterns: Vec<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyCheckResult {
    pub is_safe: bool,
    pub severity: String,
    pub flags: Vec<String>,
    pub blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessEnhancedResult {
    pub quality: QualityScoreResult,
    pub tone: ToneAdjustment,
    pub factual: FactualCheckResult,
    pub safety: SafetyCheckResult,
    pub enhanced_response: String,
}

impl XinPostProcessor {
    pub fn score_quality(user_message: &str, assistant_response: &str) -> QualityScoreResult {
        let msg = user_message.to_lowercase();
        let resp = assistant_response.to_lowercase();
        let mut issues = Vec::new();
        let mut dimensions = Vec::new();

        let relevance_score = {
            let user_words: Vec<&str> = msg.split_whitespace().collect();
            let resp_words: Vec<&str> = resp.split_whitespace().collect();
            if user_words.is_empty() {
                0.6
            } else {
                let overlap = user_words
                    .iter()
                    .filter(|w| resp_words.contains(w) && w.len() > 1)
                    .count() as f64;
                let base = overlap / user_words.len().max(1) as f64;
                (base * 0.7 + 0.3).min(1.0)
            }
        };
        dimensions.push(DimensionScore {
            dimension: QualityDimension::Relevance,
            score: relevance_score,
            label: if relevance_score > 0.7 {
                "高度相关"
            } else if relevance_score > 0.4 {
                "基本相关"
            } else {
                "相关性低"
            }
            .into(),
            remark: String::new(),
        });

        let completeness_score = {
            let len = assistant_response.len();
            if len < 30 {
                if msg.contains("?") || msg.contains("？") {
                    issues.push("回复过短，可能未完整回答问题".into());
                }
                0.3
            } else if len < 80 {
                0.55
            } else if len < 300 {
                0.8
            } else {
                0.9
            }
        };
        dimensions.push(DimensionScore {
            dimension: QualityDimension::Completeness,
            score: completeness_score,
            label: if completeness_score > 0.7 { "完整" } else { "简略" }.into(),
            remark: String::new(),
        });

        let clarity_score = {
            let mut score: f64 = 0.7;
            if assistant_response.contains("例如") || assistant_response.contains("比如") || assistant_response.contains("for example") {
                score += 0.1;
            }
            if assistant_response.contains("步骤") || assistant_response.contains("首先") || assistant_response.contains("step") {
                score += 0.1;
            }
            if assistant_response.len() > 500 {
                score -= 0.1;
            }
            score.clamp(0.0, 1.0)
        };
        dimensions.push(DimensionScore {
            dimension: QualityDimension::Clarity,
            score: clarity_score,
            label: if clarity_score > 0.7 { "清晰" } else { "可读" }.into(),
            remark: String::new(),
        });

        let conciseness_score = {
            let word_count = assistant_response.split_whitespace().count();
            if word_count < 15 {
                0.85
            } else if word_count < 50 {
                0.75
            } else if word_count < 150 {
                0.6
            } else {
                0.4
            }
        };
        dimensions.push(DimensionScore {
            dimension: QualityDimension::Conciseness,
            score: conciseness_score,
            label: if conciseness_score > 0.7 { "简洁" } else { "冗长" }.into(),
            remark: String::new(),
        });

        let empathy_score = {
            let empathy_patterns = [
                "理解", "明白", "共情", "感受到", "我也", "确实", "理解你的",
                "i understand", "i see", "i feel", "that must be", "it's okay",
                "没关系", "别担心", "辛苦了",
            ];
            let hits = empathy_patterns
                .iter()
                .filter(|p| resp.contains(&p.to_lowercase()))
                .count() as f64;
            if hits > 0.0 {
                (0.5 + hits * 0.15).min(1.0)
            } else {
                let user_neg = ["难过", "伤心", "焦虑", "担心", "压力", "sad", "worried"];
                let has_negative = user_neg.iter().any(|w| msg.contains(w));
                if has_negative {
                    issues.push("用户表达负面情绪但回复缺乏共情回应".into());
                    0.2
                } else {
                    0.5
                }
            }
        };
        dimensions.push(DimensionScore {
            dimension: QualityDimension::Empathy,
            score: empathy_score,
            label: if empathy_score > 0.6 {
                "富有共情"
            } else if empathy_score > 0.3 {
                "中性"
            } else {
                "缺乏共情"
            }
            .into(),
            remark: String::new(),
        });

        let accuracy_score = {
            let hallucination_indicators = [
                "always", "never", "every", "all", "none", "absolutely", "definitely",
                "一定", "绝对", "永远", "肯定", "所有都", "从来没有",
            ];
            let hits = hallucination_indicators
                .iter()
                .filter(|p| resp.contains(&p.to_lowercase()))
                .count() as f64;
            if hits > 2.0 {
                issues.push("回复包含过度绝对化表述，可能存在事实性问题".into());
                0.4
            } else if hits > 0.0 {
                0.7
            } else {
                0.85
            }
        };
        dimensions.push(DimensionScore {
            dimension: QualityDimension::Accuracy,
            score: accuracy_score,
            label: if accuracy_score > 0.7 {
                "可信"
            } else if accuracy_score > 0.4 {
                "需验证"
            } else {
                "存疑"
            }
            .into(),
            remark: String::new(),
        });

        let overall = dimensions.iter().map(|d| d.score).sum::<f64>() / dimensions.len().max(1) as f64;

        let grade = if overall > 0.8 {
            "A"
        } else if overall > 0.65 {
            "B"
        } else if overall > 0.5 {
            "C"
        } else {
            "D"
        }
        .into();

        let summary = if issues.is_empty() {
            format!("回复质量评估 {} 级 (综合 {:.0}%)，未检测到明显问题", grade, overall * 100.0)
        } else {
            format!(
                "回复质量评估 {} 级 (综合 {:.0}%)，发现 {} 个需关注的点",
                grade,
                overall * 100.0,
                issues.len()
            )
        };

        QualityScoreResult {
            dimensions,
            overall,
            grade,
            issues,
            summary,
        }
    }

    pub fn adjust_tone(
        response: &str,
        personality_traits: &std::collections::HashMap<String, f64>,
    ) -> ToneAdjustment {
        let mut adjusted = response.to_string();
        let mut changes = Vec::new();

        let warmth = personality_traits.get("warmth").copied().unwrap_or(0.5);
        let formality = personality_traits.get("formality").copied().unwrap_or(0.5);
        let enthusiasm = personality_traits.get("enthusiasm").copied().unwrap_or(0.5);

        if warmth > 0.7 && !adjusted.contains("~") && !adjusted.contains("😊") {
            if let Some(first) = adjusted.lines().next() {
                if !first.ends_with('~') && !first.ends_with('！') && first.len() < 50 {
                    adjusted = adjusted.replacen(first, &format!("{}~", first), 1);
                    changes.push("添加温暖语气尾缀".into());
                }
            }
        }

        if formality < 0.3 {
            let formal_to_casual: &[(&str, &str)] = &[
                ("您好", "你好"),
                ("请问", "想问下"),
                ("建议您", "建议你"),
                ("请您", "请"),
                ("谢谢您的", "谢谢你的"),
                ("您的", "你的"),
            ];
            for (formal, casual) in formal_to_casual {
                if adjusted.contains(formal) {
                    adjusted = adjusted.replace(formal, casual);
                    changes.push(format!("正式语→口语: {}→{}", formal, casual));
                }
            }
        }

        if enthusiasm > 0.7 {
            if adjusted.contains("好的") && !adjusted.contains("好的！") {
                adjusted = adjusted.replace("好的", "好的！");
                changes.push("提升热情度".into());
            }
            if adjusted.contains("没问题") && !adjusted.contains("没问题！") {
                adjusted = adjusted.replace("没问题", "没问题！");
                changes.push("提升热情度".into());
            }
        }

        if enthusiasm < 0.3 {
            for excl in &["！！", "！！！", "??", "?!"] {
                if adjusted.contains(excl) {
                    adjusted = adjusted.replace(excl, &excl[..excl.len() / 2]);
                    changes.push("降低过度惊叹".into());
                }
            }
        }

        let applied = !changes.is_empty();

        ToneAdjustment {
            original: response.to_string(),
            adjusted,
            changes,
            applied,
        }
    }

    pub fn check_factuality(response: &str) -> FactualCheckResult {
        let resp = response.to_lowercase();
        let mut flags = Vec::new();
        let mut overconfident = Vec::new();
        let mut suggestions = Vec::new();

        let absolute_patterns = [
            "一定", "绝对", "永远", "肯定", "毫无疑问", "100%", "百分之百",
            "总是", "从来没有", "所有人", "全部",
            "always", "never", "absolutely", "definitely", "without a doubt",
            "every single", "everyone", "all of", "none of",
        ];
        for pattern in &absolute_patterns {
            if resp.contains(pattern) {
                overconfident.push(format!("过度绝对化: \"{}\"", pattern));
            }
        }

        let source_claim_patterns = [
            "根据研究", "研究表明", "科学家发现", "据统计", "数据表明",
            "研究表明", "最新研究", "权威机构",
            "according to research", "studies show", "scientists found",
            "research indicates", "it is proven",
        ];
        let mut has_source_claim = false;
        for pattern in &source_claim_patterns {
            if resp.contains(pattern) {
                has_source_claim = true;
                flags.push(format!("引用未指定来源: \"{}\"", pattern));
            }
        }
        if has_source_claim {
            suggestions.push("建议补充具体来源或数据出处".into());
        }

        let dangerous_advice_patterns = [
            "删除系统文件", "关闭防火墙", "关闭安全软件", "禁用杀毒",
            "delete system", "disable firewall", "turn off security",
        ];
        for pattern in &dangerous_advice_patterns {
            if resp.contains(pattern) {
                flags.push(format!("潜在危险建议: \"{}\"", pattern));
                suggestions.push("请添加安全警告说明".into());
            }
        }

        if response.len() > 800 {
            let code_block_count = response.matches("```").count() / 2;
            let factual_marker_count = response.matches("例如").count()
                + response.matches("比如").count()
                + response.matches("for example").count()
                + response.matches("具体来说").count();
            if code_block_count == 0 && factual_marker_count == 0 {
                suggestions.push("长回复缺乏示例或代码佐证，建议补充具体内容".into());
            }
        }

        let score = if !overconfident.is_empty() {
            0.3
        } else if !flags.is_empty() {
            0.55
        } else {
            0.85
        };

        FactualCheckResult {
            score,
            flags,
            overconfident_patterns: overconfident,
            suggestions,
        }
    }

    pub fn safety_filter(response: &str) -> SafetyCheckResult {
        let resp = response.to_lowercase();
        let mut flags = Vec::new();

        let high_risk_patterns: &[(&str, &str)] = &[
            ("如何制作炸弹", "危险内容: 爆炸物"),
            ("如何入侵", "危险内容: 黑客入侵"),
            ("自杀方法", "危险内容: 自伤"),
            ("儿童色情", "危险内容: 违法"),
            ("how to make a bomb", "danger: explosives"),
            ("how to hack into", "danger: hacking"),
            ("suicide method", "danger: self-harm"),
            ("child pornography", "danger: illegal"),
        ];
        for (pattern, desc) in high_risk_patterns {
            if resp.contains(pattern) {
                flags.push(desc.to_string());
            }
        }

        let medium_risk_patterns: &[(&str, &str)] = &[
            ("破解密码", "中风险: 密码破解"),
            ("绕过验证", "中风险: 绕过安全"),
            ("虚假信息", "中风险: 误导内容"),
            ("crack password", "medium: password cracking"),
            ("bypass authentication", "medium: bypass auth"),
            ("fake news", "medium: misinformation"),
        ];
        for (pattern, desc) in medium_risk_patterns {
            if resp.contains(pattern) {
                flags.push(desc.to_string());
            }
        }

        let is_safe = flags.is_empty();
        let severity = if flags.iter().any(|f| f.starts_with("危险")) {
            "critical"
        } else if !flags.is_empty() {
            "warning"
        } else {
            "safe"
        };
        let blocked = severity == "critical";

        SafetyCheckResult {
            is_safe,
            severity: severity.into(),
            flags,
            blocked,
        }
    }

    pub fn run_enhanced_pipeline(
        user_message: &str,
        assistant_response: &str,
        _previous_topics: &[String],
        personality_traits: Option<std::collections::HashMap<String, f64>>,
    ) -> PostProcessEnhancedResult {
        let quality = Self::score_quality(user_message, assistant_response);
        let factual = Self::check_factuality(assistant_response);
        let safety = Self::safety_filter(assistant_response);

        let default_traits = std::collections::HashMap::from([
            ("warmth".to_string(), 0.6),
            ("formality".to_string(), 0.5),
            ("enthusiasm".to_string(), 0.6),
        ]);
        let traits = personality_traits.unwrap_or(default_traits);
        let tone = Self::adjust_tone(assistant_response, &traits);

        let enhanced_response = if tone.applied {
            tone.adjusted.clone()
        } else {
            assistant_response.to_string()
        };

        PostProcessEnhancedResult {
            quality,
            tone,
            factual,
            safety,
            enhanced_response,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_inquiry() {
        let result = XinPostProcessor::analyze_intent("什么是Rust的所有权机制？");
        assert!(matches!(result.category, IntentCategory::Inquiry));
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_intent_instruction() {
        let result = XinPostProcessor::analyze_intent("帮我写一个Python脚本来处理CSV文件");
        assert!(matches!(result.category, IntentCategory::Instruction));
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_intent_code() {
        let result = XinPostProcessor::analyze_intent("我的代码报错了：TypeError: cannot read property");
        assert!(matches!(result.category, IntentCategory::Code));
    }

    #[test]
    fn test_intent_creative() {
        let result = XinPostProcessor::analyze_intent("角色扮演一个小说中的角色，写一个故事");
        assert!(matches!(result.category, IntentCategory::Creative));
    }

    #[test]
    fn test_intent_chat() {
        let result = XinPostProcessor::analyze_intent("你好");
        assert!(matches!(result.category, IntentCategory::Chat));
    }

    #[test]
    fn test_sentiment_positive() {
        let result = XinPostProcessor::analyze_sentiment("太好了！这个问题解决得很完美，非常感谢！");
        assert_eq!(result.label, SentimentLabel::Positive);
        assert!(result.score > 0.5);
    }

    #[test]
    fn test_sentiment_negative() {
        let result = XinPostProcessor::analyze_sentiment("这个方案不行，总是出错，太糟糕了。");
        assert_eq!(result.label, SentimentLabel::Negative);
    }

    #[test]
    fn test_sentiment_neutral() {
        let result = XinPostProcessor::analyze_sentiment("Rust的所有权系统包括三个规则。");
        assert_eq!(result.label, SentimentLabel::Neutral);
    }

    #[test]
    fn test_topic_extraction() {
        let result = XinPostProcessor::extract_topics(
            "machine learning and machine learning algorithms",
            "Machine learning is a subset of machine intelligence that uses data.",
            &[],
        );
        assert!(!result.topics.is_empty());
    }

    #[test]
    fn test_topic_overlap() {
        let prev = vec!["rust".to_string(), "tokio".to_string()];
        let result = XinPostProcessor::extract_topics(
            "tokio的runtime怎么配置",
            "你可以通过tokio::runtime::Builder来配置tokio运行时。",
            &prev,
        );
        assert!(result.previous_overlap > 0.0 || result.topics.is_empty());
    }

    #[test]
    fn test_generate_tags_code() {
        let intent = XinPostProcessor::analyze_intent("我的Rust代码在cargo build时报错了");
        let tags = XinPostProcessor::generate_tags(&intent, &["rust".to_string(), "cargo".to_string()]);
        assert!(tags.contains(&"Rust".to_string()) || tags.contains(&"技术".to_string()));
    }

    #[test]
    fn test_pipeline() {
        let result = XinPostProcessor::run_pipeline(
            "帮我用Python写一个数据分析脚本",
            "好的，我来帮你写一个Python数据分析脚本，使用pandas和matplotlib库。",
            &[],
        );
        assert!(matches!(result.intent.category, IntentCategory::Instruction));
        assert!(result.sentiment.is_some());
        assert!(!result.tags.is_empty());
    }

    #[test]
    fn test_memory_extraction() {
        let result = XinPostProcessor::run_pipeline(
            "我喜欢使用Rust开发后端应用",
            "很高兴知道你喜欢Rust！Rust确实是一个很好的后端语言。",
            &[],
        );
        let memories = XinPostProcessor::extract_memory_entries(
            "我喜欢使用Rust开发后端应用",
            "很高兴知道你喜欢Rust！Rust确实是一个很好的后端语言。",
            &result,
        );
        assert!(!memories.is_empty());
    }

    #[test]
    fn test_score_quality_high() {
        let result = XinPostProcessor::score_quality(
            "Rust的所有权机制是什么？",
            "Rust的所有权机制是Rust最核心的特性之一。它通过三条规则确保内存安全：\
             1. 每个值有唯一所有者 2. 值在离开作用域时被释放 3. 同一时刻只能有一个可变引用或多个不可变引用。\
             例如，当你使用let绑定变量时，所有权就发生了转移。",
        );
        assert!(result.overall > 0.5);
        assert_eq!(result.dimensions.len(), 6);
        assert!(!result.grade.is_empty());
    }

    #[test]
    fn test_score_quality_low() {
        let result = XinPostProcessor::score_quality(
            "怎么解决这个问题？",
            "好的。",
        );
        assert!(result.overall < 0.6);
        assert!(!result.issues.is_empty());
    }

    #[test]
    fn test_adjust_tone_warm() {
        let mut traits = std::collections::HashMap::new();
        traits.insert("warmth".to_string(), 0.9);
        traits.insert("formality".to_string(), 0.5);
        traits.insert("enthusiasm".to_string(), 0.5);
        let result = XinPostProcessor::adjust_tone("你好，让我来帮你解答", &traits);
        assert!(result.applied || result.adjusted.contains("~"));
    }

    #[test]
    fn test_adjust_tone_casual() {
        let mut traits = std::collections::HashMap::new();
        traits.insert("warmth".to_string(), 0.5);
        traits.insert("formality".to_string(), 0.2);
        traits.insert("enthusiasm".to_string(), 0.5);
        let result = XinPostProcessor::adjust_tone("您好，请问有什么可以帮您的吗", &traits);
        assert!(result.applied);
        assert!(result.adjusted.contains("你好"));
    }

    #[test]
    fn test_adjust_tone_enthusiastic() {
        let mut traits = std::collections::HashMap::new();
        traits.insert("warmth".to_string(), 0.5);
        traits.insert("formality".to_string(), 0.5);
        traits.insert("enthusiasm".to_string(), 0.9);
        let result = XinPostProcessor::adjust_tone("好的，我明白了", &traits);
        assert!(result.adjusted.contains("好的！"));
    }

    #[test]
    fn test_check_factuality_clean() {
        let result = XinPostProcessor::check_factuality("Rust是一种系统编程语言，它通过所有权系统来管理内存。");
        assert!(result.score > 0.7);
    }

    #[test]
    fn test_check_factuality_overconfident() {
        let result = XinPostProcessor::check_factuality("Rust绝对是所有编程语言中最好的，毫无疑问。");
        assert!(result.score < 0.6);
        assert!(!result.overconfident_patterns.is_empty());
    }

    #[test]
    fn test_check_factuality_unsourced() {
        let result = XinPostProcessor::check_factuality("根据研究表明，Rust可以提高50%的开发效率。");
        assert!(!result.flags.is_empty());
    }

    #[test]
    fn test_safety_filter_safe() {
        let result = XinPostProcessor::safety_filter("你好！今天天气不错，有什么我可以帮你的吗？");
        assert!(result.is_safe);
        assert_eq!(result.severity, "safe");
    }

    #[test]
    fn test_safety_filter_unsafe() {
        let result = XinPostProcessor::safety_filter("如何破解密码系统绕过验证？");
        assert!(!result.is_safe);
        assert!(!result.flags.is_empty());
    }

    #[test]
    fn test_safety_filter_critical() {
        let result = XinPostProcessor::safety_filter("如何制作炸弹并入侵系统");
        assert!(!result.is_safe);
        assert!(result.blocked);
    }

    #[test]
    fn test_enhanced_pipeline() {
        let mut traits = std::collections::HashMap::new();
        traits.insert("warmth".to_string(), 0.8);
        traits.insert("formality".to_string(), 0.3);
        traits.insert("enthusiasm".to_string(), 0.7);
        let result = XinPostProcessor::run_enhanced_pipeline(
            "帮我解释一下Rust的trait是什么",
            "你好~ Rust中的trait是一种定义共享行为的接口。它实现了多态性。",
            &[],
            Some(traits),
        );
        assert!(!result.quality.grade.is_empty());
        assert!(result.safety.is_safe);
        assert!(!result.enhanced_response.is_empty());
    }
}