use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use once_cell::sync::Lazy;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::tokenizer::*;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};

use crate::models::search::{
    AdvancedSearchQuery, BatchIndexRequest, BatchIndexResult, FacetStats, HotQuery,
    IndexStatus, IndexableDocument, SearchHistoryEntry, SearchResponse, SearchResult,
    SearchSource, SearchSuggestRequest, SearchSuggestResult, SearchSuggestion,
    SourceBreakdown,
};

// ========== 常量 ==========

const INDEX_DIR: &str = "search_index";
const HISTORY_FILE: &str = "search_history.json";
const MAX_HISTORY: usize = 500;
const MAX_HOT_QUERIES: usize = 20;

// ========== 全局索引状态 ==========

static INDEX_STATE: Lazy<Mutex<Option<SearchIndexState>>> =
    Lazy::new(|| Mutex::new(None));

struct SearchIndexState {
    index: Index,
    reader: IndexReader,
    #[allow(dead_code)]
    schema: Schema,
    id_field: Field,
    title_field: Field,
    content_field: Field,
    source_field: Field,
    file_path_field: Field,
    updated_at_field: Field,
}

fn get_index_dir() -> PathBuf {
    dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("NexTerm")
        .join(INDEX_DIR)
}

fn get_history_path() -> PathBuf {
    dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("NexTerm")
        .join(HISTORY_FILE)
}

fn ensure_index_state() -> Result<(), String> {
    let mut state = INDEX_STATE.lock().map_err(|e| format!("锁失败: {}", e))?;
    if state.is_some() {
        return Ok(());
    }

    let index_dir = get_index_dir();
    fs::create_dir_all(&index_dir).map_err(|e| format!("创建索引目录失败: {}", e))?;

    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("id", STRING | STORED);
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("content", TEXT | STORED);
    schema_builder.add_text_field("source", STRING | STORED);
    schema_builder.add_text_field("file_path", STRING | STORED);
    schema_builder.add_text_field("updated_at", STRING | STORED);
    let schema = schema_builder.build();

    let id_field = schema.get_field("id").unwrap();
    let title_field = schema.get_field("title").unwrap();
    let content_field = schema.get_field("content").unwrap();
    let source_field = schema.get_field("source").unwrap();
    let file_path_field = schema.get_field("file_path").unwrap();
    let updated_at_field = schema.get_field("updated_at").unwrap();

    let index = if index_dir.join("meta.json").exists() {
        Index::open_in_dir(&index_dir).map_err(|e| format!("打开索引失败: {}", e))?
    } else {
        let index = Index::create_in_dir(&index_dir, schema.clone())
            .map_err(|e| format!("创建索引失败: {}", e))?;
        // 注册中文分词器
        index.tokenizers().register("cjk", TextAnalyzer::builder(SimpleTokenizer::default()).build());
        index
    };

    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()
        .map_err(|e| format!("创建 reader 失败: {}", e))?;

    *state = Some(SearchIndexState {
        index,
        reader,
        schema,
        id_field,
        title_field,
        content_field,
        source_field,
        file_path_field,
        updated_at_field,
    });

    Ok(())
}

fn with_index<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&SearchIndexState) -> Result<R, String>,
{
    ensure_index_state()?;
    let state = INDEX_STATE.lock().map_err(|e| format!("锁失败: {}", e))?;
    let st = state
        .as_ref()
        .ok_or_else(|| "索引未初始化".to_string())?;
    f(st)
}

fn with_index_writer<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&SearchIndexState, &mut IndexWriter) -> Result<R, String>,
{
    ensure_index_state()?;
    let mut state = INDEX_STATE.lock().map_err(|e| format!("锁失败: {}", e))?;
    let st = state
        .as_mut()
        .ok_or_else(|| "索引未初始化".to_string())?;
    let mut writer = st
        .index
        .writer(50_000_000)
        .map_err(|e| format!("创建 writer 失败: {}", e))?;
    let result = f(st, &mut writer);
    writer.commit().map_err(|e| format!("提交失败: {}", e))?;
    result
}

// ========== 全局搜索 ==========

pub fn global_search(
    query: &str,
    source_filter: Option<Vec<SearchSource>>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<SearchResponse, String> {
    let start = std::time::Instant::now();
    let limit = limit.unwrap_or(20).min(100);
    let offset = offset.unwrap_or(0);
    let query_str = query.to_string();

    with_index(|st| {
        let searcher = st.reader.searcher();
        let query_parser = QueryParser::for_index(&st.index, vec![st.title_field, st.content_field]);
        let parsed_query = query_parser
            .parse_query(query)
            .map_err(|e| format!("查询解析失败: {}", e))?;

        let top_docs = searcher
            .search(&parsed_query, &TopDocs::with_limit(limit + offset))
            .map_err(|e| format!("搜索失败: {}", e))?;

        let total = top_docs.len();
        let results: Vec<SearchResult> = top_docs
            .into_iter()
            .skip(offset)
            .take(limit)
            .filter_map(|(score, doc_addr)| {
                let doc: TantivyDocument = searcher.doc(doc_addr).ok()?;
                let source_str = doc
                    .get_first(st.source_field)?
                    .as_str()?;

                let source = match source_str {
                    "file" => SearchSource::File,
                    "note" => SearchSource::Note,
                    "knowledge" => SearchSource::Knowledge,
                    "kernel" => SearchSource::Kernel,
                    _ => SearchSource::File,
                };

                // 过滤 source
                if let Some(ref filter) = source_filter {
                    if !filter.contains(&source) {
                        return None;
                    }
                }

                Some(SearchResult {
                    doc_id: doc.get_first(st.id_field)?.as_str()?.to_string(),
                    title: doc.get_first(st.title_field)?.as_str()?.to_string(),
                    source,
                    snippet: doc
                        .get_first(st.content_field)
                        .and_then(|v| v.as_str())
                        .map(|s| {
                            if s.len() > 200 {
                                format!("{}...", &s[..200])
                            } else {
                                s.to_string()
                            }
                        })
                        .unwrap_or_default(),
                    score: score as f64,
                    file_path: doc
                        .get_first(st.file_path_field)
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    updated_at: doc
                        .get_first(st.updated_at_field)
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                })
            })
            .collect();

        Ok(SearchResponse {
            query: query_str,
            results,
            total,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    })
}

// ========== 索引文档 ==========

pub fn index_document(doc: IndexableDocument) -> Result<(), String> {
    with_index_writer(|st, writer| {
        // 先删除旧文档（按 ID 去重）
        let id_term = tantivy::Term::from_field_text(st.id_field, &doc.doc_id);
        writer.delete_term(id_term);

        let source_str = match doc.source {
            SearchSource::File => "file",
            SearchSource::Note => "note",
            SearchSource::Knowledge => "knowledge",
            SearchSource::Kernel => "kernel",
        };

        writer
            .add_document(doc!(
                st.id_field => doc.doc_id,
                st.title_field => doc.title,
                st.content_field => doc.content,
                st.source_field => source_str,
                st.file_path_field => doc.file_path.unwrap_or_default(),
                st.updated_at_field => doc.updated_at.unwrap_or_default(),
            ))
            .map_err(|e| format!("添加文档失败: {}", e))?;

        Ok(())
    })
}

// ========== 删除文档 ==========

pub fn delete_document(doc_id: &str) -> Result<(), String> {
    with_index_writer(|st, writer| {
        let id_term = tantivy::Term::from_field_text(st.id_field, doc_id);
        writer.delete_term(id_term);
        Ok(())
    })
}

// ========== 清空索引 ==========

pub fn clear_index() -> Result<(), String> {
    with_index_writer(|_st, writer| {
        writer.delete_all_documents().map_err(|e| format!("清空索引失败: {}", e))?;
        Ok(())
    })
}

// ========== 重建索引 ==========

pub fn rebuild_index() -> Result<(), String> {
    clear_index()?;
    Ok(())
}

// ========== 索引状态 ==========

pub fn index_status() -> Result<IndexStatus, String> {
    with_index(|st| {
        let searcher = st.reader.searcher();
        let total = searcher.num_docs();

        let mut source_counts: HashMap<String, u64> = HashMap::new();

        // 统计各来源文档数
        for (segment_ord, segment_reader) in searcher.segment_readers().iter().enumerate() {
            for doc_id in 0..segment_reader.num_docs() {
                let doc_addr = tantivy::DocAddress::new(segment_ord as u32, doc_id);
                if let Ok(doc) = searcher.doc::<TantivyDocument>(doc_addr) {
                    if let Some(source_val) = doc.get_first(st.source_field) {
                        if let Some(source_str) = source_val.as_str() {
                            *source_counts
                                .entry(source_str.to_string())
                                .or_default() += 1;
                        }
                    }
                }
            }
        }

        let source_breakdown: Vec<SourceBreakdown> = source_counts
            .into_iter()
            .map(|(s, c)| SourceBreakdown {
                source: match s.as_str() {
                    "file" => SearchSource::File,
                    "note" => SearchSource::Note,
                    "knowledge" => SearchSource::Knowledge,
                    "kernel" => SearchSource::Kernel,
                    _ => SearchSource::File,
                },
                count: c,
            })
            .collect();

        let index_size = get_index_dir_size();

        Ok(IndexStatus {
            total_documents: total,
            source_breakdown,
            index_size_bytes: index_size,
            last_updated: Some(chrono::Utc::now().to_rfc3339()),
        })
    })
}

fn get_index_dir_size() -> u64 {
    let dir = get_index_dir();
    dir_size(&dir).unwrap_or(0)
}

fn dir_size(path: &Path) -> Result<u64, String> {
    let mut total = 0u64;
    if path.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| format!("读取目录失败: {}", e))? {
            let entry = entry.map_err(|e| format!("读取条目失败: {}", e))?;
            let path = entry.path();
            if path.is_dir() {
                total += dir_size(&path).unwrap_or(0);
            } else {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    Ok(total)
}

// ========== 搜索建议 ==========

pub fn search_suggest(request: SearchSuggestRequest) -> Result<SearchSuggestResult, String> {
    let limit = request.limit.unwrap_or(10).min(30);

    with_index(|st| {
        let searcher = st.reader.searcher();
        let query_parser = QueryParser::for_index(&st.index, vec![st.title_field]);
        let prefix_query = format!("{}*", request.prefix);
        let query = query_parser
            .parse_query(&prefix_query)
            .map_err(|e| format!("查询解析失败: {}", e))?;

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| format!("搜索失败: {}", e))?;

        let mut suggestion_map: HashMap<String, u64> = HashMap::new();
        for (_score, doc_addr) in top_docs {
            if let Ok(doc) = searcher.doc::<TantivyDocument>(doc_addr) {
                if let Some(title) = doc.get_first(st.title_field).and_then(|v| v.as_str()) {
                    let title_lower = title.to_lowercase();
                    if title_lower.starts_with(&request.prefix.to_lowercase()) {
                        *suggestion_map.entry(title.to_string()).or_default() += 1;
                    }
                }
            }
        }

        let mut suggestions: Vec<SearchSuggestion> = suggestion_map
            .into_iter()
            .map(|(text, frequency)| SearchSuggestion {
                text,
                frequency,
                source: None,
            })
            .collect();
        suggestions.sort_by(|a, b| b.frequency.cmp(&a.frequency));

        Ok(SearchSuggestResult {
            prefix: request.prefix,
            suggestions,
        })
    })
}

// ========== 高级搜索 ==========

pub fn advanced_search(query: AdvancedSearchQuery) -> Result<SearchResponse, String> {
    global_search(
        &query.query,
        query.source_filter,
        query.limit,
        query.offset,
    )
}

// ========== 分面统计 ==========

pub fn facet_stats(query: &str) -> Result<FacetStats, String> {
    let response = global_search(query, None, Some(1000), None)?;

    let mut source_counts: HashMap<String, usize> = HashMap::new();
    let mut total_score = 0.0f64;

    for result in &response.results {
        let source_str = match result.source {
            SearchSource::File => "file",
            SearchSource::Note => "note",
            SearchSource::Knowledge => "knowledge",
            SearchSource::Kernel => "kernel",
        };
        *source_counts.entry(source_str.to_string()).or_default() += 1;
        total_score += result.score;
    }

    let avg_score = if response.results.is_empty() {
        0.0
    } else {
        total_score / response.results.len() as f64
    };

    Ok(FacetStats {
        source_counts,
        total_count: response.total,
        avg_score,
    })
}

// ========== 搜索历史 ==========

pub fn search_history() -> Vec<SearchHistoryEntry> {
    load_history()
}

pub fn add_search_history(
    query: &str,
    result_count: usize,
    source_filter: Option<Vec<SearchSource>>,
) {
    let mut history = load_history();

    // 插入到最前面
    history.insert(
        0,
        SearchHistoryEntry {
            query: query.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            result_count,
            source_filter,
        },
    );

    // 限制最大条数
    if history.len() > MAX_HISTORY {
        history.truncate(MAX_HISTORY);
    }

    save_history(&history);
}

pub fn clear_search_history() {
    save_history(&vec![]);
}

pub fn hot_queries(limit: Option<usize>) -> Vec<HotQuery> {
    let limit = limit.unwrap_or(MAX_HOT_QUERIES);
    let history = load_history();

    let mut query_counts: HashMap<String, (usize, i64)> = HashMap::new();
    for entry in &history {
        let entry_data = query_counts
            .entry(entry.query.clone())
            .or_insert((0, entry.timestamp));
        entry_data.0 += 1;
        if entry.timestamp > entry_data.1 {
            entry_data.1 = entry.timestamp;
        }
    }

    let mut hot: Vec<HotQuery> = query_counts
        .into_iter()
        .map(|(query, (count, last_searched_at))| HotQuery {
            query,
            count,
            last_searched_at,
        })
        .collect();

    hot.sort_by(|a, b| b.count.cmp(&a.count));
    hot.truncate(limit);
    hot
}

fn load_history() -> Vec<SearchHistoryEntry> {
    let path = get_history_path();
    if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        vec![]
    }
}

fn save_history(history: &[SearchHistoryEntry]) {
    let path = get_history_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    if let Ok(json) = serde_json::to_string_pretty(history) {
        fs::write(&path, json).ok();
    }
}

// ========== 批量索引 ==========

pub fn batch_index(request: BatchIndexRequest) -> Result<BatchIndexResult, String> {
    let start = std::time::Instant::now();
    let dir = Path::new(&request.directory);

    if !dir.exists() || !dir.is_dir() {
        return Err(format!("目录不存在或不是目录: {}", request.directory));
    }

    let extensions: Vec<String> = request
        .file_extensions
        .unwrap_or_default()
        .into_iter()
        .map(|e| e.trim_start_matches('.').to_lowercase())
        .collect();

    let max_size = request.max_file_size_kb.unwrap_or(1024) * 1024;

    let mut total_found = 0usize;
    let mut indexed = 0usize;
    let mut skipped = 0usize;
    let mut errors = Vec::new();

    let files = collect_files(dir, request.recursive.unwrap_or(true), &extensions, max_size);

    for file_path in &files {
        total_found += 1;
        match fs::read_to_string(file_path) {
            Ok(content) => {
                let file_name = file_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| file_path.to_string_lossy().to_string());
                let doc_id = format!("file:{}", file_path.to_string_lossy());
                let doc = IndexableDocument {
                    doc_id,
                    title: file_name.clone(),
                    content,
                    source: request.source.clone(),
                    file_path: Some(file_path.to_string_lossy().to_string()),
                    updated_at: Some(chrono::Utc::now().to_rfc3339()),
                };
                match index_document(doc) {
                    Ok(_) => indexed += 1,
                    Err(e) => {
                        skipped += 1;
                        errors.push(format!("{}: {}", file_name, e));
                    }
                }
            }
            Err(e) => {
                skipped += 1;
                errors.push(format!(
                    "{}: {}",
                    file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                    e
                ));
            }
        }
    }

    Ok(BatchIndexResult {
        total_found,
        indexed,
        skipped,
        errors,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })
}

fn collect_files(
    dir: &Path,
    recursive: bool,
    extensions: &[String],
    max_size: u64,
) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && recursive {
                files.extend(collect_files(&path, recursive, extensions, max_size));
            } else if path.is_file() {
                // 检查文件大小
                if let Ok(meta) = entry.metadata() {
                    if meta.len() > max_size {
                        continue;
                    }
                }
                // 检查扩展名
                if !extensions.is_empty() {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_lowercase())
                        .unwrap_or_default();
                    if !extensions.contains(&ext) {
                        continue;
                    }
                }
                files.push(path);
            }
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_and_search() {
        // 清理
        let _ = clear_index();

        // 索引一个测试文档
        let doc = IndexableDocument {
            doc_id: "test-1".into(),
            title: "Rust 编程语言指南".into(),
            content: "Rust 是一种系统编程语言，注重安全性和并发性。".into(),
            source: SearchSource::Note,
            file_path: None,
            updated_at: Some("2026-01-01T00:00:00Z".into()),
        };
        assert!(index_document(doc).is_ok());

        // 搜索
        let response = global_search("Rust", None, Some(10), None);
        assert!(response.is_ok());
        let resp = response.unwrap();
        assert!(resp.total > 0);
        assert!(resp.results.iter().any(|r| r.doc_id == "test-1"));

        // 状态
        let status = index_status().unwrap();
        assert!(status.total_documents > 0);

        // 删除
        assert!(delete_document("test-1").is_ok());
    }

    #[test]
    fn test_suggest() {
        let doc = IndexableDocument {
            doc_id: "suggest-test".into(),
            title: "Actix Web 框架教程".into(),
            content: "Actix 是 Rust 生态中最快的 Web 框架".into(),
            source: SearchSource::Knowledge,
            file_path: None,
            updated_at: None,
        };
        let _ = index_document(doc);

        let req = SearchSuggestRequest {
            prefix: "Act".into(),
            limit: Some(5),
            source_filter: None,
        };
        let result = search_suggest(req).unwrap();
        assert!(!result.suggestions.is_empty());
    }

    #[test]
    fn test_history() {
        clear_search_history();
        add_search_history("Rust async", 5, None);
        add_search_history("Rust async", 10, None);
        add_search_history("Tokio runtime", 3, None);

        let history = search_history();
        assert_eq!(history.len(), 3);

        let hot = hot_queries(Some(5));
        assert!(hot.first().map(|h| &h.query) == Some(&"Rust async".to_string()));
    }
}