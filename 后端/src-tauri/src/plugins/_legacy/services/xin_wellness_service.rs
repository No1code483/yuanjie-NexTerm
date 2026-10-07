use std::collections::HashMap;

use crate::models::xin::{
    ActivityDigest, ConversationBridge, DigestStats, EvolutionEntry,
    HabitCheckin, LinkedSession, MemoryCluster, MemoryConsolidation, MemoryLink, Mood,
    MoodEntry, MoodHistory, PersonalityEvolution, PomodoroSession, Reminder, TraitDrift,
    UserMemory,
};

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub struct MoodTracker {
    history: Vec<MoodEntry>,
}

impl MoodTracker {
    pub fn new() -> Self {
        Self { history: Vec::new() }
    }

    pub fn record(&mut self, mood: Mood, context: Option<String>) {
        let entry = MoodEntry {
            mood,
            timestamp: now_iso(),
            context,
        };
        self.history.push(entry);
        if self.history.len() > 100 {
            self.history.remove(0);
        }
    }

    pub fn analyze(&self) -> MoodHistory {
        if self.history.is_empty() {
            return MoodHistory {
                entries: vec![],
                trend: "stable".into(),
                dominant_emotion: "neutral".into(),
                stability_score: 1.0,
            };
        }

        let mut emotion_counts: HashMap<String, usize> = HashMap::new();
        for entry in &self.history {
            *emotion_counts.entry(entry.mood.category.clone()).or_insert(0) += 1;
        }

        let dominant = emotion_counts
            .iter()
            .max_by_key(|(_, c)| *c)
            .map(|(e, _)| e.clone())
            .unwrap_or_else(|| "neutral".into());

        let total = self.history.len() as f64;
        let dominant_ratio = *emotion_counts.get(&dominant).unwrap_or(&0) as f64 / total;
        let stability_score = dominant_ratio;

        let trend = if self.history.len() >= 3 {
            let recent = &self.history[self.history.len() - 3..];
            let mut rising = 0;
            for i in 1..recent.len() {
                if recent[i].mood.intensity > recent[i - 1].mood.intensity {
                    rising += 1;
                }
            }
            if rising >= 2 {
                "improving"
            } else if rising == 0 {
                "declining"
            } else {
                "stable"
            }
        } else {
            "stable"
        };

        MoodHistory {
            entries: self.history.clone(),
            trend: trend.into(),
            dominant_emotion: dominant,
            stability_score,
        }
    }
}

pub struct MemoryConsolidator;

impl MemoryConsolidator {
    pub fn consolidate(memories: &[UserMemory]) -> MemoryConsolidation {
        let total = memories.len();
        if total == 0 {
            return MemoryConsolidation {
                clusters: vec![],
                total_memories: 0,
                auto_generated_summaries: vec![],
                performed_at: now_iso(),
            };
        }

        let mut keyword_groups: HashMap<String, Vec<String>> = HashMap::new();

        for m in memories {
            let keywords: Vec<&str> = m
                .value
                .split(|c: char| !c.is_alphanumeric() && c != '-')
                .filter(|w| w.len() >= 3)
                .take(5)
                .collect();

            for kw in keywords {
                let kw_lower = kw.to_lowercase();
                keyword_groups
                    .entry(kw_lower)
                    .or_default()
                    .push(m.id.clone());
            }
        }

        let mut clusters: Vec<MemoryCluster> = keyword_groups
            .into_iter()
            .filter(|(_, ids)| ids.len() >= 2)
            .map(|(keyword, memory_ids)| {
                let theme = keyword.clone();
                let importance = (memory_ids.len() as f64 / total.max(1) as f64).min(1.0);
                MemoryCluster {
                    cluster_id: uuid_v4(),
                    theme: theme.clone(),
                    memory_ids: memory_ids.clone(),
                    summary: format!(
                        "{}条相关记忆围绕主题「{}」",
                        memory_ids.len(),
                        theme
                    ),
                    importance_score: importance,
                }
            })
            .collect();

        clusters.sort_by(|a, b| b.importance_score.partial_cmp(&a.importance_score).unwrap_or(std::cmp::Ordering::Equal));
        clusters.truncate(10);

        let summaries: Vec<String> = memories
            .iter()
            .filter(|m| m.importance >= 0.7)
            .map(|m| format!("[{}] {}", m.category_name(), m.value))
            .take(5)
            .collect();

        MemoryConsolidation {
            clusters,
            total_memories: total,
            auto_generated_summaries: summaries,
            performed_at: now_iso(),
        }
    }

    pub fn generate_links(memories: &[UserMemory]) -> Vec<MemoryLink> {
        let mut links = Vec::new();
        for (i, a) in memories.iter().enumerate() {
            for b in memories.iter().skip(i + 1) {
                let overlap = word_overlap(&a.value, &b.value);
                if overlap > 0.3 {
                    links.push(MemoryLink {
                        source_id: a.id.clone(),
                        target_id: b.id.clone(),
                        relation_type: "semantic".into(),
                        strength: (overlap * 100.0).round() / 100.0,
                    });
                }
            }
        }
        links.sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap_or(std::cmp::Ordering::Equal));
        links.truncate(50);
        links
    }
}

fn word_overlap(a: &str, b: &str) -> f64 {
    let words_a: Vec<&str> = a.split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() >= 2).collect();
    let words_b: Vec<&str> = b.split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() >= 2).collect();

    if words_a.is_empty() || words_b.is_empty() {
        return 0.0;
    }

    let mut matches = 0;
    for wa in &words_a {
        let wa_l = wa.to_lowercase();
        for wb in &words_b {
            if wa_l == wb.to_lowercase() {
                matches += 1;
                break;
            }
        }
    }

    matches as f64 / words_a.len().max(words_b.len()) as f64
}

pub struct ConversationBridgeBuilder;

impl ConversationBridgeBuilder {
    pub fn build(
        current_session_id: Option<String>,
        recent_summaries: &[crate::models::xin::ConversationSummary],
    ) -> ConversationBridge {
        if recent_summaries.is_empty() {
            return ConversationBridge {
                current_session_id,
                linked_sessions: vec![],
                continuing_topics: vec![],
                context_summary: "无历史会话上下文".into(),
            };
        }

        let mut topic_freq: HashMap<String, usize> = HashMap::new();
        for s in recent_summaries {
            for topic in &s.topics {
                *topic_freq.entry(topic.clone()).or_insert(0) += 1;
            }
        }

        let mut continuing_topics: Vec<String> = topic_freq
            .into_iter()
            .filter(|(_, c)| *c >= 2)
            .map(|(t, _)| t)
            .collect();
        continuing_topics.sort();
        continuing_topics.truncate(10);

        let linked_sessions: Vec<LinkedSession> = recent_summaries
            .iter()
            .take(5)
            .map(|s| {
                let session_id = s
                    .conversation_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| uuid_v4());

                let overlap = if !continuing_topics.is_empty() {
                    let matched = s
                        .topics
                        .iter()
                        .filter(|t| continuing_topics.contains(t))
                        .count();
                    matched as f64 / continuing_topics.len().max(1) as f64
                } else {
                    0.0
                };

                LinkedSession {
                    session_id,
                    title: s.topics.first().cloned().unwrap_or_else(|| "未命名会话".into()),
                    topic_overlap: (overlap * 100.0).round() / 100.0,
                    last_active: s.created_at.clone(),
                    summary: s.summary.clone(),
                }
            })
            .collect();

        let context_summary = if continuing_topics.is_empty() {
            "暂无延续话题".into()
        } else {
            format!(
                "检测到 {} 个延续话题: {}",
                continuing_topics.len(),
                continuing_topics.join("、")
            )
        };

        ConversationBridge {
            current_session_id,
            linked_sessions,
            continuing_topics,
            context_summary,
        }
    }
}

pub struct ReminderManager {
    reminders: Vec<Reminder>,
}

impl ReminderManager {
    pub fn new() -> Self {
        Self {
            reminders: Vec::new(),
        }
    }

    pub fn add_reminder(&mut self, title: String, description: String, trigger_at: Option<String>, cron_expr: Option<String>) -> Reminder {
        let reminder = Reminder {
            id: uuid_v4(),
            title,
            description,
            reminder_type: if cron_expr.is_some() { "recurring" } else { "one_time" }.into(),
            trigger_at,
            cron_expression: cron_expr,
            is_active: true,
            created_at: now_iso(),
            last_triggered_at: None,
            repeat_count: 0,
        };
        self.reminders.push(reminder.clone());
        reminder
    }

    pub fn list_active(&self) -> Vec<Reminder> {
        self.reminders
            .iter()
            .filter(|r| r.is_active)
            .cloned()
            .collect()
    }

    pub fn dismiss(&mut self, id: &str) -> bool {
        if let Some(r) = self.reminders.iter_mut().find(|r| r.id == id) {
            r.is_active = false;
            true
        } else {
            false
        }
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let len = self.reminders.len();
        self.reminders.retain(|r| r.id != id);
        self.reminders.len() < len
    }
}

pub struct HabitTracker {
    habits: Vec<HabitCheckin>,
}

impl HabitTracker {
    pub fn new() -> Self {
        Self { habits: Vec::new() }
    }

    pub fn register(&mut self, name: String, category: String) -> HabitCheckin {
        let habit = HabitCheckin {
            id: uuid_v4(),
            habit_name: name,
            streak_days: 0,
            total_checkins: 0,
            last_checkin: None,
            completed_today: false,
            category,
        };
        self.habits.push(habit.clone());
        habit
    }

    pub fn checkin(&mut self, habit_id: &str) -> Option<HabitCheckin> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        if let Some(h) = self.habits.iter_mut().find(|h| h.id == habit_id) {
            if h.completed_today {
                return Some(h.clone());
            }
            h.total_checkins += 1;
            h.streak_days += 1;
            h.last_checkin = Some(today.clone());
            h.completed_today = true;
            Some(h.clone())
        } else {
            None
        }
    }

    pub fn list(&self) -> Vec<HabitCheckin> {
        self.habits.clone()
    }

    pub fn reset_daily(&mut self) {
        for h in &mut self.habits {
            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
            if let Some(ref last) = h.last_checkin {
                if last != &today {
                    h.completed_today = false;
                }
            } else {
                h.completed_today = false;
            }
        }
    }
}

pub struct PomodoroManager {
    sessions: Vec<PomodoroSession>,
    active_id: Option<String>,
}

impl PomodoroManager {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            active_id: None,
        }
    }

    pub fn start(
        &mut self,
        task_name: String,
        duration_minutes: u32,
        break_minutes: u32,
        total_cycles: u32,
    ) -> PomodoroSession {
        let id = uuid_v4();
        let session = PomodoroSession {
            id: id.clone(),
            task_name,
            duration_minutes,
            break_minutes,
            cycles_completed: 0,
            total_cycles,
            started_at: now_iso(),
            is_active: true,
            time_remaining_secs: duration_minutes * 60,
        };
        self.sessions.push(session.clone());
        self.active_id = Some(id);
        session
    }

    pub fn complete_cycle(&mut self) -> Option<PomodoroSession> {
        if let Some(ref id) = self.active_id {
            if let Some(s) = self.sessions.iter_mut().find(|s| &s.id == id) {
                s.cycles_completed += 1;
                if s.cycles_completed >= s.total_cycles {
                    s.is_active = false;
                    self.active_id = None;
                } else {
                    s.time_remaining_secs = s.duration_minutes * 60;
                }
                return Some(s.clone());
            }
        }
        None
    }

    pub fn stop(&mut self) -> Option<PomodoroSession> {
        if let Some(ref id) = self.active_id {
            if let Some(s) = self.sessions.iter_mut().find(|s| &s.id == id) {
                s.is_active = false;
                let result = s.clone();
                self.active_id = None;
                return Some(result);
            }
        }
        None
    }

    pub fn active(&self) -> Option<PomodoroSession> {
        self.active_id.as_ref().and_then(|id| {
            self.sessions.iter().find(|s| &s.id == id).cloned()
        })
    }
}

pub struct DigestGenerator;

impl DigestGenerator {
    pub fn generate(
        period: &str,
        memories: &[UserMemory],
        summaries: &[crate::models::xin::ConversationSummary],
        interactions: usize,
    ) -> ActivityDigest {
        let now = chrono::Utc::now();
        let date_range = match period {
            "week" => {
                let start = now - chrono::Duration::days(7);
                format!("{} ~ {}", start.format("%m-%d"), now.format("%m-%d"))
            }
            "month" => {
                let start = now - chrono::Duration::days(30);
                format!("{} ~ {}", start.format("%m-%d"), now.format("%m-%d"))
            }
            _ => now.format("%Y-%m-%d").to_string(),
        };

        let mut highlights = Vec::new();
        if !memories.is_empty() {
            highlights.push(format!("新增 {} 条记忆", memories.len()));
        }
        if !summaries.is_empty() {
            highlights.push(format!("完成 {} 次会话", summaries.len()));
        }
        if interactions > 100 {
            highlights.push(format!("活跃互动 {} 次", interactions));
        }
        if highlights.is_empty() {
            highlights.push("新的一天，继续加油！".into());
        }

        let mut topic_counts: HashMap<String, usize> = HashMap::new();
        for s in summaries {
            for t in &s.topics {
                *topic_counts.entry(t.clone()).or_insert(0) += 1;
            }
        }
        let mut top_topics: Vec<(String, usize)> = topic_counts.into_iter().collect();
        top_topics.sort_by(|a, b| b.1.cmp(&a.1));
        top_topics.truncate(5);
        let top_topic_names: Vec<String> = top_topics.into_iter().map(|(t, _)| t).collect();

        let stats = DigestStats {
            total_interactions: interactions,
            memories_created: memories.len(),
            conversations_completed: summaries.len(),
            active_hours_avg: (interactions.max(1) as f64 * 0.05).min(12.0),
            top_topics: top_topic_names,
            mood_average: "calm".into(),
        };

        ActivityDigest {
            period: period.into(),
            date_range,
            highlights,
            stats_summary: stats,
            generated_at: now_iso(),
        }
    }
}

pub struct PersonalityEvolver;

impl PersonalityEvolver {
    pub fn evolve(
        current_persona: &crate::models::xin::Persona,
        mood_history: &MoodHistory,
        memory_count: usize,
        preferred_topics: &[String],
    ) -> PersonalityEvolution {
        let mut trait_drifts = Vec::new();
        let mut evolution_log = Vec::new();
        let mut recommendation: Option<String> = None;

        let empathy_drift = Self::calc_empathy_drift(current_persona, mood_history);
        if empathy_drift.abs() > 0.1 {
            trait_drifts.push(TraitDrift {
                trait_name: "共情能力".into(),
                original_value: current_persona
                    .traits
                    .iter()
                    .find(|t| t.name == "共情")
                    .map(|t| t.value)
                    .unwrap_or(0.5),
                current_value: (current_persona
                    .traits
                    .iter()
                    .find(|t| t.name == "共情")
                    .map(|t| t.value)
                    .unwrap_or(0.5)
                    + empathy_drift)
                    .clamp(0.0, 1.0),
                drift_direction: if empathy_drift > 0.0 {
                    "increasing"
                } else {
                    "decreasing"
                }
                .into(),
                confidence: empathy_drift.abs(),
            });
        }

        if memory_count > 500 {
            evolution_log.push(EvolutionEntry {
                timestamp: now_iso(),
                previous_persona_id: current_persona.id.clone(),
                new_persona_id: current_persona.id.clone(),
                reason: "积累了大量记忆数据，人格建议更加个性化".into(),
            });
            recommendation = Some("积累了足够的交互数据，可以考虑创建自定义人格".into());
        }

        if mood_history.stability_score < 0.3 && mood_history.entries.len() > 10 {
            let suggested = if preferred_topics.contains(&"技术".to_string()) {
                "knowledge_tutor"
            } else {
                "caring_friend"
            };
            recommendation = Some(format!(
                "情绪波动较大，建议切换到「{}」人格以获得更好的体验",
                suggested
            ));
        }

        PersonalityEvolution {
            current_persona_id: current_persona.id.clone(),
            evolution_log,
            trait_drifts,
            recommendation,
        }
    }

    fn calc_empathy_drift(
        persona: &crate::models::xin::Persona,
        mood_history: &MoodHistory,
    ) -> f64 {
        if mood_history.entries.len() < 5 {
            return 0.0;
        }

        let empathy_trait = persona
            .traits
            .iter()
            .find(|t| t.name == "共情")
            .map(|t| t.value)
            .unwrap_or(0.5);

        let negative_ratio = mood_history
            .entries
            .iter()
            .filter(|e| {
                e.mood.category == "sad" || e.mood.category == "anxious" || e.mood.category == "angry"
            })
            .count() as f64
            / mood_history.entries.len().max(1) as f64;

        if negative_ratio > 0.4 && empathy_trait < 0.9 {
            0.12
        } else if negative_ratio < 0.1 && empathy_trait > 0.3 {
            -0.05
        } else {
            0.0
        }
    }
}

trait MemoryCategoryName {
    fn category_name(&self) -> &str;
}

impl MemoryCategoryName for UserMemory {
    fn category_name(&self) -> &str {
        match self.category {
            crate::models::xin::MemoryCategory::Fact => "事实",
            crate::models::xin::MemoryCategory::Preference => "偏好",
            crate::models::xin::MemoryCategory::Experience => "经验",
            crate::models::xin::MemoryCategory::Knowledge => "知识",
            crate::models::xin::MemoryCategory::Habit => "习惯",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::xin::{
        ConversationSummary, EmotionCategory, EmotionInfo, MemoryCategory, Mood,
        SentimentLabel, SentimentResult, UserMemory,
    };

    fn make_memory(id: &str, value: &str, importance: f64, cat: MemoryCategory) -> UserMemory {
        UserMemory {
            id: id.into(),
            category: cat,
            key: format!("key_{}", id),
            value: value.into(),
            importance,
            source: "test".into(),
            confidence: 0.9,
            created_at: now_iso(),
            last_recalled_at: None,
        }
    }

    fn make_mood(category: &str, intensity: f64) -> Mood {
        Mood {
            category: category.into(),
            intensity,
            updated_at: now_iso(),
            trigger: None,
        }
    }

    fn make_summary(topics: Vec<&str>, summary: &str) -> ConversationSummary {
        ConversationSummary {
            id: uuid_v4(),
            conversation_id: None,
            summary: summary.into(),
            key_takeaways: vec![],
            topics: topics.iter().map(|s| s.to_string()).collect(),
            sentiment: Some(SentimentResult {
                label: SentimentLabel::Neutral,
                score: 0.5,
                emotions: vec![EmotionInfo {
                    category: EmotionCategory::Calm,
                    intensity: 0.5,
                }],
            }),
            created_at: now_iso(),
        }
    }

    #[test]
    fn test_mood_tracker_record_and_analyze() {
        let mut tracker = MoodTracker::new();
        tracker.record(make_mood("happy", 0.8), None);
        tracker.record(make_mood("calm", 0.5), None);
        tracker.record(make_mood("happy", 0.9), None);

        let history = tracker.analyze();
        assert_eq!(history.entries.len(), 3);
        assert_eq!(history.dominant_emotion, "happy");
    }

    #[test]
    fn test_memory_consolidation() {
        let memories = vec![
            make_memory("1", "Rust异步编程模式分析", 0.8, MemoryCategory::Knowledge),
            make_memory("2", "Rust错误处理最佳实践", 0.7, MemoryCategory::Knowledge),
            make_memory("3", "最喜欢的编辑器是VS Code", 0.3, MemoryCategory::Preference),
        ];
        let result = MemoryConsolidator::consolidate(&memories);
        assert_eq!(result.total_memories, 3);
        assert!(!result.auto_generated_summaries.is_empty());
    }

    #[test]
    fn test_memory_links() {
        let memories = vec![
            make_memory("a", "Rust programming language learning", 0.9, MemoryCategory::Knowledge),
            make_memory("b", "Rust programming practice project", 0.8, MemoryCategory::Experience),
            make_memory("c", "Python data analysis pipeline", 0.5, MemoryCategory::Knowledge),
        ];
        let links = MemoryConsolidator::generate_links(&memories);
        assert!(!links.is_empty());
        assert!(links[0].strength > 0.0);
    }

    #[test]
    fn test_conversation_bridge() {
        let summaries = vec![
            make_summary(vec!["Rust", "异步"], "讨论Rust异步编程"),
            make_summary(vec!["Rust", "错误处理"], "分析错误处理模式"),
        ];
        let bridge = ConversationBridgeBuilder::build(Some("session_1".into()), &summaries);
        assert!(bridge.continuing_topics.contains(&"Rust".to_string()));
    }

    #[test]
    fn test_reminder_manager() {
        let mut mgr = ReminderManager::new();
        let r = mgr.add_reminder("喝水".into(), "该休息了".into(), None, Some("0 */1 * * *".into()));
        assert_eq!(r.title, "喝水");
        assert_eq!(mgr.list_active().len(), 1);
        assert!(mgr.dismiss(&r.id));
    }

    #[test]
    fn test_habit_tracker() {
        let mut tracker = HabitTracker::new();
        let h = tracker.register("每日代码提交".into(), "coding".into());
        let checked = tracker.checkin(&h.id).unwrap();
        assert_eq!(checked.streak_days, 1);
        assert!(checked.completed_today);
    }

    #[test]
    fn test_pomodoro() {
        let mut mgr = PomodoroManager::new();
        let session = mgr.start("写代码".into(), 25, 5, 4);
        assert!(session.is_active);
        let completed = mgr.complete_cycle().unwrap();
        assert_eq!(completed.cycles_completed, 1);
        assert!(completed.is_active);
        mgr.stop();
        assert!(mgr.active().is_none());
    }

    #[test]
    fn test_digest_generator() {
        let memories = vec![make_memory("x", "test", 0.5, MemoryCategory::Fact)];
        let summaries = vec![make_summary(vec!["学习"], "学习Rust")];
        let digest = DigestGenerator::generate("week", &memories, &summaries, 150);
        assert_eq!(digest.period, "week");
        assert!(!digest.highlights.is_empty());
        assert!(digest.stats_summary.total_interactions > 0);
    }

    #[test]
    fn test_personality_evolution() {
        let persona = crate::models::xin::Persona {
            id: "code_assistant".into(),
            name: "代码助手".into(),
            description: "test".into(),
            traits: vec![
                crate::models::xin::PersonaTrait { name: "共情".into(), value: 0.5 },
                crate::models::xin::PersonaTrait { name: "逻辑性".into(), value: 0.95 },
            ],
            speaking_style: crate::models::xin::SpeakingStyle::default(),
            base_mood: "calm".into(),
            avatar_emoji: "🤖".into(),
            is_builtin: true,
        };

        let mut tracker = MoodTracker::new();
        for _ in 0..10 {
            tracker.record(make_mood("sad", 0.8), None);
        }
        let history = tracker.analyze();

        let evolution = PersonalityEvolver::evolve(
            &persona,
            &history,
            600,
            &["技术".into()],
        );
        assert!(!evolution.trait_drifts.is_empty());
        assert!(evolution.recommendation.is_some());
    }
}