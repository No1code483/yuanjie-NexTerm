use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchQuery {
    pub query: String,
    pub source_filter: Option<Vec<SearchSource>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SearchSource {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "note")]
    Note,
    #[serde(rename = "knowledge")]
    Knowledge,
    #[serde(rename = "kernel")]
    Kernel,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub doc_id: String,
    pub title: String,
    pub source: SearchSource,
    pub snippet: String,
    pub score: f64,
    pub file_path: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestRequest {
    pub prefix: String,
    pub limit: Option<usize>,
    pub source_filter: Option<Vec<SearchSource>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestResult {
    pub prefix: String,
    pub suggestions: Vec<SearchSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestion {
    pub text: String,
    pub frequency: u64,
    pub source: Option<SearchSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSearchQuery {
    pub query: String,
    pub source_filter: Option<Vec<SearchSource>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub file_extensions: Option<Vec<String>>,
    pub path_contains: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetStats {
    pub source_counts: HashMap<String, usize>,
    pub total_count: usize,
    pub avg_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryEntry {
    pub query: String,
    pub timestamp: i64,
    pub result_count: usize,
    pub source_filter: Option<Vec<SearchSource>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotQuery {
    pub query: String,
    pub count: usize,
    pub last_searched_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchIndexRequest {
    pub directory: String,
    pub source: SearchSource,
    pub file_extensions: Option<Vec<String>>,
    pub recursive: Option<bool>,
    pub max_file_size_kb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchIndexResult {
    pub total_found: usize,
    pub indexed: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub total: usize,
    pub elapsed_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexStatus {
    pub total_documents: u64,
    pub source_breakdown: Vec<SourceBreakdown>,
    pub index_size_bytes: u64,
    pub last_updated: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourceBreakdown {
    pub source: SearchSource,
    pub count: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexableDocument {
    pub doc_id: String,
    pub title: String,
    pub content: String,
    pub source: SearchSource,
    pub file_path: Option<String>,
    pub updated_at: Option<String>,
}

// ========== 全站统一搜索 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSearchRequest {
    pub query: String,
    pub modules: Option<Vec<String>>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSearchResult {
    pub module: String,
    pub id: String,
    pub title: String,
    pub preview: String,
    pub score: f32,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAiSummaryRequest {
    pub query: String,
    pub results: Vec<GlobalSearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAiSummaryResponse {
    pub summary: String,
}