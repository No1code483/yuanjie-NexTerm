// LSP (Language Server Protocol) Client Module
// 对标 VSCode 的 LSP 集成，提供语言服务器管理、代码补全、诊断等功能

pub mod client;
pub mod protocol;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::error::app_error::AppError;

use self::client::LspClient;
use self::protocol::{
    CompletionItem, CompletionParams, Diagnostic, HoverResult,
    Location, Position, TextDocumentIdentifier,
};

/// 语言服务器管理器
/// 负责检测、启动、管理多个语言服务器进程
pub struct LspManager {
    /// 已启动的语言服务器客户端 (language_id -> client)
    clients: HashMap<String, Arc<Mutex<LspClient>>>,
    /// 语言到语言服务器的映射
    language_servers: HashMap<String, LanguageServerConfig>,
}

/// 语言服务器配置
#[derive(Clone)]
pub struct LanguageServerConfig {
    /// 语言 ID (如 "rust", "python", "typescript")
    pub language_id: String,
    /// 可执行文件名称
    pub command: String,
    /// 启动参数
    pub args: Vec<String>,
    /// 文件扩展名列表
    pub extensions: Vec<String>,
    /// 初始化超时(秒)
    pub init_timeout_secs: u64,
}

impl LspManager {
    pub fn new() -> Self {
        let mut manager = Self {
            clients: HashMap::new(),
            language_servers: HashMap::new(),
        };
        manager.register_default_servers();
        manager
    }

    /// 注册默认语言服务器配置
    fn register_default_servers(&mut self) {
        let defaults = vec![
            LanguageServerConfig {
                language_id: "rust".into(),
                command: "rust-analyzer".into(),
                args: vec![],
                extensions: vec!["rs".into()],
                init_timeout_secs: 30,
            },
            LanguageServerConfig {
                language_id: "python".into(),
                command: "pyright-langserver".into(),
                args: vec!["--stdio".into()],
                extensions: vec!["py".into()],
                init_timeout_secs: 30,
            },
            LanguageServerConfig {
                language_id: "typescript".into(),
                command: "typescript-language-server".into(),
                args: vec!["--stdio".into()],
                extensions: vec!["ts".into(), "tsx".into(), "js".into(), "jsx".into()],
                init_timeout_secs: 30,
            },
            LanguageServerConfig {
                language_id: "go".into(),
                command: "gopls".into(),
                args: vec![],
                extensions: vec!["go".into()],
                init_timeout_secs: 30,
            },
        ];

        for config in defaults {
            self.language_servers
                .insert(config.language_id.clone(), config);
        }
    }

    /// 根据文件扩展名检测语言
    pub fn detect_language(&self, file_path: &str) -> Option<String> {
        let ext = std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        for (lang_id, config) in &self.language_servers {
            if config.extensions.iter().any(|e| e == ext) {
                return Some(lang_id.clone());
            }
        }
        None
    }

    /// 获取或启动语言服务器
    pub async fn get_or_start_client(
        &mut self,
        language_id: &str,
        workspace_root: &str,
    ) -> Result<Arc<Mutex<LspClient>>, AppError> {
        if let Some(client) = self.clients.get(language_id) {
            return Ok(client.clone());
        }

        let config = self
            .language_servers
            .get(language_id)
            .ok_or_else(|| AppError::Internal(format!("不支持的语言: {}", language_id)))?;

        let client: LspClient = LspClient::start(config, workspace_root).await?;
        let client = Arc::new(Mutex::new(client));
        self.clients
            .insert(language_id.to_string(), client.clone());
        Ok(client)
    }

    /// 获取代码补全
    pub async fn get_completions(
        &mut self,
        file_path: &str,
        line: u32,
        character: u32,
        workspace_root: &str,
    ) -> Result<Vec<CompletionItem>, AppError> {
        let language_id = match self.detect_language(file_path) {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        let client: Arc<Mutex<LspClient>> = self.get_or_start_client(&language_id, workspace_root).await?;
        let mut client = client.lock().await;

        // 确保文件已打开
        client.did_open(file_path, "").await?;

        let params = CompletionParams {
            text_document: TextDocumentIdentifier {
                uri: format!("file://{}", file_path.replace('\\', "/")),
            },
            position: Position { line, character },
        };

        client.get_completions(params).await
    }

    /// 获取悬停提示
    pub async fn get_hover(
        &mut self,
        file_path: &str,
        line: u32,
        character: u32,
        workspace_root: &str,
    ) -> Result<Option<HoverResult>, AppError> {
        let language_id = match self.detect_language(file_path) {
            Some(id) => id,
            None => return Ok(None),
        };

        let client: Arc<Mutex<LspClient>> = self.get_or_start_client(&language_id, workspace_root).await?;
        let mut client = client.lock().await;

        client.did_open(file_path, "").await?;

        let position = Position { line, character };
        let doc = TextDocumentIdentifier {
            uri: format!("file://{}", file_path.replace('\\', "/")),
        };

        client.get_hover(doc, position).await
    }

    /// 获取跳转定义
    pub async fn get_definition(
        &mut self,
        file_path: &str,
        line: u32,
        character: u32,
        workspace_root: &str,
    ) -> Result<Vec<Location>, AppError> {
        let language_id = match self.detect_language(file_path) {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        let client: Arc<Mutex<LspClient>> = self.get_or_start_client(&language_id, workspace_root).await?;
        let mut client = client.lock().await;

        client.did_open(file_path, "").await?;

        let position = Position { line, character };
        let doc = TextDocumentIdentifier {
            uri: format!("file://{}", file_path.replace('\\', "/")),
        };

        client.get_definition(doc, position).await
    }

    /// 获取诊断信息
    pub async fn get_diagnostics(
        &mut self,
        file_path: &str,
        workspace_root: &str,
    ) -> Result<Vec<Diagnostic>, AppError> {
        let language_id = match self.detect_language(file_path) {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        let client: Arc<Mutex<LspClient>> = self.get_or_start_client(&language_id, workspace_root).await?;
        let client = client.lock().await;

        // 诊断是异步推送的，这里返回缓存结果
        client.get_cached_diagnostics(file_path).await
    }

    /// 关闭所有语言服务器
    pub async fn shutdown_all(&mut self) {
        for (_, client) in self.clients.drain() {
            let mut client = client.lock().await;
            let _ = client.shutdown().await;
        }
    }
}