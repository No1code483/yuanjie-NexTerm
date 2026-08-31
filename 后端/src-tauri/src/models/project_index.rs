use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 项目索引：被索引的项目根目录
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectIndexProject {
    pub id: i64,
    pub root_path: String,
    pub name: Option<String>,
    pub language_major: Option<String>,
    pub indexed_at: Option<i64>,
    pub file_count: i64,
    pub symbol_count: i64,
    pub status: String,
    pub last_error: Option<String>,
}

/// 项目索引：文件元数据
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectIndexFile {
    pub id: i64,
    pub project_id: i64,
    pub path: String,
    pub absolute_path: String,
    pub language: Option<String>,
    pub size_bytes: i64,
    pub last_modified: Option<i64>,
    pub content_hash: Option<String>,
    pub line_count: i64,
    pub indexed_at: i64,
}

/// 项目索引：符号（函数 / 类 / 类型 / 变量）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectIndexSymbol {
    pub id: i64,
    pub file_id: i64,
    pub name: String,
    pub kind: String,
    pub line_start: i64,
    pub line_end: i64,
    pub column_start: i64,
    pub column_end: i64,
    pub signature: Option<String>,
    pub documentation: Option<String>,
    pub is_exported: i64,
}

/// 项目索引：导入关系
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectIndexImport {
    pub id: i64,
    pub file_id: i64,
    pub imported_path: String,
    pub imported_symbol: Option<String>,
    pub line_number: i64,
    pub import_kind: String,
}

/// 项目索引：依赖图（文件间已解析的依赖）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectIndexDependency {
    pub id: i64,
    pub project_id: i64,
    pub from_file_id: i64,
    pub to_file_id: Option<i64>,
    pub from_path: String,
    pub to_path: String,
    pub dependency_type: String,
    pub strength: f64,
}

/// 项目索引：变更历史
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectIndexChange {
    pub id: i64,
    pub project_id: i64,
    pub file_path: String,
    pub change_type: String,
    pub changed_at: i64,
    pub diff_summary: Option<String>,
    pub commit_hash: Option<String>,
}

// ============ 请求 / 响应类型 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexProjectRequest {
    pub root_path: String,
    /// 强制全量重建索引（忽略 content_hash 增量）
    #[serde(default)]
    pub force_full: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexProjectResult {
    pub project_id: i64,
    pub files_indexed: usize,
    pub symbols_indexed: usize,
    pub imports_indexed: usize,
    pub duration_ms: u64,
    pub skipped_unchanged: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSymbolsRequest {
    pub project_id: i64,
    pub query: String,
    /// 限定符号种类（如 "function"）；None 表示所有
    pub kind: Option<String>,
    /// 限定只搜索导出符号
    #[serde(default)]
    pub exported_only: bool,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolSearchHit {
    pub symbol: ProjectIndexSymbol,
    pub file_path: String,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindReferencesRequest {
    pub project_id: i64,
    pub symbol_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceHit {
    pub file_path: String,
    pub line: i64,
    pub imported_symbol: Option<String>,
    pub import_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRelatedFilesRequest {
    pub project_id: i64,
    pub file_path: String,
    /// 最大返回数量
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedFile {
    pub path: String,
    pub relation: String,    // 'import' | 'imported_by' | 'call' | 'called_by'
    pub dependency_type: String,
    pub strength: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIndexStatus {
    pub project_id: i64,
    pub root_path: String,
    pub name: Option<String>,
    pub status: String,
    pub file_count: i64,
    pub symbol_count: i64,
    pub indexed_at: Option<i64>,
    pub last_error: Option<String>,
}
