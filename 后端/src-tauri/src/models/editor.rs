use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EditorDocument {
    pub id: i64,
    pub doc_uuid: String,
    pub title: String,
    pub source_type: String,
    pub source_id: Option<i64>,
    pub content_type: String,
    pub language: Option<String>,
    pub file_size: i64,
    pub version_count: i64,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EditorVersion {
    pub id: i64,
    pub doc_uuid: String,
    pub version_num: i64,
    pub file_path: String,
    pub file_size: i64,
    pub change_summary: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EditorSession {
    pub id: i64,
    pub doc_uuid: String,
    pub is_dirty: i64,
    pub cursor_line: Option<i64>,
    pub cursor_column: Option<i64>,
    pub last_activity: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenDocumentRequest {
    pub source_type: String,
    pub source_id: Option<i64>,
    pub title: String,
    pub content_type: String,
    pub initial_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveDocumentRequest {
    pub doc_uuid: String,
    pub content: String,
    pub create_version: Option<bool>,
    pub change_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSaveRequest {
    pub doc_uuid: String,
    pub content: String,
    pub cursor_line: Option<i64>,
    pub cursor_column: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorContent {
    pub doc_uuid: String,
    pub title: String,
    pub content: String,
    pub content_type: String,
    pub language: Option<String>,
    pub file_size: i64,
    pub version_count: i64,
    pub versions: Vec<EditorVersion>,
}

// ============ Inline Completion 类型 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineCompletionRequest {
    pub file_path: String,
    pub language: String,
    pub position: InlineCompletionPosition,
    pub context_before: String,
    pub context_after: String,
    /// 指定使用的 AI 模型 ID（来自统一模型管理）
    /// None 时使用用户配置的默认编程模型
    #[serde(default)]
    pub model_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineCompletionPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineCompletionItem {
    pub insert_text: String,
    pub range: InlineCompletionRange,
    pub filter_text: Option<String>,
    pub sort_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineCompletionRange {
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineCompletionResult {
    pub items: Vec<InlineCompletionItem>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveredSession {
    pub doc_uuid: String,
    pub title: String,
    pub content: String,
    pub content_type: String,
    pub cursor_line: Option<i64>,
    pub cursor_column: Option<i64>,
    pub last_activity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub file_type: String,
    pub file_size: i64,
    pub extra: MediaMetadataExtra,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadataExtra {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub color_type: Option<String>,
    pub duration_secs: Option<f64>,
    pub frame_count: Option<u64>,
    pub page_count: Option<u32>,
    pub author: Option<String>,
    pub title: Option<String>,
    pub extra_info: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailRequest {
    pub doc_uuid: String,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailResult {
    pub doc_uuid: String,
    pub width: u32,
    pub height: u32,
    pub base64_png: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRequest {
    pub content: String,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxToken {
    pub text: String,
    pub scope: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightResult {
    pub tokens: Vec<SyntaxToken>,
    pub line_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvInfo {
    pub headers: Vec<String>,
    pub row_count: usize,
    pub column_count: usize,
    pub rows: Vec<Vec<String>>,
    pub delimiter: char,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvPreviewRequest {
    pub doc_uuid: String,
    pub max_rows: Option<usize>,
    pub delimiter: Option<char>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecryptedPreview {
    pub doc_uuid: String,
    pub mime_type: String,
    pub base64_content: String,
    pub file_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub doc_uuid: String,
    pub title: String,
    pub content_type: String,
    pub snippet: String,
    pub rank: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<i64>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertRequest {
    pub doc_uuid: String,
    pub target_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertResult {
    pub doc_uuid: String,
    pub source_format: String,
    pub target_format: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasObject {
    pub object_type: String,
    pub data_json: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub z_index: i64,
    pub rotation: f64,
    pub opacity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasSaveRequest {
    pub doc_uuid: String,
    pub objects: Vec<CanvasObject>,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasData {
    pub doc_uuid: String,
    pub objects: Vec<CanvasObject>,
    pub width: f64,
    pub height: f64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDiffRequest {
    pub doc_uuid: String,
    pub version_a: i64,
    pub version_b: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_type: String,
    pub old_line_no: Option<i64>,
    pub new_line_no: Option<i64>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDiffResult {
    pub doc_uuid: String,
    pub version_a: i64,
    pub version_b: i64,
    pub lines_added: i64,
    pub lines_removed: i64,
    pub lines_unchanged: i64,
    pub diff_lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionCleanupRequest {
    pub doc_uuid: String,
    pub keep_latest: Option<i64>,
    pub older_than_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionCleanupResult {
    pub doc_uuid: String,
    pub removed_count: i64,
    pub remaining_count: i64,
}

// ============ Inline Edit 类型 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineEditRequest {
    pub selected_code: String,
    pub instruction: String,
    pub language: String,
    pub file_path: String,
    /// 指定使用的 AI 模型 ID（来自统一模型管理）
    /// None 时使用用户配置的默认编程模型
    #[serde(default)]
    pub model_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineEditResponse {
    pub modified_code: String,
}