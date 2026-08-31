use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::services::xin_context_service::{ChatMessage, ChatRole, ContextWindow};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSearchQuery {
    pub keyword: String,
    pub persona_id: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSearchResult {
    pub conversation_id: String,
    pub title: String,
    pub persona_id: String,
    pub model_id: String,
    pub message_count: i64,
    pub total_tokens: i64,
    pub summary: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub matched_snippets: Vec<String>,
    pub highlights: Vec<Vec<HighlightSpan>>,
    pub relevance_score: f64,
    pub match_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSearchResponse {
    pub keyword: String,
    pub results: Vec<ConversationSearchResult>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct ConversationSearchRow {
    pub id: String,
    pub persona_id: String,
    pub model_id: String,
    pub title: String,
    pub context_json: String,
    pub message_count: i64,
    pub total_tokens: i64,
    pub summary: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct XinConversationSearchService;

impl XinConversationSearchService {
    pub async fn search(
        pool: &SqlitePool,
        user_id: i64,
        query: &ConversationSearchQuery,
    ) -> Result<ConversationSearchResponse, AppError> {
        let keyword = query.keyword.trim();
        let limit = query.limit.unwrap_or(20).min(100);
        let offset = query.offset.unwrap_or(0);

        if keyword.is_empty() {
            return Ok(ConversationSearchResponse {
                keyword: String::new(),
                results: vec![],
                total: 0,
            });
        }

        let pattern = format!("%{}%", keyword);

        let mut where_clauses = vec![
            "user_id = ?".to_string(),
            "(title LIKE ? OR summary LIKE ? OR context_json LIKE ?)".to_string(),
        ];
        let mut bind_values: Vec<String> = vec![
            pattern.clone(),
            pattern.clone(),
            pattern.clone(),
        ];

        if let Some(ref pid) = query.persona_id {
            where_clauses.push("persona_id = ?".to_string());
            bind_values.push(pid.clone());
        }

        if let Some(ref df) = query.date_from {
            where_clauses.push("updated_at >= ?".to_string());
            bind_values.push(df.clone());
        }

        if let Some(ref dt) = query.date_to {
            where_clauses.push("updated_at <= ?".to_string());
            bind_values.push(dt.clone());
        }

        let where_sql = where_clauses.join(" AND ");

        let sort = match query.sort_by.as_deref() {
            Some("created") => "created_at DESC",
            Some("title") => "title ASC",
            Some("messages") => "message_count DESC",
            _ => "updated_at DESC",
        };

        let count_sql = format!(
            "SELECT COUNT(*) FROM xin_conversations WHERE {}",
            where_sql
        );

        let search_sql = format!(
            "SELECT id, persona_id, model_id, title, context_json, message_count, total_tokens, summary, created_at, updated_at
             FROM xin_conversations WHERE {} ORDER BY {} LIMIT ? OFFSET ?",
            where_sql, sort
        );

        let total: (i64,) = {
            let mut q = sqlx::query_as(&count_sql);
            q = q.bind(user_id);
            for v in &bind_values {
                q = q.bind(v);
            }
            q.fetch_one(pool)
                .await
                .map_err(AppError::Database)?
        };

        let rows: Vec<ConversationSearchRow> = {
            let mut q = sqlx::query_as(&search_sql);
            q = q.bind(user_id);
            for v in &bind_values {
                q = q.bind(v);
            }
            q = q.bind(limit as i64).bind(offset as i64);
            q.fetch_all(pool).await.map_err(AppError::Database)?
        };

        let keyword_lower = keyword.to_lowercase();
        let results: Vec<ConversationSearchResult> = rows
            .into_iter()
            .map(|row| {
                let title_hit = row.title.to_lowercase().contains(&keyword_lower);
                let summary_hit = row
                    .summary
                    .as_deref()
                    .map(|s| s.to_lowercase().contains(&keyword_lower))
                    .unwrap_or(false);

                let contexts: Vec<ContextWindow> =
                    serde_json::from_str(&row.context_json).unwrap_or_default();
                let context = if contexts.is_empty() {
                    ContextWindow {
                        messages: vec![],
                        model_id: String::new(),
                        max_tokens: 0,
                        used_tokens: 0,
                        summary: None,
                    }
                } else {
                    contexts.into_iter().next().expect("contexts should not be empty")
                };

                let (snippets, highlights, match_type) = Self::extract_snippets(
                    &context.messages,
                    &keyword_lower,
                    &row.title,
                    &row.summary,
                );

                let relevance_score = Self::calc_relevance(
                    title_hit,
                    summary_hit,
                    snippets.len(),
                    &match_type,
                );

                ConversationSearchResult {
                    conversation_id: row.id,
                    title: row.title,
                    persona_id: row.persona_id,
                    model_id: row.model_id,
                    message_count: row.message_count,
                    total_tokens: row.total_tokens,
                    summary: row.summary,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    matched_snippets: snippets,
                    highlights,
                    relevance_score,
                    match_type,
                }
            })
            .collect();

        Ok(ConversationSearchResponse {
            keyword: keyword.to_string(),
            results,
            total: total.0 as usize,
        })
    }

    fn extract_snippets(
        messages: &[ChatMessage],
        keyword: &str,
        title: &str,
        summary: &Option<String>,
    ) -> (Vec<String>, Vec<Vec<HighlightSpan>>, String) {
        let mut snippets = Vec::new();
        let mut all_highlights = Vec::new();
        let mut match_type = String::from("none");

        if title.to_lowercase().contains(keyword) {
            let spans = Self::find_highlights(title, keyword);
            snippets.push(format!("[标题] {}", title));
            all_highlights.push(spans.iter().map(|s| HighlightSpan {
                start: s.0 + 4,
                end: s.1 + 4,
            }).collect());
            match_type = "title".into();
        }

        if let Some(ref s) = summary {
            if s.to_lowercase().contains(keyword) {
                let spans = Self::find_highlights(s, keyword);
                let excerpt = Self::excerpt(s, keyword, 120);
                snippets.push(format!("[摘要] {}", excerpt));
                all_highlights.push(spans.iter().map(|s| HighlightSpan {
                    start: s.0 + 4,
                    end: s.1 + 4,
                }).collect());
                if match_type == "none" {
                    match_type = "summary".into();
                }
            }
        }

        let mut found_in_content = false;
        for msg in messages.iter().filter(|m| m.role != ChatRole::System) {
            let lower = msg.content.to_lowercase();
            if lower.contains(keyword) {
                let role_label = match msg.role {
                    ChatRole::User => "用户",
                    ChatRole::Assistant => "小欣",
                    _ => "系统",
                };
                let excerpt = Self::excerpt(&msg.content, keyword, 150);
                let snippet = format!("[{}] {}", role_label, excerpt);
                let spans = Self::find_highlights(&msg.content, keyword);

                let base_offset = role_label.len() + 3;
                snippets.push(snippet);
                all_highlights.push(
                    spans
                        .iter()
                        .map(|s| HighlightSpan {
                            start: s.0 + base_offset,
                            end: s.1 + base_offset,
                        })
                        .collect(),
                );

                found_in_content = true;
                if snippets.len() >= 5 {
                    break;
                }
            }
        }

        if found_in_content && match_type == "none" {
            match_type = "content".into();
        }

        (snippets, all_highlights, match_type)
    }

    fn find_highlights(text: &str, keyword: &str) -> Vec<(usize, usize)> {
        let lower = text.to_lowercase();
        let mut spans = Vec::new();
        let mut start = 0;
        while let Some(pos) = lower[start..].find(keyword) {
            let abs_pos = start + pos;
            spans.push((abs_pos, abs_pos + keyword.len()));
            start = abs_pos + 1;
        }
        spans
    }

    fn excerpt(text: &str, keyword: &str, max_len: usize) -> String {
        let lower = text.to_lowercase();
        let kw_lower = keyword.to_lowercase();
        if let Some(byte_pos) = lower.find(&kw_lower) {
            let total_len = text.chars().count();
            let kw_len = keyword.chars().count();

            if total_len <= max_len {
                return text.to_string();
            }

            let char_pos = text
                .char_indices()
                .position(|(bi, _)| bi >= byte_pos)
                .unwrap_or(0);

            let half = (max_len - kw_len) / 2;
            let char_start = char_pos.saturating_sub(half);
            let char_end = (char_pos + kw_len + half).min(total_len);

            let prefix = if char_start > 0 { "..." } else { "" };
            let suffix = if char_end < total_len { "..." } else { "" };

            let excerpt_chars: String = text
                .chars()
                .skip(char_start)
                .take(char_end - char_start)
                .collect();

            format!("{}{}{}", prefix, excerpt_chars, suffix)
        } else {
            text.chars().take(max_len).collect::<String>() + "..."
        }
    }

    fn calc_relevance(
        title_hit: bool,
        summary_hit: bool,
        snippet_count: usize,
        match_type: &str,
    ) -> f64 {
        let mut score = 0.0;
        if title_hit {
            score += 3.0;
        }
        if summary_hit {
            score += 1.5;
        }
        score += (snippet_count as f64).min(5.0) * 0.8;
        match match_type {
            "title" => score += 1.0,
            "summary" => score += 0.5,
            "content" => score += 0.3,
            _ => {}
        }
        (score / 10.0).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_empty_keyword() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let query = ConversationSearchQuery {
            keyword: "".into(),
            persona_id: None,
            date_from: None,
            date_to: None,
            limit: None,
            offset: None,
            sort_by: None,
        };
        let result = XinConversationSearchService::search(&pool, 1, &query)
            .await
            .unwrap();
        assert!(result.results.is_empty());
        assert_eq!(result.total, 0);
    }

    #[test]
    fn test_find_highlights() {
        let text = "Hello world, hello Rust!";
        let spans = XinConversationSearchService::find_highlights(text, "hello");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0], (0, 5));
        assert_eq!(spans[1], (13, 18));
    }

    #[test]
    fn test_find_highlights_no_match() {
        let spans = XinConversationSearchService::find_highlights("no match here", "xyz");
        assert!(spans.is_empty());
    }

    #[test]
    fn test_excerpt_short() {
        let text = "你好世界";
        let result = XinConversationSearchService::excerpt(text, "世界", 100);
        assert_eq!(result, "你好世界");
    }

    #[test]
    fn test_excerpt_long() {
        let text = "这是一段很长的文本，其中包含关键词Rust系统编程，然后继续更多内容";
        let result = XinConversationSearchService::excerpt(text, "Rust", 15);
        assert!(result.contains("Rust"));
        assert!(result.starts_with("...") || result.ends_with("..."));
    }

    #[test]
    fn test_calc_relevance_title_hit() {
        let score = XinConversationSearchService::calc_relevance(true, false, 0, "title");
        assert!(score > 0.3);
    }

    #[test]
    fn test_calc_relevance_content_only() {
        let score = XinConversationSearchService::calc_relevance(false, false, 3, "content");
        assert!(score > 0.1);
        assert!(score < 0.5);
    }

    #[test]
    fn test_extract_snippets_title_match() {
        let messages = vec![
            ChatMessage {
                role: ChatRole::User,
                content: "今天聊聊Python".into(),
                name: None,
                timestamp: None,
            },
        ];
        let (snippets, highlights, match_type) =
            XinConversationSearchService::extract_snippets(
                &messages,
                "python",
                "Python学习笔记",
                &None,
            );
        assert_eq!(match_type, "title");
        assert!(!snippets.is_empty());
        assert!(!highlights.is_empty());
    }

    #[test]
    fn test_extract_snippets_content_match() {
        let messages = vec![
            ChatMessage {
                role: ChatRole::User,
                content: "什么是Rust的所有权？".into(),
                name: None,
                timestamp: None,
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "Rust的所有权机制是核心特性之一".into(),
                name: None,
                timestamp: None,
            },
        ];
        let (snippets, _highlights, match_type) =
            XinConversationSearchService::extract_snippets(
                &messages,
                "rust",
                "随便聊聊",
                &None,
            );
        assert_eq!(match_type, "content");
        assert_eq!(snippets.len(), 2);
        assert!(snippets[0].contains("用户"));
        assert!(snippets[1].contains("小欣"));
    }
}