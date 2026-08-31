// LSP 协议类型定义
// 对应 Language Server Protocol 规范的核心类型

use serde::{Deserialize, Serialize};

// ========== 基础类型 ==========

/// 位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// 行号 (0-based)
    pub line: u32,
    /// 字符偏移 (0-based, UTF-16 code units)
    pub character: u32,
}

/// 范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// 位置 (含 URI)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// 文本编辑
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

/// 文本文档标识符
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentIdentifier {
    pub uri: String,
}

// ========== Initialize ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    pub process_id: Option<u32>,
    pub root_uri: Option<String>,
    pub capabilities: ClientCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    pub text_document: Option<TextDocumentClientCapabilities>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentClientCapabilities {
    pub completion: Option<CompletionCapabilities>,
    pub hover: Option<HoverCapabilities>,
    pub definition: Option<DefinitionCapabilities>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionCapabilities {
    pub completion_item: Option<CompletionItemCapabilities>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItemCapabilities {
    pub snippet_support: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverCapabilities {
    pub content_format: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionCapabilities {
    pub link_support: Option<bool>,
}

// ========== textDocument/didOpen ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidOpenTextDocumentParams {
    pub text_document: TextDocumentItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentItem {
    pub uri: String,
    pub language_id: String,
    pub version: i32,
    pub text: String,
}

// ========== textDocument/completion ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionParams {
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    #[serde(default)]
    pub kind: Option<CompletionItemKind>,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub documentation: Option<String>,
    #[serde(default)]
    pub insert_text: Option<String>,
    #[serde(default)]
    pub insert_text_format: Option<InsertTextFormat>,
    #[serde(default)]
    pub text_edit: Option<TextEdit>,
    #[serde(default)]
    pub sort_text: Option<String>,
    #[serde(default)]
    pub filter_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(i32)]
pub enum CompletionItemKind {
    Text = 1,
    Method = 2,
    Function = 3,
    Constructor = 4,
    Field = 5,
    Variable = 6,
    Class = 7,
    Interface = 8,
    Module = 9,
    Property = 10,
    Unit = 11,
    Value = 12,
    Enum = 13,
    Keyword = 14,
    Snippet = 15,
    Color = 16,
    File = 17,
    Reference = 18,
    Folder = 19,
    EnumMember = 20,
    Constant = 21,
    Struct = 22,
    Event = 23,
    Operator = 24,
    TypeParameter = 25,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(i32)]
pub enum InsertTextFormat {
    PlainText = 1,
    Snippet = 2,
}

// ========== textDocument/hover ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverParams {
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverResult {
    pub contents: HoverContents,
    #[serde(default)]
    pub range: Option<Range>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HoverContents {
    String(String),
    MarkupContent {
        kind: String,
        value: String,
    },
    Array(Vec<HoverContents>),
}

// ========== textDocument/definition ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionParams {
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

// ========== textDocument/publishDiagnostics ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishDiagnosticsParams {
    pub uri: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    #[serde(default)]
    pub severity: Option<DiagnosticSeverity>,
    #[serde(default)]
    pub code: Option<DiagnosticCode>,
    pub message: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(i32)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DiagnosticCode {
    Number(i32),
    String(String),
}

// ========== 辅助类型 ==========

/// 诊断转为前端可用的格式
impl Diagnostic {
    pub fn to_frontend_format(&self) -> FrontendDiagnostic {
        FrontendDiagnostic {
            message: self.message.clone(),
            severity: match self.severity {
                Some(DiagnosticSeverity::Error) => "error".into(),
                Some(DiagnosticSeverity::Warning) => "warning".into(),
                Some(DiagnosticSeverity::Information) => "info".into(),
                Some(DiagnosticSeverity::Hint) => "hint".into(),
                None => "info".into(),
            },
            start_line: self.range.start.line,
            start_column: self.range.start.character,
            end_line: self.range.end.line,
            end_column: self.range.end.character,
            source: self.source.clone().unwrap_or_default(),
            code: self.code.as_ref().map(|c| match c {
                DiagnosticCode::Number(n) => n.to_string(),
                DiagnosticCode::String(s) => s.clone(),
            }).unwrap_or_default(),
        }
    }
}

/// 前端诊断格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendDiagnostic {
    pub message: String,
    pub severity: String,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub source: String,
    pub code: String,
}