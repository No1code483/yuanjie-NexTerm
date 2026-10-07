use chrono::{DateTime, Utc};
#[cfg(test)]
use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::models::xin::{MemoryCategory, MemoryCluster, UserMemory};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamConfig {
    pub enabled: bool,
    pub timezone: Option<String>,
    pub verbose: bool,
    pub phases: DreamPhasesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamPhasesConfig {
    pub light: LightDreamConfig,
    pub deep: DeepDreamConfig,
    pub rem: RemDreamConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightDreamConfig {
    pub enabled: bool,
    pub interval_hours: u32,
    pub lookback_hours: u32,
    pub max_candidates: usize,
    pub dedupe_similarity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepDreamConfig {
    pub enabled: bool,
    pub interval_hours: u32,
    pub max_promotions: usize,
    pub min_score: f64,
    pub min_recall_count: u32,
    pub recency_half_life_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemDreamConfig {
    pub enabled: bool,
    pub interval_hours: u32,
    pub lookback_days: u32,
    pub min_pattern_strength: f64,
}

impl Default for DreamConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timezone: None,
            verbose: false,
            phases: DreamPhasesConfig {
                light: LightDreamConfig {
                    enabled: true,
                    interval_hours: 6,
                    lookback_hours: 12,
                    max_candidates: 100,
                    dedupe_similarity: 0.9,
                },
                deep: DeepDreamConfig {
                    enabled: true,
                    interval_hours: 24,
                    max_promotions: 10,
                    min_score: 0.8,
                    min_recall_count: 3,
                    recency_half_life_days: 14,
                },
                rem: RemDreamConfig {
                    enabled: true,
                    interval_hours: 168,
                    lookback_days: 30,
                    min_pattern_strength: 0.75,
                },
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamState {
    pub last_light_at: Option<DateTime<Utc>>,
    pub last_deep_at: Option<DateTime<Utc>>,
    pub last_rem_at: Option<DateTime<Utc>>,
    pub light_candidates_count: usize,
    pub deep_promotions_count: usize,
    pub rem_patterns_count: usize,
}

impl Default for DreamState {
    fn default() -> Self {
        Self {
            last_light_at: None,
            last_deep_at: None,
            last_rem_at: None,
            light_candidates_count: 0,
            deep_promotions_count: 0,
            rem_patterns_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamCandidate {
    pub key: String,
    pub value: String,
    pub category: String,
    pub source: String,
    pub confidence: f64,
    pub importance: f64,
    pub keywords: Vec<String>,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightDreamResult {
    pub candidates: Vec<DreamCandidate>,
    pub deduped_count: usize,
    pub sources_processed: usize,
    pub duration_ms: u64,
    pub performed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepDreamResult {
    pub promoted: usize,
    pub clusters: Vec<MemoryCluster>,
    pub health_score: f64,
    pub health_status: String,
    pub duration_ms: u64,
    pub performed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemDreamResult {
    pub patterns: Vec<DreamPattern>,
    pub personality_suggestions: Vec<String>,
    pub dominant_categories: HashMap<String, f64>,
    pub duration_ms: u64,
    pub performed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamPattern {
    pub theme: String,
    pub description: String,
    pub strength: f64,
    pub supporting_memory_count: usize,
    pub sample_memory_keys: Vec<String>,
}

pub struct XinDreamService;

impl XinDreamService {
    pub fn check_dreaming_due(phase: &str, state: &DreamState, config: &DreamConfig) -> bool {
        if !config.enabled {
            return false;
        }
        let now = Utc::now();
        let (last_at, interval_hours, phase_enabled) = match phase {
            "light" => (
                state.last_light_at,
                config.phases.light.interval_hours,
                config.phases.light.enabled,
            ),
            "deep" => (
                state.last_deep_at,
                config.phases.deep.interval_hours,
                config.phases.deep.enabled,
            ),
            "rem" => (
                state.last_rem_at,
                config.phases.rem.interval_hours,
                config.phases.rem.enabled,
            ),
            _ => return false,
        };
        if !phase_enabled {
            return false;
        }
        match last_at {
            None => true,
            Some(last) => {
                let elapsed = now - last;
                elapsed.num_hours() as u32 >= interval_hours
            }
        }
    }

    pub fn run_light_dreaming(
        memories: &[UserMemory],
        conversation_texts: &[String],
        config: &LightDreamConfig,
    ) -> LightDreamResult {
        let started = std::time::Instant::now();
        let mut candidates = Vec::new();

        for text in conversation_texts.iter().take(10) {
            let keywords = Self::extract_keywords(text);
            if keywords.is_empty() {
                continue;
            }
            let key = keywords.first().cloned().unwrap_or_default();
            let value = format!("对话中提到: {}", Self::summarize_text(text, 100));
            let candidate = DreamCandidate {
                key,
                value,
                category: "fact".to_string(),
                source: "light_dream".to_string(),
                confidence: Self::calc_confidence(&keywords, text),
                importance: Self::calc_importance(&keywords),
                keywords,
                timestamp_ms: Utc::now().timestamp_millis(),
            };
            candidates.push(candidate);
        }

        candidates.truncate(config.max_candidates);
        let before_dedupe = candidates.len();

        let candidates = Self::deduplicate_candidates(candidates, config.dedupe_similarity);
        let deduped_count = before_dedupe - candidates.len();

        for candidate in &candidates {
            if let Some(_existing) = memories.iter().find(|m| {
                Self::jaccard_similarity(&m.key, &candidate.key) > config.dedupe_similarity
            }) {
                continue;
            }
        }

        LightDreamResult {
            candidates,
            deduped_count,
            sources_processed: conversation_texts.len(),
            duration_ms: started.elapsed().as_millis() as u64,
            performed_at: Utc::now(),
        }
    }

    pub fn run_deep_dreaming(
        candidates: &[DreamCandidate],
        memories: &[UserMemory],
        config: &DeepDreamConfig,
    ) -> DeepDreamResult {
        let started = std::time::Instant::now();
        let now_ms = Utc::now().timestamp_millis();

        let scored: Vec<(&DreamCandidate, f64)> = candidates
            .iter()
            .filter_map(|c| {
                let recall_count = memories
                    .iter()
                    .filter(|m| {
                        Self::jaccard_similarity(&m.key, &c.key) > 0.3
                    })
                    .count();

                if recall_count < config.min_recall_count as usize {
                    return None;
                }

                let age_days = ((now_ms - c.timestamp_ms) as f64) / (1000.0 * 86400.0);
                let recency = 2.0_f64.powf(-age_days / config.recency_half_life_days as f64);
                let score = recency * 0.3 + c.importance * 0.4 + c.confidence * 0.3;

                if score < config.min_score {
                    return None;
                }

                Some((c, score))
            })
            .collect();

        let mut scored = scored;
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let promoted_count = scored.len().min(config.max_promotions);

        let clusters = Self::cluster_by_theme(
            &scored[..promoted_count]
                .iter()
                .map(|(c, _)| (*c).clone())
                .collect::<Vec<_>>(),
        );

        let health_score = Self::calc_memory_health(memories);
        let health_status = if health_score >= 0.7 {
            "healthy".to_string()
        } else if health_score >= 0.4 {
            "moderate".to_string()
        } else {
            "low".to_string()
        };

        DeepDreamResult {
            promoted: promoted_count,
            clusters,
            health_score,
            health_status,
            duration_ms: started.elapsed().as_millis() as u64,
            performed_at: Utc::now(),
        }
    }

    pub fn run_rem_dreaming(
        memories: &[UserMemory],
        _clusters: &[MemoryCluster],
        config: &RemDreamConfig,
    ) -> RemDreamResult {
        let started = std::time::Instant::now();

        let mut category_counts: HashMap<String, usize> = HashMap::new();
        for memory in memories {
            let cat = format!("{:?}", memory.category);
            *category_counts.entry(cat).or_default() += 1;
        }
        let total = memories.len().max(1) as f64;
        let dominant_categories: HashMap<String, f64> = category_counts
            .iter()
            .map(|(k, v)| (k.clone(), *v as f64 / total))
            .collect();

        let mut patterns = Vec::new();
        let mut personality_suggestions = Vec::new();

        if let Some(fact_pct) = dominant_categories.get("Fact") {
            if *fact_pct > 0.5 {
                patterns.push(DreamPattern {
                    theme: "信息收集型交互".to_string(),
                    description: "用户倾向于事实型对话，适合知识整理风格".to_string(),
                    strength: *fact_pct,
                    supporting_memory_count: (total * fact_pct) as usize,
                    sample_memory_keys: memories
                        .iter()
                        .filter(|m| m.category == MemoryCategory::Fact)
                        .take(3)
                        .map(|m| m.key.clone())
                        .collect(),
                });
            }
        }

        if let Some(pref_pct) = dominant_categories.get("Preference") {
            if *pref_pct > 0.15 {
                patterns.push(DreamPattern {
                    theme: "偏好敏感型".to_string(),
                    description: "用户有明确的偏好模式，适合个性化服务风格".to_string(),
                    strength: *pref_pct,
                    supporting_memory_count: (total * pref_pct) as usize,
                    sample_memory_keys: memories
                        .iter()
                        .filter(|m| m.category == MemoryCategory::Preference)
                        .take(3)
                        .map(|m| m.key.clone())
                        .collect(),
                });
                personality_suggestions.push("增加个性化记忆引用频率".to_string());
            }
        }

        if let Some(habit_pct) = dominant_categories.get("Habit") {
            if *habit_pct > 0.1 {
                personality_suggestions.push("启用习惯追踪与提醒功能".to_string());
            }
        }

        let memory_age_days =
            if let Some(oldest) = memories.iter().min_by_key(|m| &m.created_at) {
                let created = DateTime::parse_from_rfc3339(&oldest.created_at)
                    .map(|dt| (Utc::now() - dt.with_timezone(&Utc)).num_days())
                    .unwrap_or(0);
                created
            } else {
                0
            };

        if total > 20.0 && memory_age_days > 7 {
            patterns.push(DreamPattern {
                theme: "长期互动关系".to_string(),
                description: format!(
                    "已积累 {} 条记忆，持续 {} 天，适合深化陪伴模式",
                    total as usize, memory_age_days
                ),
                strength: (total / 50.0).min(1.0),
                supporting_memory_count: memories.len(),
                sample_memory_keys: memories.iter().take(3).map(|m| m.key.clone()).collect(),
            });
        }

        patterns.retain(|p| p.strength >= config.min_pattern_strength);

        RemDreamResult {
            patterns,
            personality_suggestions,
            dominant_categories,
            duration_ms: started.elapsed().as_millis() as u64,
            performed_at: Utc::now(),
        }
    }

    pub fn calc_memory_health(memories: &[UserMemory]) -> f64 {
        if memories.is_empty() {
            return 0.0;
        }

        let total = memories.len() as f64;

        let recent_count = memories
            .iter()
            .filter(|m| {
                if let Ok(dt) = DateTime::parse_from_rfc3339(&m.created_at) {
                    let age = Utc::now() - dt.with_timezone(&Utc);
                    age.num_days() < 7
                } else {
                    false
                }
            })
            .count() as f64;

        let high_importance = memories
            .iter()
            .filter(|m| m.importance >= 0.6)
            .count() as f64;

        let categories = memories
            .iter()
            .map(|m| format!("{:?}", m.category))
            .collect::<HashSet<_>>();
        let category_diversity = categories.len() as f64 / 5.0;

        let recency_score = (recent_count / total.max(1.0)).min(1.0);
        let importance_score = (high_importance / total.max(1.0)).min(1.0);

        recency_score * 0.35 + importance_score * 0.35 + category_diversity.min(1.0) * 0.3
    }

    pub fn find_recovery_candidates(
        _memories: &[UserMemory],
        mut candidates: Vec<DreamCandidate>,
        min_confidence: f64,
    ) -> Vec<DreamCandidate> {
        candidates.retain(|c| c.confidence >= min_confidence);
        candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).expect("confidence should not be NaN"));
        candidates
    }

    fn extract_keywords(text: &str) -> Vec<String> {
        let stopwords: HashSet<&str> = [
            "的", "了", "在", "是", "我", "有", "和", "就", "不", "人", "都", "一", "一个",
            "上", "也", "很", "到", "说", "要", "去", "你", "会", "着", "没", "看", "好",
            "自己", "这", "他", "她", "它", "们", "那", "什么", "怎么", "哪", "吗", "呢",
            "啊", "吧", "哦", "嗯", "哈", "the", "a", "an", "is", "are", "was", "were",
            "be", "been", "being", "have", "has", "had", "do", "does", "did", "will",
            "would", "could", "should", "may", "might", "can", "shall", "to", "of", "in",
            "for", "on", "with", "at", "by", "from", "as", "into", "through", "during",
            "before", "after", "above", "below", "between", "and", "but", "or", "not",
            "this", "that", "these", "those", "it", "its", "i", "me", "my", "we", "our",
            "you", "your", "he", "him", "his", "she", "her", "they", "them", "their",
        ]
        .iter()
        .copied()
        .collect();

        let mut word_freq: HashMap<String, usize> = HashMap::new();
        for word in text.split(|c: char| !c.is_alphanumeric() && c != '-') {
            let w = word.trim().to_lowercase();
            if w.len() < 2 || stopwords.contains(w.as_str()) || w.chars().all(|c| c.is_numeric()) {
                continue;
            }
            *word_freq.entry(w).or_default() += 1;
        }

        let mut pairs: Vec<(String, usize)> = word_freq.into_iter().collect();
        pairs.sort_by(|a, b| b.1.cmp(&a.1));
        pairs.truncate(10);
        pairs.into_iter().map(|(k, _)| k).collect()
    }

    fn summarize_text(text: &str, max_len: usize) -> String {
        let cleaned: String = text
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
            .collect();
        let trimmed = cleaned.trim();
        if trimmed.len() <= max_len {
            trimmed.to_string()
        } else {
            let mut end = max_len;
            while end > 0 && !trimmed.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...", &trimmed[..end])
        }
    }

    fn calc_confidence(keywords: &[String], text: &str) -> f64 {
        if keywords.is_empty() {
            return 0.0;
        }
        let keyword_hits = keywords
            .iter()
            .filter(|kw| text.to_lowercase().contains(&kw.to_lowercase()))
            .count();
        let base = keyword_hits as f64 / keywords.len() as f64;
        let length_bonus = (text.len() as f64 / 500.0).min(0.3);
        (base * 0.7 + length_bonus).min(1.0)
    }

    fn calc_importance(keywords: &[String]) -> f64 {
        if keywords.is_empty() {
            return 0.0;
        }
        let high_value: HashSet<&str> = [
            "bug", "error", "fix", "重要", "紧急", "deadline", "密码", "secret",
            "key", "token", "关键", "事故", "安全", "漏洞", "crash",
        ]
        .iter()
        .copied()
        .collect();
        let hits = keywords
            .iter()
            .filter(|kw| high_value.contains(kw.as_str()))
            .count();
        let base = hits as f64 / keywords.len().max(1) as f64;
        (base * 1.5).min(1.0).max(0.1)
    }

    fn jaccard_similarity(a: &str, b: &str) -> f64 {
        let set_a: HashSet<char> = a.chars().collect();
        let set_b: HashSet<char> = b.chars().collect();
        if set_a.is_empty() && set_b.is_empty() {
            return 1.0;
        }
        let intersection = set_a.intersection(&set_b).count();
        let union = set_a.union(&set_b).count();
        intersection as f64 / union as f64
    }

    fn deduplicate_candidates(
        mut candidates: Vec<DreamCandidate>,
        threshold: f64,
    ) -> Vec<DreamCandidate> {
        let mut result: Vec<DreamCandidate> = Vec::new();
        candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).expect("confidence should not be NaN"));

        for candidate in candidates {
            let is_dup = result.iter().any(|existing| {
                Self::jaccard_similarity(&existing.value, &candidate.value) > threshold
                    || Self::jaccard_similarity(&existing.key, &candidate.key) > threshold
            });
            if !is_dup {
                result.push(candidate);
            }
        }
        result
    }

    fn cluster_by_theme(candidates: &[DreamCandidate]) -> Vec<MemoryCluster> {
        let mut clusters: Vec<MemoryCluster> = Vec::new();
        let mut used: HashSet<usize> = HashSet::new();

        for (i, base) in candidates.iter().enumerate() {
            if used.contains(&i) {
                continue;
            }
            let mut group: Vec<usize> = vec![i];
            for (j, other) in candidates.iter().enumerate().skip(i + 1) {
                if used.contains(&j) {
                    continue;
                }
                let kw_overlap = base
                    .keywords
                    .iter()
                    .filter(|kw| other.keywords.contains(kw))
                    .count();
                if kw_overlap >= 1 {
                    group.push(j);
                    used.insert(j);
                }
            }
            used.insert(i);

            let theme = if let Some(kw) = base.keywords.first() {
                kw.clone()
            } else {
                "未分类".to_string()
            };

            let summary = format!(
                "{} - 共 {} 条相关记忆",
                base.value,
                group.len()
            );

            clusters.push(MemoryCluster {
                cluster_id: format!("cluster-{}", clusters.len()),
                theme,
                memory_ids: group.iter().map(|_| uuid::Uuid::new_v4().to_string()).collect(),
                summary,
                importance_score: group.iter().map(|_| base.importance).sum::<f64>()
                    / group.len().max(1) as f64,
            });
        }

        clusters
    }

    pub fn auto_promote_to_memories(candidates: &[DreamCandidate]) -> Vec<UserMemory> {
        let now = Utc::now().to_rfc3339();
        candidates
            .iter()
            .map(|c| UserMemory {
                id: uuid::Uuid::new_v4().to_string(),
                category: Self::infer_memory_category(&c.keywords),
                key: c.key.clone(),
                value: c.value.clone(),
                importance: c.importance,
                source: "dream_promoted".to_string(),
                confidence: c.confidence,
                created_at: now.clone(),
                last_recalled_at: None,
            })
            .collect()
    }

    fn infer_memory_category(keywords: &[String]) -> MemoryCategory {
        let pref_indicators: HashSet<&str> = ["喜欢", "偏好", "常用", "习惯", "prefer"]
            .iter()
            .copied()
            .collect();
        let habit_indicators: HashSet<&str> = ["每天", "每周", "总是", "经常", "daily", "routine"]
            .iter()
            .copied()
            .collect();
        let experience_indicators: HashSet<&str> = ["上次", "之前", "曾经", "经历过", "过去"]
            .iter()
            .copied()
            .collect();

        let joined: HashSet<&str> = keywords.iter().map(|s| s.as_str()).collect();

        if joined.intersection(&pref_indicators).count() > 0 {
            MemoryCategory::Preference
        } else if joined.intersection(&habit_indicators).count() > 0 {
            MemoryCategory::Habit
        } else if joined.intersection(&experience_indicators).count() > 0 {
            MemoryCategory::Experience
        } else {
            MemoryCategory::Fact
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_memory(key: &str, value: &str, importance: f64, category: MemoryCategory) -> UserMemory {
        UserMemory {
            id: uuid::Uuid::new_v4().to_string(),
            category,
            key: key.to_string(),
            value: value.to_string(),
            importance,
            source: "test".to_string(),
            confidence: 0.8,
            created_at: Utc::now().to_rfc3339(),
            last_recalled_at: None,
        }
    }

    #[test]
    fn test_dreaming_due_first_time() {
        let state = DreamState::default();
        let config = DreamConfig::default();
        assert!(XinDreamService::check_dreaming_due("light", &state, &config));
    }

    #[test]
    fn test_dreaming_not_due() {
        let state = DreamState {
            last_light_at: Some(Utc::now() - TimeDelta::hours(1)),
            ..Default::default()
        };
        let config = DreamConfig::default();
        assert!(!XinDreamService::check_dreaming_due("light", &state, &config));
    }

    #[test]
    fn test_dreaming_due_after_interval() {
        let state = DreamState {
            last_light_at: Some(Utc::now() - TimeDelta::hours(7)),
            ..Default::default()
        };
        let config = DreamConfig::default();
        assert!(XinDreamService::check_dreaming_due("light", &state, &config));
    }

    #[test]
    fn test_disabled_dreaming() {
        let state = DreamState::default();
        let config = DreamConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(!XinDreamService::check_dreaming_due("light", &state, &config));
    }

    #[test]
    fn test_disabled_phase() {
        let state = DreamState::default();
        let mut config = DreamConfig::default();
        config.phases.deep.enabled = false;
        assert!(!XinDreamService::check_dreaming_due("deep", &state, &config));
    }

    #[test]
    fn test_extract_keywords() {
        let text = "我今天学习了Rust编程语言，感觉非常有趣也很实用";
        let kw = XinDreamService::extract_keywords(text);
        assert!(!kw.is_empty());
        assert!(kw.iter().any(|k| k.contains("rust") || k.contains("编程") || k.contains("实用")));
    }

    #[test]
    fn test_extract_keywords_english() {
        let text = "I learned about memory dreaming systems and pattern discovery today";
        let kw = XinDreamService::extract_keywords(text);
        assert!(kw.iter().any(|k| k.contains("memory") || k.contains("dreaming") || k.contains("pattern")));
    }

    #[test]
    fn test_jaccard_similarity() {
        let sim = XinDreamService::jaccard_similarity("hello", "hello");
        assert!((sim - 1.0).abs() < 0.01);

        let sim = XinDreamService::jaccard_similarity("abc", "xyz");
        assert!((sim - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_light_dreaming() {
        let conversations = vec![
            "我今天学习了Rust编程，感觉非常有趣".to_string(),
            "Python 的异步编程也很强大".to_string(),
            "我在用 Tauri 构建桌面应用".to_string(),
        ];
        let config = LightDreamConfig {
            enabled: true,
            interval_hours: 6,
            lookback_hours: 12,
            max_candidates: 10,
            dedupe_similarity: 0.9,
        };
        let result = XinDreamService::run_light_dreaming(&[], &conversations, &config);
        assert!(!result.candidates.is_empty());
        assert!(result.candidates.len() <= 10);
    }

    #[test]
    fn test_deep_dreaming() {
        let candidates = vec![DreamCandidate {
            key: "rust编程".to_string(),
            value: "用户在学习Rust编程".to_string(),
            category: "fact".to_string(),
            source: "light".to_string(),
            confidence: 0.9,
            importance: 0.85,
            keywords: vec!["rust".to_string(), "编程".to_string(), "学习".to_string()],
            timestamp_ms: Utc::now().timestamp_millis(),
        }];
        let memories = vec![
            make_memory("rust学习", "用户对Rust感兴趣", 0.8, MemoryCategory::Fact),
            make_memory("编程偏好", "用户喜欢系统编程", 0.7, MemoryCategory::Preference),
            make_memory("工具使用", "用户使用VS Code", 0.6, MemoryCategory::Fact),
            make_memory("项目经历", "用户做过Web项目", 0.5, MemoryCategory::Experience),
        ];
        let config = DeepDreamConfig {
            enabled: true,
            interval_hours: 24,
            max_promotions: 10,
            min_score: 0.5,
            min_recall_count: 1,
            recency_half_life_days: 14,
        };
        let result = XinDreamService::run_deep_dreaming(&candidates, &memories, &config);
        assert!(result.promoted > 0);
        assert!(result.health_score >= 0.0 && result.health_score <= 1.0);
    }

    #[test]
    fn test_calc_memory_health() {
        let mems = vec![
            make_memory("a", "v1", 0.9, MemoryCategory::Fact),
            make_memory("b", "v2", 0.8, MemoryCategory::Preference),
            make_memory("c", "v3", 0.3, MemoryCategory::Experience),
        ];
        let score = XinDreamService::calc_memory_health(&mems);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_empty_memory_health() {
        let score = XinDreamService::calc_memory_health(&[]);
        assert!((score - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_rem_dreaming() {
        let memories = vec![
            make_memory("a", "v1", 0.9, MemoryCategory::Fact),
            make_memory("b", "v2", 0.8, MemoryCategory::Fact),
            make_memory("c", "v3", 0.7, MemoryCategory::Fact),
            make_memory("d", "v4", 0.6, MemoryCategory::Preference),
            make_memory("e", "v5", 0.5, MemoryCategory::Habit),
        ];
        let config = RemDreamConfig {
            enabled: true,
            interval_hours: 168,
            lookback_days: 30,
            min_pattern_strength: 0.4,
        };
        let result = XinDreamService::run_rem_dreaming(&memories, &[], &config);
        assert!(!result.dominant_categories.is_empty());
    }

    #[test]
    fn test_auto_promote_to_memories() {
        let candidates = vec![DreamCandidate {
            key: "rust学习".to_string(),
            value: "用户在学习Rust编程".to_string(),
            category: "fact".to_string(),
            source: "light".to_string(),
            confidence: 0.9,
            importance: 0.85,
            keywords: vec!["rust".to_string(), "编程".to_string()],
            timestamp_ms: Utc::now().timestamp_millis(),
        }];
        let mems = XinDreamService::auto_promote_to_memories(&candidates);
        assert_eq!(mems.len(), 1);
        assert_eq!(mems[0].source, "dream_promoted");
    }

    #[test]
    fn test_deduplicate_candidates() {
        let candidates = vec![
            DreamCandidate {
                key: "rust学习".to_string(),
                value: "用户在学习Rust编程语言".to_string(),
                category: "fact".to_string(),
                source: "light".to_string(),
                confidence: 0.9,
                importance: 0.85,
                keywords: vec!["rust".to_string()],
                timestamp_ms: 1,
            },
            DreamCandidate {
                key: "rust学习1".to_string(),
                value: "用户在学习Rust编程语言中".to_string(),
                category: "fact".to_string(),
                source: "light".to_string(),
                confidence: 0.8,
                importance: 0.75,
                keywords: vec!["rust".to_string()],
                timestamp_ms: 2,
            },
        ];
        let result = XinDreamService::deduplicate_candidates(candidates, 0.7);
        assert_eq!(result.len(), 1);
    }
}