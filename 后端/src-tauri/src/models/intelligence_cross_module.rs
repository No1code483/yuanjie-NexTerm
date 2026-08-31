use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActivityLog {
    pub id: i64,
    pub user_id: String,
    pub timestamp: String,
    pub module: String,
    pub operation: String,
    pub detail: Option<String>,
    pub remark: Option<String>,
    pub duration_secs: i64,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityLogRecord {
    pub user_id: String,
    pub timestamp: String,
    pub module: String,
    pub operation: String,
    pub detail: Option<String>,
    pub remark: Option<String>,
    pub duration_secs: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLogQuery {
    pub user_id: Option<String>,
    pub module: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLogPage {
    pub logs: Vec<ActivityLog>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityStats {
    pub total_operations: i64,
    pub by_module: Vec<(String, i64)>,
    pub by_operation: Vec<(String, i64)>,
    pub by_hour: Vec<(String, i64)>,
    pub by_day: Vec<(String, i64)>,
    pub period: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub period: String,
    pub total_active_secs: i64,
    pub total_active_hours: f64,
    pub total_operations: i64,
    pub top_files: Vec<TopFile>,
    pub top_modules: Vec<(String, i64)>,
    pub hourly_heatmap: Vec<HourlyActivity>,
    pub period_labels: Vec<String>,
    pub productivity_score: f64,
    pub productivity_breakdown: ProductivityBreakdown,
    pub peak_hours: Vec<String>,
    pub daily_trend: Vec<DailyTrend>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopFile {
    pub file_name: String,
    pub kb_path: String,       // 知识库内相对路径 (path_url)
    pub local_path: String,    // 本地磁盘绝对路径 (source_path)
    pub access_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyActivity {
    pub hour: String,
    pub count: i64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductivityBreakdown {
    pub focus_ratio: f64,
    pub todo_completion_ratio: f64,
    pub knowledge_regularity: f64,
    pub focus_weight: f64,
    pub todo_weight: f64,
    pub knowledge_weight: f64,
    pub raw_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyTrend {
    pub date: String,
    pub active_secs: i64,
    pub operation_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeStats {
    pub active_duration_today_secs: i64,
    pub operations_today: i64,
    pub current_productivity_score: f64,
    pub modules_used_today: usize,
    pub current_focus_module: Option<String>,
    pub last_activity_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Suggestion {
    pub id: i64,
    pub user_id: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub priority: String,
    pub source: String,
    pub status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionPage {
    pub suggestions: Vec<Suggestion>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionFeedback {
    pub suggestion_id: i64,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BehaviorPattern {
    pub id: i64,
    pub user_id: String,
    pub date: String,
    pub focus_score: f64,
    pub distraction_count: i64,
    pub kb_avg_duration_secs: i64,
    pub kb_entry_count: i64,
    pub error_operation_count: i64,
    pub total_operation_count: i64,
    pub active_start_hour: Option<i32>,
    pub active_end_hour: Option<i32>,
    pub peak_hour: Option<i32>,
    pub module_diversity: i64,
    pub consistency_score: f64,
    pub summary: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorAnalysis {
    pub date: String,
    pub focus_score: f64,
    pub focus_verdict: String,
    pub distraction_count: i64,
    pub distraction_verdict: String,
    pub kb_avg_duration_secs: i64,
    pub kb_avg_duration_human: String,
    pub kb_entry_count: i64,
    pub error_rate: f64,
    pub error_verdict: String,
    pub active_period: String,
    pub peak_hour: Option<i32>,
    pub peak_hour_label: Option<String>,
    pub module_diversity: i64,
    pub module_diversity_verdict: String,
    pub consistency_score: f64,
    pub consistency_verdict: String,
    pub summary: String,
    pub comparison: Option<BehaviorComparison>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorComparison {
    pub period_label: String,
    pub focus_change: f64,
    pub distraction_change: f64,
    pub error_change: f64,
    pub consistency_change: f64,
    pub overall_trend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorTrend {
    pub dates: Vec<String>,
    pub focus_scores: Vec<f64>,
    pub distraction_counts: Vec<i64>,
    pub kb_avg_durations: Vec<i64>,
    pub error_rates: Vec<f64>,
    pub consistency_scores: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct IntelligenceSettings {
    pub id: i64,
    pub user_id: String,
    pub log_retention_days: i64,
    pub suggestion_retention_days: i64,
    pub behavior_analysis_enabled: i64,
    pub suggestion_enabled: i64,
    pub use_llm_enhancement: i64,
    pub llm_model: Option<String>,
    pub behavior_analysis_period: String,
    pub dashboard_default_period: String,
    pub activity_log_batch_size: i64,
    pub show_productivity_score: i64,
    pub show_behavior_analysis: i64,
    pub show_suggestions: i64,
    pub notification_frequency: String,
    pub extra_config: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLogExport {
    pub logs: Vec<ActivityLog>,
    pub total_count: i64,
    pub export_time: String,
    pub filters: ActivityLogExportFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLogExportFilter {
    pub user_id: Option<String>,
    pub module: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationTemplate {
    pub module: String,
    pub template: String,
    pub description: String,
}

// ========== 跨模块智能模型 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumePolishResult {
    pub original: Option<String>,
    pub polished: String,
    pub changes: Vec<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteCheckResult {
    pub field: String,
    pub issue: String,
    pub suggestion: Option<String>,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsSummaryResult {
    pub title: String,
    pub summary: String,
    pub keywords: Vec<String>,
    pub original_word_count: i64,
    pub summary_word_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartTodoEnhance {
    pub suggest_priority: String,
    pub suggest_estimate_minutes: i64,
    pub suggest_category: Option<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalSmartFill {
    pub template: String,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerSmartReminder {
    pub suggest_break: bool,
    pub suggest_stop: bool,
    pub suggestions: Vec<String>,
    pub optimal_session_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyRecommendRequest {
    pub entry_name: String,
    pub content: String,
    pub existing_categories: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyRecommendation {
    pub category_name: String,
    pub confidence: f64,
    pub reason: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandContext {
    pub partial_input: Option<String>,
    pub current_dir: Option<String>,
    pub recent_history: Vec<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCompletion {
    pub command: String,
    pub description: String,
    pub category: String,
    pub match_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRecommendResult {
    pub recommended_games: Vec<String>,
    pub addiction_risk: String,
    pub anti_addiction_warnings: Vec<String>,
    pub today_total_minutes: i64,
    pub suggest_daily_limit_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAnalysisResult {
    pub original_query: String,
    pub corrected_query: Option<String>,
    pub corrections: Vec<String>,
    pub suggestions: Vec<String>,
    pub related_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConnectionResult {
    pub success: bool,
    pub latency_ms: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConnectionRequest {
    pub provider: String,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: Option<String>,
}

// ========== 知识库智能体 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbSummarizeRequest {
    pub entry_id: Option<String>,
    pub entry_name: String,
    pub content: String,
    pub provider: String,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbSummarizeResult {
    pub summary: String,
    pub key_points: Vec<String>,
    pub original_length: usize,
    pub summary_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbTagRequest {
    pub content: String,
    pub existing_tags: Vec<String>,
    pub provider: String,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbTagResult {
    pub suggested_tags: Vec<String>,
}