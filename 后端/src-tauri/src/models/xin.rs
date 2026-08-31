use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Persona {
    pub id: String,
    pub name: String,
    pub description: String,
    pub traits: Vec<PersonaTrait>,
    pub speaking_style: SpeakingStyle,
    pub base_mood: String,
    pub avatar_emoji: String,
    pub is_builtin: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersonaTrait {
    pub name: String,
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpeakingStyle {
    pub formality: f64,
    pub verbosity: f64,
    pub humor: f64,
    pub technical_depth: f64,
    pub empathy: f64,
}

impl Default for SpeakingStyle {
    fn default() -> Self {
        Self {
            formality: 0.5,
            verbosity: 0.5,
            humor: 0.3,
            technical_depth: 0.6,
            empathy: 0.7,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mood {
    pub category: String,
    pub intensity: f64,
    pub updated_at: String,
    pub trigger: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserMemory {
    pub id: String,
    pub category: MemoryCategory,
    pub key: String,
    pub value: String,
    pub importance: f64,
    pub source: String,
    pub confidence: f64,
    pub created_at: String,
    pub last_recalled_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum MemoryCategory {
    #[serde(rename = "fact")]
    Fact,
    #[serde(rename = "preference")]
    Preference,
    #[serde(rename = "experience")]
    Experience,
    #[serde(rename = "knowledge")]
    Knowledge,
    #[serde(rename = "habit")]
    Habit,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversationSummary {
    pub id: String,
    pub conversation_id: Option<i64>,
    pub summary: String,
    pub key_takeaways: Vec<String>,
    pub topics: Vec<String>,
    pub sentiment: Option<SentimentResult>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SentimentResult {
    pub label: SentimentLabel,
    pub score: f64,
    pub emotions: Vec<EmotionInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SentimentLabel {
    #[serde(rename = "positive")]
    Positive,
    #[serde(rename = "negative")]
    Negative,
    #[serde(rename = "neutral")]
    Neutral,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmotionInfo {
    pub category: EmotionCategory,
    pub intensity: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum EmotionCategory {
    #[serde(rename = "happy")]
    Happy,
    #[serde(rename = "sad")]
    Sad,
    #[serde(rename = "angry")]
    Angry,
    #[serde(rename = "surprised")]
    Surprised,
    #[serde(rename = "anxious")]
    Anxious,
    #[serde(rename = "calm")]
    Calm,
    #[serde(rename = "curious")]
    Curious,
    #[serde(rename = "confused")]
    Confused,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TtsStatus {
    pub available: bool,
    pub engine: String,
    pub voices: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TtsSpeakRequest {
    pub text: String,
    pub speed: Option<f64>,
    pub pitch: Option<f64>,
    pub volume: Option<f64>,
    pub voice: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MultimodalInput {
    pub text: Option<String>,
    pub images: Vec<ImageInput>,
    pub attachments: Vec<AttachmentInput>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageInput {
    pub base64_data: String,
    pub mime_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttachmentInput {
    pub name: String,
    pub mime_type: String,
    pub content: Option<String>,
    pub file_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DailyBriefing {
    pub date: String,
    pub greeting: String,
    pub quote_of_the_day: String,
    pub memory_recall: Vec<UserMemory>,
    pub mood_suggestion: String,
    pub focus_suggestion: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersonalityInsight {
    pub dominant_traits: Vec<String>,
    pub communication_style: String,
    pub interest_domains: Vec<String>,
    pub suggested_persona: String,
    pub insights_text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct XinConfig {
    pub active_persona_id: String,
    pub auto_memory_enabled: bool,
    pub memory_retention_days: u32,
    pub sentiment_monitoring: bool,
    pub tts_enabled: bool,
    pub tts_speed: f64,
    pub tts_pitch: f64,
    pub daily_briefing_enabled: bool,
    pub custom_personas: Vec<Persona>,
}

impl Default for XinConfig {
    fn default() -> Self {
        Self {
            active_persona_id: "code_assistant".into(),
            auto_memory_enabled: true,
            memory_retention_days: 90,
            sentiment_monitoring: true,
            tts_enabled: false,
            tts_speed: 1.0,
            tts_pitch: 1.0,
            daily_briefing_enabled: true,
            custom_personas: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MoodHistory {
    pub entries: Vec<MoodEntry>,
    pub trend: String,
    pub dominant_emotion: String,
    pub stability_score: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MoodEntry {
    pub mood: Mood,
    pub timestamp: String,
    pub context: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryConsolidation {
    pub clusters: Vec<MemoryCluster>,
    pub total_memories: usize,
    pub auto_generated_summaries: Vec<String>,
    pub performed_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryCluster {
    pub cluster_id: String,
    pub theme: String,
    pub memory_ids: Vec<String>,
    pub summary: String,
    pub importance_score: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryLink {
    pub source_id: String,
    pub target_id: String,
    pub relation_type: String,
    pub strength: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversationBridge {
    pub current_session_id: Option<String>,
    pub linked_sessions: Vec<LinkedSession>,
    pub continuing_topics: Vec<String>,
    pub context_summary: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkedSession {
    pub session_id: String,
    pub title: String,
    pub topic_overlap: f64,
    pub last_active: String,
    pub summary: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Reminder {
    pub id: String,
    pub title: String,
    pub description: String,
    pub reminder_type: String,
    pub trigger_at: Option<String>,
    pub cron_expression: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub last_triggered_at: Option<String>,
    pub repeat_count: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HabitCheckin {
    pub id: String,
    pub habit_name: String,
    pub streak_days: u32,
    pub total_checkins: u32,
    pub last_checkin: Option<String>,
    pub completed_today: bool,
    pub category: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PomodoroSession {
    pub id: String,
    pub task_name: String,
    pub duration_minutes: u32,
    pub break_minutes: u32,
    pub cycles_completed: u32,
    pub total_cycles: u32,
    pub started_at: String,
    pub is_active: bool,
    pub time_remaining_secs: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActivityDigest {
    pub period: String,
    pub date_range: String,
    pub highlights: Vec<String>,
    pub stats_summary: DigestStats,
    pub generated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DigestStats {
    pub total_interactions: usize,
    pub memories_created: usize,
    pub conversations_completed: usize,
    pub active_hours_avg: f64,
    pub top_topics: Vec<String>,
    pub mood_average: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersonalityEvolution {
    pub current_persona_id: String,
    pub evolution_log: Vec<EvolutionEntry>,
    pub trait_drifts: Vec<TraitDrift>,
    pub recommendation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvolutionEntry {
    pub timestamp: String,
    pub previous_persona_id: String,
    pub new_persona_id: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TraitDrift {
    pub trait_name: String,
    pub original_value: f64,
    pub current_value: f64,
    pub drift_direction: String,
    pub confidence: f64,
}