// LSP Tauri 命令 — 前端调用的 LSP 接口

use tauri::State;
use crate::lsp::LspManager;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::connection::AppState;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

/// LSP 应用状态
pub struct LspState {
    pub manager: Arc<Mutex<LspManager>>,
}

/// 获取代码补全
#[tauri::command]
pub async fn lsp_completions(
    state: State<'_, AppState>,
    lsp_state: State<'_, LspState>,
    file_path: String,
    line: u32,
    character: u32,
    workspace_root: String,
) -> Result<Vec<lsp_completion::CompletionItem>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut manager = lsp_state.manager.lock().await;
    let items = manager
        .get_completions(&file_path, line, character, &workspace_root)
        .await
        .map_err(|e| e.to_string())?;

    // 转换为前端格式
    let frontend_items: Vec<lsp_completion::CompletionItem> = items
        .into_iter()
        .map(|item| lsp_completion::CompletionItem {
            label: item.label,
            kind: item.kind.map(|k| format!("{:?}", k)),
            detail: item.detail,
            documentation: item.documentation,
            insert_text: item.insert_text,
            sort_text: item.sort_text,
        })
        .collect();

    Ok(frontend_items)
}

/// 获取悬停提示
#[tauri::command]
pub async fn lsp_hover(
    state: State<'_, AppState>,
    lsp_state: State<'_, LspState>,
    file_path: String,
    line: u32,
    character: u32,
    workspace_root: String,
) -> Result<Option<lsp_completion::HoverInfo>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut manager = lsp_state.manager.lock().await;
    let result = manager
        .get_hover(&file_path, line, character, &workspace_root)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result.map(|r| lsp_completion::HoverInfo {
        contents: format!("{:?}", r.contents),
    }))
}

/// 获取跳转定义
#[tauri::command]
pub async fn lsp_definition(
    state: State<'_, AppState>,
    lsp_state: State<'_, LspState>,
    file_path: String,
    line: u32,
    character: u32,
    workspace_root: String,
) -> Result<Vec<lsp_completion::LocationInfo>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut manager = lsp_state.manager.lock().await;
    let locations = manager
        .get_definition(&file_path, line, character, &workspace_root)
        .await
        .map_err(|e| e.to_string())?;

    let frontend: Vec<lsp_completion::LocationInfo> = locations
        .into_iter()
        .map(|loc| lsp_completion::LocationInfo {
            uri: loc.uri,
            start_line: loc.range.start.line,
            start_column: loc.range.start.character,
            end_line: loc.range.end.line,
            end_column: loc.range.end.character,
        })
        .collect();

    Ok(frontend)
}

/// 获取诊断信息
#[tauri::command]
pub async fn lsp_diagnostics(
    state: State<'_, AppState>,
    lsp_state: State<'_, LspState>,
    file_path: String,
    workspace_root: String,
) -> Result<Vec<lsp_completion::DiagnosticInfo>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut manager = lsp_state.manager.lock().await;
    let diagnostics = manager
        .get_diagnostics(&file_path, &workspace_root)
        .await
        .map_err(|e| e.to_string())?;

    let frontend: Vec<lsp_completion::DiagnosticInfo> = diagnostics
        .into_iter()
        .map(|d| d.to_frontend_format())
        .map(|d| lsp_completion::DiagnosticInfo {
            message: d.message,
            severity: d.severity,
            start_line: d.start_line,
            start_column: d.start_column,
            end_line: d.end_line,
            end_column: d.end_column,
            source: d.source,
            code: d.code,
        })
        .collect();

    Ok(frontend)
}

/// 检测语言
#[tauri::command]
pub async fn lsp_detect_language(
    state: State<'_, AppState>,
    lsp_state: State<'_, LspState>,
    file_path: String,
) -> Result<Option<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let manager = lsp_state.manager.lock().await;
    Ok(manager.detect_language(&file_path))
}

/// 前端类型定义 (避免依赖后端内部类型)
mod lsp_completion {
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    pub struct CompletionItem {
        pub label: String,
        pub kind: Option<String>,
        pub detail: Option<String>,
        pub documentation: Option<String>,
        pub insert_text: Option<String>,
        pub sort_text: Option<String>,
    }

    #[derive(Debug, Serialize)]
    pub struct HoverInfo {
        pub contents: String,
    }

    #[derive(Debug, Serialize)]
    pub struct LocationInfo {
        pub uri: String,
        pub start_line: u32,
        pub start_column: u32,
        pub end_line: u32,
        pub end_column: u32,
    }

    #[derive(Debug, Serialize)]
    pub struct DiagnosticInfo {
        pub message: String,
        pub severity: String,
        pub start_line: u32,
        pub start_column: u32,
        pub end_line: u32,
        pub end_column: u32,
        pub source: String,
        pub code: String,
    }
}