use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use sqlx::SqlitePool;

use crate::crypto::mek_manager::MekManager;
use crate::error::app_error::AppError;
use crate::lsp::LspManager;
use crate::models::editor::{
    InlineCompletionItem, InlineCompletionRange, InlineCompletionRequest, InlineCompletionResult,
};

/// 行内补全服务：融合 LSP + AI 补全，去重排序
///
/// 设计意图（D1.1）：让 yuan_inline_complete 真正调用云端 API 模型，
/// 替代原"启发式补全"占位实现。AI 调用统一走 AiModelService（支持 9 种 provider）。
pub struct InlineService {
    lsp_manager: Arc<Mutex<LspManager>>,
    pool: SqlitePool,
    mek_manager: Arc<RwLock<MekManager>>,
}

impl InlineService {
    pub fn new(
        lsp_manager: Arc<Mutex<LspManager>>,
        pool: SqlitePool,
        mek_manager: Arc<RwLock<MekManager>>,
    ) -> Self {
        Self {
            lsp_manager,
            pool,
            mek_manager,
        }
    }

    /// 检查指定语言是否支持补全
    pub async fn is_available(&self, _file_path: &str, language: &str) -> bool {
        let supported = [
            "python", "javascript", "typescript", "rust", "go", "java", "c", "cpp",
            "html", "css", "json", "markdown", "yaml", "toml", "sql", "shell",
        ];
        supported.contains(&language.to_lowercase().as_str())
    }

    /// 请求行内补全
    ///
    /// `user_id` 用于 AiModelService 解密 API Key（从 MEK 中获取）。
    /// 传 0 时仅可调用本地 ollama 等无需 API Key 的模型。
    pub async fn complete(
        &self,
        request: InlineCompletionRequest,
        user_id: i64,
    ) -> Result<InlineCompletionResult, AppError> {
        let mut items: Vec<InlineCompletionItem> = Vec::new();
        let mut sources: Vec<&str> = Vec::new();

        // 1. LSP 补全
        if let Ok(lsp_items) = self.get_lsp_completions(&request).await {
            if !lsp_items.is_empty() {
                items.extend(lsp_items);
                sources.push("lsp");
            }
        }

        // 2. AI 补全（接入统一模型管理）
        // 仅在用户配置了模型时尝试；失败不阻塞 LSP/启发式结果
        if let Some(ai_item) = self.get_ai_completion(&request, user_id).await {
            items.push(ai_item);
            sources.push("ai");
        } else {
            // 3. 启发式补全（fallback：无可用模型或 AI 调用失败时）
            if let Some(heuristic_item) = self.get_heuristic_completion(&request) {
                items.push(heuristic_item);
                sources.push("heuristic");
            }
        }

        // 4. 去重（按 insert_text 去重，保留 LSP 优先）
        let mut seen = HashSet::new();
        let mut deduped: Vec<InlineCompletionItem> = Vec::new();
        for item in items {
            if seen.insert(item.insert_text.clone()) {
                deduped.push(item);
            }
        }

        let source = if deduped.is_empty() {
            "none".to_string()
        } else if sources.len() >= 2 {
            "hybrid".to_string()
        } else {
            sources.first().copied().unwrap_or("none").to_string()
        };

        Ok(InlineCompletionResult {
            items: deduped,
            source,
        })
    }

    /// 从 LSP 获取补全
    async fn get_lsp_completions(
        &self,
        request: &InlineCompletionRequest,
    ) -> Result<Vec<InlineCompletionItem>, AppError> {
        let mut manager = self.lsp_manager.lock().await;
        let completions = manager
            .get_completions(
                &request.file_path,
                request.position.line,
                request.position.character,
                ".", // workspace_root
            )
            .await?;

        let items: Vec<InlineCompletionItem> = completions
            .into_iter()
            .filter_map(|c| {
                let label = c.label.clone();
                let insert_text = c.insert_text.unwrap_or_else(|| label.clone());
                Some(InlineCompletionItem {
                    insert_text: insert_text.clone(),
                    range: InlineCompletionRange {
                        start_line: request.position.line,
                        start_character: request.position.character,
                        end_line: request.position.line,
                        end_character: request.position.character,
                    },
                    filter_text: Some(label),
                    sort_text: c.sort_text,
                })
            })
            .collect();

        Ok(items)
    }

    /// AI 补全：调用统一模型管理生成行内补全建议
    async fn get_ai_completion(
        &self,
        request: &InlineCompletionRequest,
        user_id: i64,
    ) -> Option<InlineCompletionItem> {
        // 解析模型 ID：优先使用请求中的 model_id，否则回退到第一个可用模型
        let model_id = if let Some(id) = request.model_id {
            id
        } else {
            let models = crate::db::repositories::ai_repo::get_all_models(&self.pool, user_id).await.ok()?;
            if models.is_empty() {
                return None;
            }
            models.first()?.id
        };

        // 构造补全 prompt（简洁，仅请求一行补全，适合 inline ghost text）
        let prompt = format!(
            "你是一个代码补全助手。请只返回光标位置应该插入的代码（一行或多行），不要包含任何说明或 markdown 代码块。\n\n\
             文件: {}\n\
             语言: {}\n\
             光标位置: 第 {} 行, 第 {} 列\n\n\
             --- 光标前 ---\n{}\n\
             【光标】\n\
             --- 光标后 ---\n{}\n\n\
             请直接返回要插入的代码：",
            request.file_path,
            request.language,
            request.position.line + 1,
            request.position.character + 1,
            request.context_before,
            request.context_after,
        );
        let system_prompt = "You are a code completion engine. Return ONLY the code to insert at the cursor. No markdown, no explanation.";

        let ai_service = crate::services::ai_model_service::AiModelService::new();
        let response = ai_service
            .call_model(
                &self.pool,
                &self.mek_manager,
                user_id,
                model_id,
                &prompt,
                Some(system_prompt),
            )
            .await
            .ok()?;

        let insert_text = response.trim().trim_matches('`').to_string();
        if insert_text.is_empty() {
            return None;
        }

        Some(InlineCompletionItem {
            insert_text: insert_text.clone(),
            range: InlineCompletionRange {
                start_line: request.position.line,
                start_character: request.position.character,
                end_line: request.position.line,
                end_character: request.position.character,
            },
            filter_text: Some(insert_text),
            sort_text: Some("9".to_string()), // AI 补全排序权重低于 LSP
        })
    }

    /// 启发式补全：基于当前行上下文生成简单补全
    fn get_heuristic_completion(
        &self,
        request: &InlineCompletionRequest,
    ) -> Option<InlineCompletionItem> {
        let before = request.context_before.trim_end();
        if before.is_empty() {
            return None;
        }

        // 获取当前行到光标位置的内容
        let current_line = before.lines().last().unwrap_or("");
        let trimmed = current_line.trim_start();

        // 简单的基于关键字的补全
        let completions: Vec<(&str, &str)> = vec![
            // Rust
            ("fn ", "fn () {\n    \n}"),
            ("struct ", "struct  {\n    \n}"),
            ("impl ", "impl  {\n    \n}"),
            ("pub fn", "pub fn () ->  {\n    \n}"),
            ("match ", "match  {\n     => {},\n    _ => {},\n}"),
            // Python
            ("def ", "def ():\n    "),
            ("class ", "class :\n    def __init__(self):\n        pass"),
            ("if ", "if :\n    "),
            ("for ", "for  in :\n    "),
            ("try", "try:\n    pass\nexcept Exception as e:\n    pass"),
            // JavaScript/TypeScript
            ("function ", "function () {\n    \n}"),
            ("const ", "const  = "),
            ("import ", "import {  } from ''"),
            // Go
            ("func ", "func () {\n    \n}"),
            ("if err", "if err != nil {\n    return err\n}"),
            // Common
            ("console.log", "console.log()"),
            ("println!", "println!(\"\")"),
            ("print(", "print()"),
            ("return ", "return "),
            ("let ", "let  = "),
            ("var ", "var  = "),
        ];

        for (prefix, completion) in &completions {
            if trimmed.ends_with(prefix) {
                return Some(InlineCompletionItem {
                    insert_text: completion.to_string(),
                    range: InlineCompletionRange {
                        start_line: request.position.line,
                        start_character: request.position.character,
                        end_line: request.position.line,
                        end_character: request.position.character,
                    },
                    filter_text: Some(completion.to_string()),
                    sort_text: Some("1".to_string()),
                });
            }
        }

        None
    }
}
