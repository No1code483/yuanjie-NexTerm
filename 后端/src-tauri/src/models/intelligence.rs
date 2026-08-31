use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserActivity {
    pub activity_type: String,
    pub action: String,
    pub detail: String,
    pub file_path: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserContext {
    pub active_file: Option<String>,
    pub active_file_language: Option<String>,
    pub active_terminal_sessions: Vec<String>,
    pub recent_activities: Vec<UserActivity>,
    pub active_window_title: Option<String>,
    pub collected_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Suggestion {
    pub id: String,
    pub title: String,
    pub description: String,
    pub action_type: SuggestionActionType,
    pub action_payload: String,
    pub priority: i32,
    pub category: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SuggestionActionType {
    #[serde(rename = "run_command")]
    RunCommand,
    #[serde(rename = "open_file")]
    OpenFile,
    #[serde(rename = "ai_help")]
    AiHelp,
    #[serde(rename = "navigation")]
    Navigation,
    #[serde(rename = "info")]
    Info,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContextSnapshot {
    pub id: String,
    pub context: UserContext,
    pub captured_at: String,
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaStatus {
    pub is_installed: bool,
    pub is_running: bool,
    pub api_url: String,
    pub version: Option<String>,
    pub available_models: Vec<LocalModelInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalModelInfo {
    pub name: String,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization: Option<String>,
    pub digest: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IntelligenceConfig {
    pub enabled: bool,
    pub ollama_url: String,
    pub ollama_model: String,
    pub auto_suggest_enabled: bool,
    pub context_collection_enabled: bool,
    pub max_snapshots: usize,
    pub suggestion_interval_secs: u64,
}

impl Default for IntelligenceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ollama_url: "http://localhost:11434".into(),
            ollama_model: "llama3.2".into(),
            auto_suggest_enabled: true,
            context_collection_enabled: true,
            max_snapshots: 100,
            suggestion_interval_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorPattern {
    pub pattern_name: String,
    pub activities: Vec<String>,
    pub frequency: usize,
    pub avg_duration_secs: f64,
    pub category: String,
    pub last_observed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorAnalysisResult {
    pub patterns: Vec<BehaviorPattern>,
    pub total_activities_analyzed: usize,
    pub dominant_category: String,
    pub analyzed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRecommendation {
    pub id: String,
    pub title: String,
    pub description: String,
    pub steps: Vec<String>,
    pub relevance_score: f64,
    pub category: String,
    pub based_on_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTechStack {
    pub project_type: String,
    pub primary_language: String,
    pub frameworks: Vec<String>,
    pub build_tools: Vec<String>,
    pub package_manager: String,
    pub detected_files: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveLoadSnapshot {
    pub load_level: String,
    pub load_score: f64,
    pub active_duration_secs: u64,
    pub activity_count: u32,
    pub context_switches: u32,
    pub timestamp: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartNotification {
    pub id: String,
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub priority: String,
    pub action_type: Option<String>,
    pub action_payload: Option<String>,
    pub triggered_by: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageDashboard {
    pub period: String,
    pub total_active_hours: f64,
    pub total_activities: usize,
    pub top_activity_types: Vec<(String, usize)>,
    pub top_files: Vec<(String, usize)>,
    pub avg_sessions_per_day: f64,
    pub productivity_score: f64,
    pub peak_hours: Vec<String>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResourceSnapshot {
    pub cpu_usage_percent: f64,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub memory_usage_percent: f64,
    pub disk_free_gb: f64,
    pub disk_total_gb: f64,
    pub disk_usage_percent: f64,
    pub process_count: usize,
    pub uptime_secs: u64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyAlert {
    pub id: String,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub message: String,
    pub resource_snapshot: Option<SystemResourceSnapshot>,
    pub suggested_action: Option<String>,
    pub triggered_by: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetectionResult {
    pub alerts: Vec<AnomalyAlert>,
    pub resource_snapshot: SystemResourceSnapshot,
    pub error_rate: f64,
    pub overall_status: String,
    pub detected_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeOrganizationResult {
    pub entries_classified: u32,
    pub categories_created: Vec<String>,
    pub entries_tagged: u32,
    pub duplicate_groups: Vec<Vec<String>>,
    pub associations_found: Vec<KnowledgeAssociation>,
    pub unorganized_count: u32,
    pub organized_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeAssociation {
    pub source_title: String,
    pub target_title: String,
    pub association_type: String,
    pub confidence: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub task_type: String,
    pub name: String,
    pub cron_expression: Option<String>,
    pub interval_secs: Option<u64>,
    pub enabled: bool,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionResult {
    pub task_id: String,
    pub task_name: String,
    pub success: bool,
    pub items_processed: u32,
    pub message: String,
    pub executed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousDecisionResult {
    pub task_results: Vec<TaskExecutionResult>,
    pub scheduled_tasks: Vec<ScheduledTask>,
    pub pending_actions: Vec<String>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyBriefingData {
    pub date: String,
    pub briefing: String,
    pub todo_count: usize,
    pub todo_completed: usize,
    pub journal_words: usize,
    pub news_count: usize,
    pub timer_sessions: usize,
    pub total_focus_seconds: i64,
    pub generated_at: String,
}

/// D2.3 模块渗透：群聊上下文（供底层智能生成群聊优化建议）
///
/// 前端在群聊事件发生时调用 intelligence_chat_suggest 传入此结构。
/// 底层智能基于此生成「触发执行式」建议（非对话回复）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatModuleContext {
    /// 当前会话 ID（可选）
    pub conversation_id: Option<i64>,
    /// 活跃 AI 数量（参与群聊的 AI 个数）
    pub active_ai_count: usize,
    /// 过去 1 小时消息数
    pub message_count_last_hour: usize,
    /// 距离上一条消息的分钟数
    pub minutes_since_last_message: u64,
}

/// D2.3 模块渗透：游戏上下文（供底层智能生成游戏优化建议）
///
/// 前端在游戏事件发生时调用 intelligence_game_suggest 传入此结构。
/// 底层智能基于此生成「触发执行式」建议（非对话回复）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameModuleContext {
    /// 世界 ID
    pub world_id: String,
    /// 当前境界
    pub realm: String,
    /// 累计修为
    pub total_xp: i64,
    /// 距离上次获得修为的分钟数
    pub minutes_since_last_xp_gain: u64,
    /// 可升级的建筑数量
    pub idle_building_count: usize,
}