// LSP 客户端 — 与语言服务器进程通信
// 对标 VSCode 的 LSP 集成，通过 stdio + JSON-RPC 与语言服务器交互

use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin, Command};
use tracing::debug;

use crate::error::app_error::AppError;
use super::protocol::*;
use super::LanguageServerConfig;

/// LSP 客户端
pub struct LspClient {
    /// 子进程句柄
    child: Option<Child>,
    /// stdin 写入端
    stdin: Option<ChildStdin>,
    /// 下一个请求 ID
    next_id: u64,
    /// 缓存的诊断结果 (file_uri -> diagnostics)
    diagnostics_cache: HashMap<String, Vec<Diagnostic>>,
    /// 初始化是否完成
    initialized: bool,
}

impl LspClient {
    /// 启动语言服务器进程
    pub async fn start(config: &LanguageServerConfig, workspace_root: &str) -> Result<Self, AppError> {
        debug!("启动语言服务器: {} (工作区: {})", config.command, workspace_root);

        let mut child = Command::new(&config.command)
            .args(&config.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                AppError::Internal(format!(
                    "无法启动语言服务器 {}: {} (请确保已安装)",
                    config.command, e
                ))
            })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            AppError::Internal("无法获取语言服务器 stdin".into())
        })?;
        // stdout 暂不存储，后续异步读取时使用
        let _stdout = child.stdout.take();

        let mut client = Self {
            child: Some(child),
            stdin: Some(stdin),
            next_id: 1,
            diagnostics_cache: HashMap::new(),
            initialized: false,
        };

        // 发送 initialize 请求
        let init_params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(format!("file://{}", workspace_root.replace('\\', "/"))),
            capabilities: ClientCapabilities {
                text_document: Some(TextDocumentClientCapabilities {
                    completion: Some(CompletionCapabilities {
                        completion_item: Some(CompletionItemCapabilities {
                            snippet_support: Some(true),
                        }),
                    }),
                    hover: Some(HoverCapabilities {
                        content_format: Some(vec!["markdown".into(), "plaintext".into()]),
                    }),
                    definition: Some(DefinitionCapabilities {
                        link_support: Some(true),
                    }),
                }),
            },
        };

        let _response = client.send_request("initialize", &init_params).await?;
        client.send_notification("initialized", &serde_json::Value::Null).await?;
        client.initialized = true;

        debug!("语言服务器 {} 初始化完成", config.command);
        Ok(client)
    }

    /// 发送 JSON-RPC 请求
    async fn send_request<T: serde::Serialize>(
        &mut self,
        method: &str,
        params: &T,
    ) -> Result<serde_json::Value, AppError> {
        let id = self.next_id;
        self.next_id += 1;

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let msg = serde_json::to_string(&request)
            .map_err(|e| AppError::Internal(format!("序列化请求失败: {}", e)))?;

        // LSP 使用 Content-Length header
        let header = format!("Content-Length: {}\r\n\r\n", msg.len());
        let full_msg = format!("{}{}", header, msg);

        // 写入 stdin
        if let Some(ref mut stdin) = self.stdin {
            stdin
                .write_all(full_msg.as_bytes())
                .await
                .map_err(|e| AppError::Internal(format!("写入 LSP stdin 失败: {}", e)))?;
            stdin
                .flush()
                .await
                .map_err(|e| AppError::Internal(format!("刷新 LSP stdin 失败: {}", e)))?;
        }

        // 读取响应 (简化实现: 返回空作为占位)
        // 完整实现需要异步读取 stdout 并解析响应
        Ok(serde_json::Value::Null)
    }

    /// 发送 JSON-RPC 通知
    async fn send_notification(
        &mut self,
        method: &str,
        params: &serde_json::Value,
    ) -> Result<(), AppError> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        let msg = serde_json::to_string(&notification)
            .map_err(|e| AppError::Internal(format!("序列化通知失败: {}", e)))?;

        let header = format!("Content-Length: {}\r\n\r\n", msg.len());
        let full_msg = format!("{}{}", header, msg);

        if let Some(ref mut stdin) = self.stdin {
            stdin
                .write_all(full_msg.as_bytes())
                .await
                .map_err(|e| AppError::Internal(format!("写入 LSP stdin 失败: {}", e)))?;
            stdin
                .flush()
                .await
                .map_err(|e| AppError::Internal(format!("刷新 LSP stdin 失败: {}", e)))?;
        }

        Ok(())
    }

    /// 通知语言服务器文件已打开
    pub async fn did_open(&mut self, file_path: &str, content: &str) -> Result<(), AppError> {
        if !self.initialized {
            return Ok(());
        }

        let uri = format!("file://{}", file_path.replace('\\', "/"));
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: detect_language_from_path(file_path),
                version: 1,
                text: content.to_string(),
            },
        };

        self.send_notification("textDocument/didOpen", &serde_json::to_value(&params).unwrap_or_default())
            .await
    }

    /// 获取代码补全
    pub async fn get_completions(
        &mut self,
        params: CompletionParams,
    ) -> Result<Vec<CompletionItem>, AppError> {
        let response: serde_json::Value = self.send_request("textDocument/completion", &params).await?;

        // 解析补全结果
        if let Some(items) = response.get("items") {
            let items: Vec<CompletionItem> = serde_json::from_value(items.clone())
                .map_err(|e| AppError::Internal(format!("解析补全结果失败: {}", e)))?;
            Ok(items)
        } else {
            Ok(vec![])
        }
    }

    /// 获取悬停提示
    pub async fn get_hover(
        &mut self,
        doc: TextDocumentIdentifier,
        position: Position,
    ) -> Result<Option<HoverResult>, AppError> {
        let params = HoverParams {
            text_document: doc,
            position,
        };

        let response: serde_json::Value = self.send_request("textDocument/hover", &params).await?;

        if response.is_null() {
            return Ok(None);
        }

        serde_json::from_value(response)
            .map_err(|e| AppError::Internal(format!("解析悬停结果失败: {}", e)))
    }

    /// 获取跳转定义
    pub async fn get_definition(
        &mut self,
        doc: TextDocumentIdentifier,
        position: Position,
    ) -> Result<Vec<Location>, AppError> {
        let params = DefinitionParams {
            text_document: doc,
            position,
        };

        let response: serde_json::Value = self.send_request("textDocument/definition", &params).await?;

        if response.is_null() {
            return Ok(vec![]);
        }

        // 支持单个 Location 或 Location 数组
        if let Some(locations) = response.as_array() {
            let locations: Vec<Location> = serde_json::from_value(serde_json::Value::Array(locations.clone()))
                .map_err(|e| AppError::Internal(format!("解析定义结果失败: {}", e)))?;
            Ok(locations)
        } else {
            let location: Location = serde_json::from_value(response)
                .map_err(|e| AppError::Internal(format!("解析定义结果失败: {}", e)))?;
            Ok(vec![location])
        }
    }

    /// 获取缓存的诊断信息
    pub async fn get_cached_diagnostics(
        &self,
        file_path: &str,
    ) -> Result<Vec<Diagnostic>, AppError> {
        let uri = format!("file://{}", file_path.replace('\\', "/"));
        Ok(self.diagnostics_cache.get(&uri).cloned().unwrap_or_default())
    }

    /// 关闭语言服务器
    pub async fn shutdown(&mut self) -> Result<(), AppError> {
        if self.initialized {
            let _ = self.send_request("shutdown", &serde_json::Value::Null).await;
            let _ = self.send_notification("exit", &serde_json::Value::Null).await;
        }

        if let Some(ref mut child) = self.child {
            let _ = child.kill().await;
        }

        Ok(())
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.start_kill();
        }
    }
}

/// 根据文件路径检测语言 ID
fn detect_language_from_path(file_path: &str) -> String {
    let ext = std::path::Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "rs" => "rust",
        "py" => "python",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "hpp" | "cc" => "c++",
        "cs" => "csharp",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kt" => "kotlin",
        "lua" => "lua",
        "sql" => "sql",
        "html" => "html",
        "css" => "css",
        "scss" => "scss",
        "json" => "json",
        "xml" => "xml",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "md" => "markdown",
        "sh" | "bash" => "shellscript",
        "ps1" => "powershell",
        "dockerfile" => "dockerfile",
        _ => "plaintext",
    }
    .to_string()
}