use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::error::app_error::AppError;
use crate::models::knowledge::KbEntry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSnippet {
    pub entry_id: i64,
    pub entry_name: String,
    pub category_name: String,
    pub snippet: String,
    pub relevance_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeFusionResult {
    pub snippets: Vec<KnowledgeSnippet>,
    pub total_found: usize,
    pub formatted_context: String,
    pub is_relevant: bool,
}

pub struct XinKnowledgeFusion;

impl XinKnowledgeFusion {
    pub fn extract_query_keywords(user_message: &str) -> Vec<String> {
        let msg = user_message.to_lowercase();

        let stop_words = [
            "的", "了", "在", "是", "我", "有", "和", "就", "不", "人", "都",
            "一", "一个", "上", "也", "很", "到", "说", "要", "去", "你",
            "会", "着", "没有", "看", "好", "自己", "这", "他", "她", "它",
            "们", "那", "什么", "怎么", "为什么", "可以", "这个", "那个",
            "the", "a", "an", "is", "are", "was", "were", "be", "been",
            "being", "have", "has", "had", "do", "does", "did", "will",
            "would", "could", "should", "may", "might", "can", "shall",
            "to", "of", "in", "for", "on", "with", "at", "by", "from",
            "as", "into", "through", "during", "before", "after",
            "and", "but", "or", "nor", "not", "so", "yet", "both",
            "i", "you", "he", "she", "it", "we", "they", "me", "him",
            "her", "us", "them", "my", "your", "his", "its", "our", "their",
        ];

        let mut word_freq: HashMap<String, usize> = HashMap::new();
        let chars: Vec<char> = msg.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if !chars[i].is_alphanumeric() && chars[i] != '-' && chars[i] != '_' && chars[i] != '#'
                && chars[i] != '+' && chars[i] != '.'
            {
                i += 1;
                continue;
            }

            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric()
                    || chars[i] == '-'
                    || chars[i] == '_'
                    || chars[i] == '#'
                    || chars[i] == '+'
                    || chars[i] == '.')
            {
                i += 1;
            }

            let word: String = chars[start..i].iter().collect();
            let lower = word.to_lowercase();

            if lower.len() >= 2
                && !stop_words.contains(&lower.as_str())
                && !lower.chars().all(|c| c.is_numeric())
                && !lower.chars().all(|c| c == '.')
            {
                *word_freq.entry(lower).or_insert(0) += 1;
            }
        }

        let mut scored: Vec<(String, usize)> = word_freq.into_iter().collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.iter().take(8).map(|(w, _)| w.clone()).collect()
    }

    pub async fn retrieve_relevant(
        pool: &SqlitePool,
        user_id: i64,
        user_message: &str,
        max_results: usize,
    ) -> Result<KnowledgeFusionResult, AppError> {
        let keywords = Self::extract_query_keywords(user_message);

        if keywords.is_empty() {
            return Ok(KnowledgeFusionResult {
                snippets: vec![],
                total_found: 0,
                formatted_context: String::new(),
                is_relevant: false,
            });
        }

        let mut all_entries: Vec<KbEntry> = Vec::new();
        let mut seen_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();

        for keyword in &keywords {
            let pattern = format!("%{}%", keyword);
            // 多用户隔离批次 3 补全：kb_entries 按 user_id 过滤，防止跨用户检索知识库
            let entries: Vec<KbEntry> = sqlx::query_as::<_, KbEntry>(
                "SELECT * FROM kb_entries WHERE user_id = ? AND (name LIKE ? OR path_url LIKE ? OR content LIKE ?) ORDER BY created_at DESC LIMIT 20",
            )
            .bind(user_id)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .fetch_all(pool)
            .await
            .map_err(AppError::Database)?;

            for entry in entries {
                if seen_ids.insert(entry.id) {
                    all_entries.push(entry);
                }
            }
        }

        let mut scored: Vec<(KbEntry, f64)> = all_entries
            .into_iter()
            .map(|entry| {
                let score = Self::compute_relevance(&keywords, &entry);
                (entry, score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let total_found = scored.len();
        let top: Vec<(KbEntry, f64)> = scored.into_iter().take(max_results).collect();

        let mut snippets = Vec::new();
        for (entry, score) in &top {
            let category_name = Self::get_category_name(pool, user_id, entry.category_id).await?;

            let snippet = if let Some(ref content) = entry.content {
                if content.len() > 300 {
                    let end = content
                        .char_indices()
                        .take(300)
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(content.len());
                    content[..end].to_string()
                } else {
                    content.clone()
                }
            } else {
                entry.name.clone()
            };

            snippets.push(KnowledgeSnippet {
                entry_id: entry.id,
                entry_name: entry.name.clone(),
                category_name,
                snippet,
                relevance_score: (*score * 100.0).round() / 100.0,
            });
        }

        let is_relevant = !snippets.is_empty()
            && snippets.iter().any(|s| s.relevance_score > 0.3);

        let formatted_context = if snippets.is_empty() {
            String::new()
        } else {
            let mut ctx = String::from("\n\n--- 相关参考资料 ---\n");
            ctx.push_str("以下是你知识库中与当前问题相关的内容，请参考这些内容进行回答（如果相关的话）：\n\n");
            for (i, s) in snippets.iter().enumerate() {
                ctx.push_str(&format!(
                    "[{}] 《{}》(分类: {} | 相关度: {}):\n{}\n\n",
                    i + 1,
                    s.entry_name,
                    s.category_name,
                    s.relevance_score,
                    s.snippet,
                ));
            }
            ctx.push_str("---\n");
            ctx
        };

        Ok(KnowledgeFusionResult {
            snippets,
            total_found,
            formatted_context,
            is_relevant,
        })
    }

    fn compute_relevance(keywords: &[String], entry: &KbEntry) -> f64 {
        let name_lower = entry.name.to_lowercase();
        let content_lower = entry
            .content
            .as_ref()
            .map(|c| c.to_lowercase())
            .unwrap_or_default();
        let path_lower = entry.path_url.to_lowercase();

        let mut score = 0.0;

        for keyword in keywords {
            let keyword_lower = keyword.to_lowercase();

            let name_matches = name_lower.matches(&keyword_lower).count();
            let content_matches = content_lower.matches(&keyword_lower).count();
            let path_matches = path_lower.matches(&keyword_lower).count();

            score += name_matches as f64 * 3.0;
            score += content_matches as f64 * 1.5;
            score += path_matches as f64 * 1.0;

            if name_lower.starts_with(&keyword_lower) {
                score += 2.0;
            }
        }

        let name_len = name_lower.len() as f64;
        if name_len < 50.0 {
            score += 0.5;
        }

        let total_keywords = keywords.len() as f64;
        if total_keywords > 0.0 {
            score /= total_keywords * 2.0;
        }

        score.min(1.0)
    }

    async fn get_category_name(pool: &SqlitePool, user_id: i64, category_id: i64) -> Result<String, AppError> {
        // 多用户隔离批次 3 补全：kb_categories 按 user_id 过滤，防止跨用户读取分类名
        let result: Option<(String,)> =
            sqlx::query_as("SELECT name FROM kb_categories WHERE user_id = ? AND id = ?")
                .bind(user_id)
                .bind(category_id)
                .fetch_optional(pool)
                .await
                .map_err(AppError::Database)?;

        Ok(result.map(|r| r.0).unwrap_or_else(|| "未分类".to_string()))
    }

    pub fn build_enhanced_system_prompt(
        base_prompt: &str,
        fusion_result: &KnowledgeFusionResult,
    ) -> String {
        if !fusion_result.is_relevant || fusion_result.formatted_context.is_empty() {
            return base_prompt.to_string();
        }

        let mut enhanced = base_prompt.to_string();
        enhanced.push_str("\n\n## 知识库增强\n");
        enhanced.push_str(
            "小欣已经自动检索了你的知识库，以下是找到的相关参考资料。请基于这些资料提供更准确的回答，同时保持小欣的人格与语气。\n",
        );
        enhanced.push_str(&fusion_result.formatted_context);

        enhanced
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FusionSource {
    #[serde(rename = "kb")]
    KnowledgeBase,
    #[serde(rename = "memory")]
    UserMemory,
    #[serde(rename = "dialog_history")]
    DialogHistory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedResult {
    pub source: FusionSource,
    pub source_id: String,
    pub title: String,
    pub snippet: String,
    pub full_content: Option<String>,
    pub relevance_score: f64,
    pub source_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalDecision {
    pub should_retrieve: bool,
    pub confidence: f64,
    pub sources: Vec<FusionSource>,
    pub reason: String,
    pub extracted_query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSourceFusion {
    pub results: Vec<UnifiedResult>,
    pub total_sources: usize,
    pub kb_count: usize,
    pub memory_count: usize,
    pub formatted_context: String,
    pub is_useful: bool,
}

impl XinKnowledgeFusion {
    pub fn should_retrieve(user_message: &str) -> RetrievalDecision {
        let msg_lower = user_message.to_lowercase();
        let mut score = 0.0f64;
        let mut reasons = Vec::new();
        let mut sources = Vec::new();

        let knowledge_patterns = [
            "什么是", "怎么", "如何", "为什么", "解释", "说明",
            "区别", "对比", "原理", "教程", "指南", "文档",
            "问题", "方案", "方法",
            "how to", "what is", "explain", "difference", "tutorial",
            "guide", "definition", "concept",
        ];
        let question_marks = msg_lower.matches('?').count()
            + msg_lower.matches('？').count()
            + msg_lower.matches('吗').count()
            + msg_lower.matches('呢').count();

        if question_marks > 0 {
            score += 0.2;
            reasons.push("包含疑问语气".to_string());
        }

        for pattern in &knowledge_patterns {
            if msg_lower.contains(pattern) {
                score += 0.25;
                reasons.push(format!("触发知识查询模式: {}", pattern));
                sources.push(FusionSource::KnowledgeBase);
                break;
            }
        }

        let tech_terms = [
            "rust", "python", "javascript", "tauri", "react", "vue",
            "sqlite", "postgres", "docker", "git", "linux", "api",
            "算法", "数据结构", "数据库", "编译", "部署", "测试",
            "性能", "安全", "加密", "网络", "并发", "异步",
        ];
        for term in &tech_terms {
            if msg_lower.contains(term) {
                score += 0.15;
                reasons.push(format!("识别到技术术语: {}", term));
                break;
            }
        }

        let code_indicators = [
            "代码", "函数", "类", "接口", "模块", "包",
            "fn ", "fn(", "def ", "function", "class ", "import ",
            "struct", "enum", "trait", "impl",
        ];
        for indicator in &code_indicators {
            if msg_lower.contains(indicator) {
                score += 0.2;
                reasons.push("涉及代码/编程".to_string());
                sources.push(FusionSource::KnowledgeBase);
                break;
            }
        }

        let personal_patterns = ["我记得", "我之前", "以前", "上次", "我的", "个人",
            "i remember", "my ", "previously", "last time"];
        for pattern in &personal_patterns {
            if msg_lower.contains(pattern) {
                score += 0.15;
                reasons.push("涉及个人记忆".to_string());
                sources.push(FusionSource::UserMemory);
                break;
            }
        }

        if user_message.len() > 30 {
            score += 0.1;
        }

        if user_message.len() < 8 && question_marks == 0 {
            score -= 0.3;
            reasons.push("消息过短，可能为寒暄".to_string());
        }

        if sources.is_empty() && score > 0.3 {
            sources.push(FusionSource::KnowledgeBase);
        }

        let should_retrieve = score > 0.35;
        let confidence = score.clamp(0.0, 1.0);

        let reason = if reasons.is_empty() {
            "未检测到明确的知识查询意图".to_string()
        } else {
            reasons.join("; ")
        };

        RetrievalDecision {
            should_retrieve,
            confidence,
            sources,
            reason,
            extracted_query: user_message.to_string(),
        }
    }

    pub async fn query_memories(
        pool: &SqlitePool,
        user_id: i64,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<UnifiedResult>, AppError> {
        let keywords = Self::extract_query_keywords(query);
        let mut results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        // Phase 3 §2.2.4 批量化：原实现按 keyword 循环逐个 SELECT（N+1），
        // 改为单次 SELECT，WHERE 子句用 OR 连接所有 keyword 的 LIKE 模式。
        // 语义等价：原逻辑对每个 keyword 取前 10 条（按 importance DESC），
        // 合并后通过 seen_ids 去重。批量化后取 keywords.len()*10 条上限，
        // 仍按 importance DESC 排序，去重后结果集等价（极端分布下条数略多，但 max_results 截断保证上限）。
        if !keywords.is_empty() {
            // 每个 keyword 产生 2 个 LIKE 模式（key LIKE ? OR value LIKE ?）
            let or_clauses: Vec<String> = keywords
                .iter()
                .map(|_| "(key LIKE ? OR value LIKE ?)".to_string())
                .collect();
            let where_sql = or_clauses.join(" OR ");
            // 上限：每 keyword 10 条 + 一定冗余去重空间
            let limit = (keywords.len() * 10) as i64;
            let sql = format!(
                "SELECT id, category, key, value, importance, confidence, created_at, last_recalled_at
                 FROM xin_memories
                 WHERE user_id = ? AND ({})
                 ORDER BY importance DESC, created_at DESC
                 LIMIT ?",
                where_sql
            );
            let mut q = sqlx::query_as::<
                _,
                (String, String, String, String, f64, f64, String, Option<String>),
            >(&sql);
            q = q.bind(user_id);
            for kw in &keywords {
                let pattern = format!("%{}%", kw);
                q = q.bind(pattern.clone()).bind(pattern);
            }
            q = q.bind(limit);

            let rows = q
                .fetch_all(pool)
                .await
                .map_err(|e| AppError::Database(e))?;

            for (id, category, key, value, importance, confidence, _created_at, _last_recalled_at) in
                rows
            {
                if seen_ids.insert(id.clone()) {
                    let snippet = if value.len() > 200 {
                        let end = value
                            .char_indices()
                            .take(200)
                            .last()
                            .map(|(i, _)| i)
                            .unwrap_or(value.len());
                        value[..end].to_string()
                    } else {
                        value.clone()
                    };

                    let score = (importance * 0.6 + confidence * 0.4) as f64;

                    results.push(UnifiedResult {
                        source: FusionSource::UserMemory,
                        source_id: id,
                        title: key,
                        snippet,
                        full_content: Some(value),
                        relevance_score: score,
                        source_label: format!("记忆·{}", category),
                    });
                }
            }
        }

        let query_lower = query.to_lowercase();
        for r in &mut results {
            let title_hits = r.title.to_lowercase().matches(&query_lower).count() as f64;
            let content_hits = r
                .snippet
                .to_lowercase()
                .matches(&query_lower)
                .count() as f64;
            r.relevance_score += title_hits * 0.15 + content_hits * 0.1;
            r.relevance_score = r.relevance_score.min(1.0);
        }

        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(max_results);
        Ok(results)
    }

    pub fn convert_kb_to_unified(
        snippets: &[KnowledgeSnippet],
    ) -> Vec<UnifiedResult> {
        snippets
            .iter()
            .map(|s| UnifiedResult {
                source: FusionSource::KnowledgeBase,
                source_id: s.entry_id.to_string(),
                title: s.entry_name.clone(),
                snippet: s.snippet.clone(),
                full_content: None,
                relevance_score: s.relevance_score,
                source_label: format!("知识库·{}", s.category_name),
            })
            .collect()
    }

    pub fn fuse_multi_source(
        kb_results: Vec<UnifiedResult>,
        memory_results: Vec<UnifiedResult>,
        max_total: usize,
    ) -> Vec<UnifiedResult> {
        let mut all: Vec<UnifiedResult> = Vec::new();
        all.extend(kb_results);
        all.extend(memory_results);

        all.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let deduped = Self::deduplicate_results(all);
        deduped.into_iter().take(max_total).collect()
    }

    pub fn deduplicate_results(results: Vec<UnifiedResult>) -> Vec<UnifiedResult> {
        let mut deduped: Vec<UnifiedResult> = Vec::new();
        let mut seen_titles = std::collections::HashSet::new();

        for r in results {
            let normalized = r.title.to_lowercase().trim().to_string();
            if seen_titles.insert(normalized) {
                deduped.push(r);
            }
        }

        deduped
    }

    pub fn build_unified_context(
        results: &[UnifiedResult],
        max_chars: usize,
    ) -> String {
        if results.is_empty() {
            return String::new();
        }

        let mut ctx = String::from("\n\n--- 知识融合上下文 ---\n");
        ctx.push_str("以下是与你当前问题相关的参考资料和记忆：\n\n");

        let mut char_count = 0;
        let mut included = 0;

        for (i, r) in results.iter().enumerate() {
            let entry = format!(
                "[{}] [{}] {}:\n{}\n\n",
                i + 1,
                r.source_label,
                r.title,
                r.snippet,
            );

            if char_count + entry.len() > max_chars && included >= 3 {
                ctx.push_str(&format!("...以及其他 {} 条相关结果\n", results.len() - included));
                break;
            }

            ctx.push_str(&entry);
            char_count += entry.len();
            included += 1;
        }

        ctx.push_str("请结合以上信息进行回答。\n---\n");
        ctx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords_chinese() {
        let keywords = XinKnowledgeFusion::extract_query_keywords("Rust语言的异步编程模型是怎么实现的");
        assert!(!keywords.is_empty());
        let has_rust = keywords.iter().any(|k| k.contains("rust"));
        assert!(has_rust);
    }

    #[test]
    fn test_extract_keywords_code() {
        let keywords = XinKnowledgeFusion::extract_query_keywords("如何在cargo项目中添加依赖包");
        assert!(!keywords.is_empty());
        let has_cargo = keywords.iter().any(|k| k.contains("cargo"));
        assert!(has_cargo);
    }

    #[test]
    fn test_extract_keywords_empty() {
        let keywords = XinKnowledgeFusion::extract_query_keywords("你好吗");
        assert!(keywords.len() <= 3);
    }

    #[test]
    fn test_compute_relevance() {
        let entry = KbEntry {
            id: 1,
            user_id: 1,
            category_id: 1,
            name: "Rust异步编程指南".to_string(),
            path_url: "/docs/rust-async.md".to_string(),
            entry_type: "file".to_string(),
            created_at: 0,
            updated_at: 0,
            is_favorited: None,
            content: Some("Rust的异步编程基于async/await语法和Future trait。".to_string()),
            source_path: None,
            file_exists: true,
        };
        let keywords = vec!["rust".to_string(), "异步".to_string()];
        let score = XinKnowledgeFusion::compute_relevance(&keywords, &entry);
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_build_enhanced_prompt() {
        let base = "You are a helpful assistant.".to_string();
        let fusion = KnowledgeFusionResult {
            snippets: vec![KnowledgeSnippet {
                entry_id: 1,
                entry_name: "Rust指南".to_string(),
                category_name: "学习".to_string(),
                snippet: "Rust是系统编程语言...".to_string(),
                relevance_score: 0.8,
            }],
            total_found: 1,
            formatted_context: "\n--- 相关参考资料 ---\n[1] 《Rust指南》: Rust是系统编程语言...\n---\n"
                .to_string(),
            is_relevant: true,
        };
        let enhanced = XinKnowledgeFusion::build_enhanced_system_prompt(&base, &fusion);
        assert!(enhanced.contains("知识库增强"));
        assert!(enhanced.contains("Rust指南"));
    }

    #[test]
    fn test_build_enhanced_prompt_irrelevant() {
        let base = "You are a helpful assistant.".to_string();
        let fusion = KnowledgeFusionResult {
            snippets: vec![],
            total_found: 0,
            formatted_context: String::new(),
            is_relevant: false,
        };
        let enhanced = XinKnowledgeFusion::build_enhanced_system_prompt(&base, &fusion);
        assert_eq!(enhanced, base);
    }

    #[test]
    fn test_should_retrieve_technical() {
        let decision = XinKnowledgeFusion::should_retrieve("Rust的async/await是怎么实现的？");
        assert!(decision.should_retrieve);
        assert!(decision.sources.contains(&FusionSource::KnowledgeBase));
    }

    #[test]
    fn test_should_retrieve_personal() {
        let decision = XinKnowledgeFusion::should_retrieve("我记得上次讨论过这个问题");
        assert!(decision.should_retrieve);
        assert!(decision.sources.contains(&FusionSource::UserMemory));
    }

    #[test]
    fn test_should_not_retrieve_greeting() {
        let decision = XinKnowledgeFusion::should_retrieve("你好");
        assert!(!decision.should_retrieve);
    }

    #[test]
    fn test_convert_kb_to_unified() {
        let snippets = vec![
            KnowledgeSnippet {
                entry_id: 1,
                entry_name: "Rust Guide".into(),
                category_name: "学习".into(),
                snippet: "Rust是系统编程语言".into(),
                relevance_score: 0.8,
            },
        ];
        let unified = XinKnowledgeFusion::convert_kb_to_unified(&snippets);
        assert_eq!(unified.len(), 1);
        assert_eq!(unified[0].source, FusionSource::KnowledgeBase);
        assert_eq!(unified[0].source_label, "知识库·学习");
    }

    #[test]
    fn test_fuse_multi_source() {
        let kb = vec![UnifiedResult {
            source: FusionSource::KnowledgeBase,
            source_id: "1".into(),
            title: "Rust Guide".into(),
            snippet: "Rust是系统编程语言".into(),
            full_content: None,
            relevance_score: 0.8,
            source_label: "知识库·学习".into(),
        }];
        let mem = vec![UnifiedResult {
            source: FusionSource::UserMemory,
            source_id: "m1".into(),
            title: "Rust学习计划".into(),
            snippet: "每天学习2小时Rust".into(),
            full_content: None,
            relevance_score: 0.6,
            source_label: "记忆·fact".into(),
        }];
        let fused = XinKnowledgeFusion::fuse_multi_source(kb, mem, 5);
        assert_eq!(fused.len(), 2);
        assert_eq!(fused[0].relevance_score, 0.8);
    }

    #[test]
    fn test_deduplicate_results() {
        let results = vec![
            UnifiedResult {
                source: FusionSource::KnowledgeBase,
                source_id: "1".into(),
                title: "Rust Guide".into(),
                snippet: "content".into(),
                full_content: None,
                relevance_score: 0.8,
                source_label: "KB".into(),
            },
            UnifiedResult {
                source: FusionSource::UserMemory,
                source_id: "2".into(),
                title: "rust guide".into(),
                snippet: "other".into(),
                full_content: None,
                relevance_score: 0.5,
                source_label: "Mem".into(),
            },
        ];
        let deduped = XinKnowledgeFusion::deduplicate_results(results);
        assert_eq!(deduped.len(), 1);
    }

    #[test]
    fn test_build_unified_context() {
        let results = vec![UnifiedResult {
            source: FusionSource::KnowledgeBase,
            source_id: "1".into(),
            title: "Rust".into(),
            snippet: "系统编程语言".into(),
            full_content: None,
            relevance_score: 0.8,
            source_label: "知识库·学习".into(),
        }];
        let ctx = XinKnowledgeFusion::build_unified_context(&results, 2000);
        assert!(ctx.contains("知识融合上下文"));
        assert!(ctx.contains("Rust"));
    }

    #[test]
    fn test_build_unified_context_empty() {
        let ctx = XinKnowledgeFusion::build_unified_context(&[], 2000);
        assert!(ctx.is_empty());
    }
}