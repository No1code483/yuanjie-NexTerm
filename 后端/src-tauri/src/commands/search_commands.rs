use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::search::{
    AdvancedSearchQuery, BatchIndexRequest, BatchIndexResult, FacetStats, GlobalSearchRequest,
    GlobalSearchResult, HotQuery, IndexStatus, IndexableDocument, SearchAiSummaryRequest,
    SearchAiSummaryResponse, SearchHistoryEntry, SearchResponse, SearchSource,
    SearchSuggestRequest, SearchSuggestResult,
};
use crate::services::search_service;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

// ========== 全局搜索 ==========

#[tauri::command]
pub async fn search_global(
    state: State<'_, AppState>,
    query: String,
    source_filter: Option<Vec<SearchSource>>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<ApiResponse<SearchResponse>, String> {
    crate::commands::common::require_auth(&state).await?;
    // 记录搜索历史
    let result = search_service::global_search(&query, source_filter.clone(), limit, offset)?;
    search_service::add_search_history(&query, result.total, source_filter);
    Ok(ApiResponse::success(result))
}

// ========== 索引文档 ==========

#[tauri::command]
pub async fn search_index_document(
    state: State<'_, AppState>,
    document: IndexableDocument,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    search_service::index_document(document).map(|_| ApiResponse::success_msg("文档已索引"))
}

// ========== 删除文档 ==========

#[tauri::command]
pub async fn search_delete_document(state: State<'_, AppState>, doc_id: String) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    search_service::delete_document(&doc_id).map(|_| ApiResponse::success_msg("文档已删除"))
}

// ========== 清空索引 ==========

#[tauri::command]
pub async fn search_clear_index(state: State<'_, AppState>) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    search_service::clear_index().map(|_| ApiResponse::success_msg("索引已清空"))
}

// ========== 重建索引 ==========

#[tauri::command]
pub async fn search_rebuild_index(state: State<'_, AppState>) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    search_service::rebuild_index().map(|_| ApiResponse::success_msg("索引已重建"))
}

// ========== 索引状态 ==========

#[tauri::command]
pub async fn search_index_status(state: State<'_, AppState>) -> Result<ApiResponse<IndexStatus>, String> {
    crate::commands::common::require_auth(&state).await?;
    search_service::index_status().map(ApiResponse::success)
}

// ========== 搜索建议 ==========

#[tauri::command]
pub async fn search_suggest(
    state: State<'_, AppState>,
    request: SearchSuggestRequest,
) -> Result<ApiResponse<SearchSuggestResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    search_service::search_suggest(request).map(ApiResponse::success)
}

// ========== 高级搜索 ==========

#[tauri::command]
pub async fn search_advanced(
    state: State<'_, AppState>,
    query: AdvancedSearchQuery,
) -> Result<ApiResponse<SearchResponse>, String> {
    crate::commands::common::require_auth(&state).await?;
    search_service::advanced_search(query).map(ApiResponse::success)
}

// ========== 分面统计 ==========

#[tauri::command]
pub async fn search_facets(state: State<'_, AppState>, query: String) -> Result<ApiResponse<FacetStats>, String> {
    crate::commands::common::require_auth(&state).await?;
    search_service::facet_stats(&query).map(ApiResponse::success)
}

// ========== 搜索历史 ==========

#[tauri::command]
pub async fn search_history(state: State<'_, AppState>) -> Result<ApiResponse<Vec<SearchHistoryEntry>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(search_service::search_history()))
}

#[tauri::command]
pub async fn search_clear_history(state: State<'_, AppState>) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    search_service::clear_search_history();
    Ok(ApiResponse::success_msg("搜索历史已清空"))
}

// ========== 热门查询 ==========

#[tauri::command]
pub async fn search_hot_queries(state: State<'_, AppState>, limit: Option<usize>) -> Result<ApiResponse<Vec<HotQuery>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(search_service::hot_queries(limit)))
}

// ========== 批量索引 ==========

#[tauri::command]
pub async fn search_batch_index(
    state: State<'_, AppState>,
    request: BatchIndexRequest,
) -> Result<ApiResponse<BatchIndexResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    search_service::batch_index(request).map(ApiResponse::success)
}

// ========== 全站统一搜索（直接查数据库） ==========

fn compute_relevance(query: &str, title: &str, content: &str) -> f32 {
    let q = query.to_lowercase();
    let t = title.to_lowercase();
    let c = content.to_lowercase();

    if t == q { return 1.0; }
    if t.starts_with(&q) { return 0.9; }
    if t.contains(&q) { return 0.7; }
    if c.contains(&q) { return 0.5; }

    // 分词匹配
    let q_words: Vec<&str> = q.split_whitespace().collect();
    if q_words.len() > 1 {
        let matched = q_words.iter().filter(|w| t.contains(*w)).count() as f32;
        let ratio = matched / q_words.len() as f32;
        if ratio > 0.5 { return 0.4 * ratio; }
    }
    0.1
}

fn truncate_preview(s: &str, max_len: usize) -> String {
    let s = s.trim();
    if s.len() <= max_len { s.to_string() }
    else { format!("{}...", &s[..max_len]) }
}

#[tauri::command]
pub async fn global_search(
    state: State<'_, AppState>,
    request: GlobalSearchRequest,
) -> Result<ApiResponse<Vec<GlobalSearchResult>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let pool = &state.pool;
    let query = &request.query;
    let modules = request.modules.clone();
    let limit = request.limit.unwrap_or(50).min(200);
    let like_pattern = format!("%{}%", query);

    let mut all_results: Vec<GlobalSearchResult> = Vec::new();

    let module_enabled = |m: &str| -> bool {
        modules.as_ref().map_or(true, |ms| ms.iter().any(|x| x == m))
    };

    // 1. 知识库（多用户隔离批次 3 补全：kb_entries 按 user_id 过滤）
    if module_enabled("knowledge_base") {
        let rows: Vec<(i64, String, Option<String>, i64)> = sqlx::query_as(
            "SELECT id, name, content, updated_at FROM kb_entries WHERE user_id = ? AND (name LIKE ? OR content LIKE ?) LIMIT 30"
        )
        .bind(user_id)
        .bind(&like_pattern)
        .bind(&like_pattern)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("查询知识库失败: {}", e))?;

        for (id, name, content, updated_at) in rows {
            let content_str = content.unwrap_or_default();
            let score = compute_relevance(query, &name, &content_str);
            all_results.push(GlobalSearchResult {
                module: "knowledge_base".into(),
                id: id.to_string(),
                title: name,
                preview: truncate_preview(&content_str, 150),
                score,
                updated_at: format_ts(updated_at),
            });
        }
    }

    // 2. 对话（多用户隔离批次 2：按 user_id 过滤）
    if module_enabled("conversations") {
        let rows: Vec<(i64, i64, String, i64)> = sqlx::query_as(
            "SELECT m.id, m.conversation_id, m.content, m.created_at
             FROM messages m WHERE m.user_id = ? AND m.content LIKE ? LIMIT 30"
        )
        .bind(user_id)
        .bind(&like_pattern)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("查询对话失败: {}", e))?;

        for (id, conv_id, content, created_at) in rows {
            let title = format!("对话 #{}", conv_id);
            let score = compute_relevance(query, &title, &content);
            all_results.push(GlobalSearchResult {
                module: "conversations".into(),
                id: id.to_string(),
                title,
                preview: truncate_preview(&content, 150),
                score,
                updated_at: format_ts(created_at),
            });
        }
    }

    // 3. 笔记 (editor_documents with content_type = 'text' or 'markdown')
    if module_enabled("notes") {
        let rows: Vec<(String, String, String, i64)> = sqlx::query_as(
            "SELECT doc_uuid, title, content_type, updated_at FROM editor_documents
             WHERE (content_type = 'text' OR content_type = 'markdown' OR content_type = 'rich_text')
             AND title LIKE ? LIMIT 30"
        )
        .bind(&like_pattern)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("查询笔记失败: {}", e))?;

        for (doc_uuid, title, content_type, updated_at) in rows {
            let score = compute_relevance(query, &title, &content_type);
            all_results.push(GlobalSearchResult {
                module: "notes".into(),
                id: doc_uuid,
                title,
                preview: format!("类型: {}", content_type),
                score,
                updated_at: format_ts(updated_at),
            });
        }
    }

    // 4. 代码文件
    if module_enabled("code_files") {
        let rows: Vec<(String, String, Option<String>, i64)> = sqlx::query_as(
            "SELECT doc_uuid, title, language, updated_at FROM editor_documents
             WHERE content_type = 'code' AND title LIKE ? LIMIT 30"
        )
        .bind(&like_pattern)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("查询代码文件失败: {}", e))?;

        for (doc_uuid, title, language, updated_at) in rows {
            let lang = language.unwrap_or_else(|| "code".into());
            let score = compute_relevance(query, &title, &lang);
            all_results.push(GlobalSearchResult {
                module: "code_files".into(),
                id: doc_uuid,
                title,
                preview: format!("语言: {}", lang),
                score,
                updated_at: format_ts(updated_at),
            });
        }
    }

    // 5. 待办（多用户隔离补全：todos 按 user_id 过滤）
    if module_enabled("todos") {
        let rows: Vec<(i64, String, Option<String>, i64)> = sqlx::query_as(
            "SELECT id, title, description, updated_at FROM todos
             WHERE user_id = ? AND (title LIKE ? OR description LIKE ?) LIMIT 30"
        )
        .bind(user_id)
        .bind(&like_pattern)
        .bind(&like_pattern)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("查询待办失败: {}", e))?;

        for (id, title, description, updated_at) in rows {
            let desc = description.unwrap_or_default();
            let score = compute_relevance(query, &title, &desc);
            all_results.push(GlobalSearchResult {
                module: "todos".into(),
                id: id.to_string(),
                title,
                preview: truncate_preview(&desc, 150),
                score,
                updated_at: format_ts(updated_at),
            });
        }
    }

    // 6. 日志（多用户隔离批次 5：journals 按 user_id 过滤）
    if module_enabled("journals") {
        let rows: Vec<(i64, String, Option<String>, i64)> = sqlx::query_as(
            "SELECT id, date, content, updated_at FROM journals
             WHERE user_id = ? AND content LIKE ? LIMIT 30"
        )
        .bind(user_id)
        .bind(&like_pattern)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("查询日志失败: {}", e))?;

        for (id, date, content, updated_at) in rows {
            let content_str = content.unwrap_or_default();
            let title = format!("日志 {}", date);
            let score = compute_relevance(query, &title, &content_str);
            all_results.push(GlobalSearchResult {
                module: "journals".into(),
                id: id.to_string(),
                title,
                preview: truncate_preview(&content_str, 150),
                score,
                updated_at: format_ts(updated_at),
            });
        }
    }

    // 按分数排序
    all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    all_results.truncate(limit);

    Ok(ApiResponse::success(all_results))
}

fn format_ts(ts: i64) -> String {
    if ts == 0 {
        return String::new();
    }
    // 毫秒时间戳转日期
    let secs = ts / 1000;
    if secs > 0 {
        chrono::DateTime::from_timestamp(secs, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default()
    } else {
        String::new()
    }
}

// ========== AI 搜索摘要 ==========

#[tauri::command]
pub async fn search_ai_summary(
    state: State<'_, AppState>,
    request: SearchAiSummaryRequest,
) -> Result<ApiResponse<SearchAiSummaryResponse>, String> {
    crate::commands::common::require_auth(&state).await?;
    if request.results.is_empty() {
        return Ok(ApiResponse::success(SearchAiSummaryResponse {
            summary: "未找到相关结果，无法生成摘要。".into(),
        }));
    }

    // 构建结果文本
    let results_text: Vec<String> = request.results.iter()
        .take(10)
        .map(|r| format!("[{}] {}: {}", r.module, r.title, r.preview))
        .collect();

    let prompt = format!(
        "用户搜索了：「{}」\n\n以下是搜索结果：\n{}\n\n请根据以上搜索结果，生成一段简洁的中文综合摘要（不超过300字），回答用户的问题。如果搜索结果不相关，请如实说明。",
        request.query,
        results_text.join("\n")
    );

    // 使用 AI 服务生成摘要
    let summary = state.intelligence_service.query_local_llm(&prompt).await
        .unwrap_or_else(|_| generate_fallback_summary(&request.query, &request.results));

    Ok(ApiResponse::success(SearchAiSummaryResponse { summary }))
}

fn generate_fallback_summary(query: &str, results: &[GlobalSearchResult]) -> String {
    let total = results.len();
    let mut module_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for r in results {
        *module_counts.entry(r.module.clone()).or_default() += 1;
    }

    let mut parts: Vec<String> = Vec::new();
    let module_names: Vec<(&str, &str)> = vec![
        ("knowledge_base", "知识库"),
        ("conversations", "对话"),
        ("notes", "笔记"),
        ("code_files", "代码"),
        ("todos", "待办"),
        ("journals", "日志"),
    ];

    for (key, name) in &module_names {
        if let Some(count) = module_counts.get(*key) {
            parts.push(format!("{}（{}条）", name, count));
        }
    }

    format!(
        "关于「{}」的搜索共找到 {} 条结果，分布在以下模块：{}。请查看各模块详情以获取更多信息。",
        query,
        total,
        parts.join("、")
    )
}