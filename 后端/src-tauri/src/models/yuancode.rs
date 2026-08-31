use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified_at: Option<i64>,
    pub children: Option<Vec<FileTreeNode>>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFilesRequest {
    pub workspace_path: String,
    pub depth: Option<usize>,
    pub exclude_patterns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub modified_at: Option<i64>,
    pub language: Option<String>,
    pub line_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileRequest {
    pub path: String,
    pub offset_line: Option<usize>,
    pub limit_lines: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteFileRequest {
    pub path: String,
    pub content: String,
    pub create_dirs: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateItemRequest {
    pub parent_path: String,
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteItemRequest {
    pub path: String,
    pub permanently: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameItemRequest {
    pub path: String,
    pub new_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightRequest {
    pub content: String,
    pub language: String,
    pub theme: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightLine {
    pub line_number: usize,
    pub tokens: Vec<HighlightToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightToken {
    pub text: String,
    pub scope: String,
    pub color: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightResult {
    pub language: String,
    pub lines: Vec<HighlightLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecutionRequest {
    pub language: String,
    pub code: String,
    pub working_dir: Option<String>,
    pub timeout_seconds: Option<u64>,
    pub env_vars: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeCompletionRequest {
    pub code: String,
    pub language: String,
    pub cursor_line: usize,
    pub cursor_column: usize,
    pub context_before: Option<String>,
    pub context_after: Option<String>,
    pub file_path: Option<String>,
    /// 指定使用的 AI 模型 ID（来自统一模型管理）
    /// None 时使用用户配置的默认编程模型
    #[serde(default)]
    pub model_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeCompletionResult {
    pub completions: Vec<CompletionItem>,
    pub model_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub text: String,
    pub display_text: Option<String>,
    pub description: Option<String>,
    pub replace_range: Option<ReplaceRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceRange {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CodeSnippet {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub language: String,
    pub code: String,
    pub description: Option<String>,
    pub tags: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSnippetRequest {
    pub name: String,
    pub language: String,
    pub code: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSnippetRequest {
    pub id: i64,
    pub name: Option<String>,
    pub language: Option<String>,
    pub code: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysisRequest {
    pub code: String,
    pub language: String,
    pub analysis_type: String,
    /// 指定使用的 AI 模型 ID（来自统一模型管理）
    /// None 时使用用户配置的默认编程模型
    #[serde(default)]
    pub model_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysisResult {
    pub analysis_type: String,
    pub summary: String,
    pub issues: Vec<CodeIssue>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIssue {
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub severity: String,
    pub message: String,
    pub rule: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffRequest {
    pub original_content: String,
    pub modified_content: String,
    pub file_path: Option<String>,
    pub context_lines: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub hunks: Vec<DiffHunk>,
    pub unified_diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub kind: String,
    pub content: String,
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilesRequest {
    pub workspace_path: String,
    pub query: String,
    pub case_sensitive: Option<bool>,
    pub whole_word: Option<bool>,
    pub use_regex: Option<bool>,
    pub regex: Option<bool>,
    pub include_pattern: Option<String>,
    pub exclude_pattern: Option<String>,
    pub file_pattern: Option<String>,
    pub max_results: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    pub file: String,
    pub file_path: String,
    pub line: usize,
    pub line_number: usize,
    pub column: usize,
    pub content: String,
    pub line_content: String,
    pub match_text: String,
    pub match_start: usize,
    pub match_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceFilesRequest {
    pub workspace_path: String,
    pub query: String,
    pub replacement: String,
    pub case_sensitive: Option<bool>,
    pub whole_word: Option<bool>,
    pub use_regex: Option<bool>,
    pub include_pattern: Option<String>,
    pub exclude_pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyMoveRequest {
    pub source_path: String,
    pub dest_path: String,
    pub is_move: bool,
    pub overwrite: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfoRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfoResult {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified_at: Option<i64>,
    pub created_at: Option<i64>,
    pub is_readonly: bool,
    pub line_count: Option<usize>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSession {
    pub id: Option<i64>,
    pub name: String,
    pub workspace_path: String,
    pub tabs: Vec<WorkspaceTab>,
    pub active_tab_index: usize,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceTab {
    pub file_path: String,
    pub cursor_line: usize,
    pub cursor_column: usize,
    pub scroll_top: Option<f64>,
    pub is_dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveWorkspaceRequest {
    pub name: String,
    pub workspace_path: String,
    pub tabs: Vec<WorkspaceTab>,
    pub active_tab_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatRequest {
    pub file_path: String,
    pub language: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatResult {
    pub formatted: String,
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsSecurity {
    #[serde(default)]
    pub auto_approve: bool,
    #[serde(default)]
    pub sandbox_enabled: bool,
    #[serde(default)]
    pub network_allowed: bool,
    #[serde(default)]
    pub max_file_size_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsMcpServer {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsCompression {
    #[serde(default)]
    pub auto_compact: bool,
    #[serde(default)]
    pub compact_threshold_tokens: u64,
    #[serde(default)]
    pub keep_recent_turns: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsTokenBudget {
    #[serde(default)]
    pub max_input_tokens: u64,
    #[serde(default)]
    pub max_output_tokens: u64,
    #[serde(default)]
    pub warning_threshold_pct: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsModel {
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub temperature: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YuanCodeSettings {
    #[serde(default)]
    pub security: SettingsSecurity,
    #[serde(default)]
    pub mcp_servers: Vec<SettingsMcpServer>,
    #[serde(default)]
    pub compression: SettingsCompression,
    #[serde(default)]
    pub token_budget: SettingsTokenBudget,
    #[serde(default)]
    pub models: Vec<SettingsModel>,
}