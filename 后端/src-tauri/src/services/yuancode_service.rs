use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

use tauri::Emitter;
use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;

use sqlx::SqlitePool;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::crypto::mek_manager::MekManager;
use crate::db::repositories::yuancode_repo;
use crate::error::app_error::AppError;
use crate::models::yuancode::{
    CodeAnalysisRequest, CodeAnalysisResult, CodeCompletionRequest, CodeCompletionResult,
    CodeExecutionRequest, CodeExecutionResult, CodeIssue, CodeSnippet,
    CompletionItem, CopyMoveRequest, CreateItemRequest, DeleteItemRequest,
    DiffHunk, DiffLine, DiffRequest, DiffResult, FileContent, FileInfoRequest,
    FileInfoResult, FileTreeNode, FormatRequest, FormatResult, HighlightLine, HighlightRequest,
    HighlightResult, HighlightToken, ListFilesRequest, ReadFileRequest,
    RenameItemRequest, ReplaceFilesRequest, ReplaceRange, SaveSnippetRequest, SaveWorkspaceRequest,
    SearchFilesRequest, SearchMatch, UpdateSnippetRequest, WorkspaceSession,
    WorkspaceTab, WriteFileRequest,
};

const DEFAULT_EXCLUDE_PATTERNS: &[&str] = &[
    ".git", "node_modules", "target", "__pycache__", ".venv",
    "venv", ".idea", ".vscode", "dist", "build", ".DS_Store",
];

const LANGUAGE_EXTENSIONS: &[(&str, &str)] = &[
    ("rs", "rust"), ("py", "python"), ("js", "javascript"), ("ts", "typescript"),
    ("tsx", "tsx"), ("jsx", "jsx"), ("go", "go"), ("java", "java"),
    ("c", "c"), ("cpp", "c++"), ("h", "c"), ("hpp", "c++"),
    ("html", "html"), ("css", "css"), ("json", "json"), ("yaml", "yaml"),
    ("yml", "yaml"), ("toml", "toml"), ("md", "markdown"), ("sh", "bash"),
    ("bat", "batch"), ("ps1", "powershell"), ("sql", "sql"),
    ("vue", "vue"), ("svelte", "svelte"), ("rb", "ruby"),
    ("php", "php"), ("swift", "swift"), ("kt", "kotlin"),
    ("lua", "lua"), ("r", "r"), ("dart", "dart"),
];

fn detect_language(path: &str) -> Option<String> {
    let path_lower = path.to_lowercase();
    for (ext, lang) in LANGUAGE_EXTENSIONS {
        if path_lower.ends_with(&format!(".{}", ext)) {
            return Some(lang.to_string());
        }
    }
    let filename = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    match filename {
        "Dockerfile" => Some("dockerfile".into()),
        "Makefile" => Some("makefile".into()),
        "CMakeLists.txt" => Some("cmake".into()),
        _ => None,
    }
}

fn is_excluded(path: &Path, patterns: &[String]) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if DEFAULT_EXCLUDE_PATTERNS.contains(&name) {
        return true;
    }
    for pattern in patterns {
        if name.contains(pattern) || path.to_string_lossy().contains(pattern) {
            return true;
        }
    }
    false
}

pub fn list_files(req: ListFilesRequest) -> Result<Vec<FileTreeNode>, AppError> {
    let root = Path::new(&req.workspace_path);
    if !root.exists() {
        return Err(AppError::NotFound);
    }
    if !root.is_dir() {
        return Err(AppError::Validation("路径不是目录".into()));
    }

    let max_depth = req.depth.unwrap_or(usize::MAX);
    let exclude = req.exclude_patterns.unwrap_or_default();
    build_file_tree(root, root, 0, max_depth, &exclude)
}

fn build_file_tree(
    root: &Path,
    current: &Path,
    depth: usize,
    max_depth: usize,
    exclude: &[String],
) -> Result<Vec<FileTreeNode>, AppError> {
    let mut nodes = Vec::new();

    if depth > max_depth || is_excluded(current, exclude) {
        return Ok(nodes);
    }

    let entries = fs::read_dir(current).map_err(|e| {
        AppError::FileSystem(e)
    })?;

    let mut entry_list: Vec<_> = entries
        .filter_map(|e| e.ok())
        .collect();

    entry_list.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        b_is_dir.cmp(&a_is_dir).then_with(|| {
            a.file_name().to_string_lossy().to_lowercase()
                .cmp(&b.file_name().to_string_lossy().to_lowercase())
        })
    });

    for entry in entry_list {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| AppError::FileSystem(e))?;
        let name = entry.file_name().to_string_lossy().to_string();

        let relative_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string()
            .replace('\\', "/");

        if file_type.is_dir() {
            if !is_excluded(&path, exclude) {
                let children = build_file_tree(root, &path, depth + 1, max_depth, exclude)?;
                let metadata = fs::metadata(&path).ok();
                nodes.push(FileTreeNode {
                    name,
                    path: relative_path,
                    is_dir: true,
                    size: None,
                    modified_at: metadata.and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64),
                    children: if children.is_empty() { None } else { Some(children) },
                    language: None,
                });
            }
        } else {
            let metadata = fs::metadata(&path).ok();
            let language = detect_language(&relative_path);
            nodes.push(FileTreeNode {
                name,
                path: relative_path,
                is_dir: false,
                size: metadata.as_ref().map(|m| m.len()),
                modified_at: metadata.and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64),
                children: None,
                language,
            });
        }
    }

    Ok(nodes)
}

pub fn read_file(req: ReadFileRequest) -> Result<FileContent, AppError> {
    let path = Path::new(&req.path);
    if !path.exists() {
        return Err(AppError::NotFound);
    }
    if !path.is_file() {
        return Err(AppError::Validation("路径不是文件".into()));
    }

    let metadata = fs::metadata(path).map_err(|e| AppError::FileSystem(e))?;
    let size = metadata.len();
    let modified_at = metadata.modified().ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);

    let mut file = fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
    let mut content = String::new();
    file.read_to_string(&mut content).map_err(|e| AppError::FileSystem(e))?;

    let language = detect_language(&req.path);
    let all_lines: Vec<&str> = content.lines().collect();
    let line_count = all_lines.len();

    let offset = req.offset_line.unwrap_or(0).min(line_count);
    let limit = req.limit_lines.unwrap_or(line_count);

    let selected = if limit == 0 || offset >= line_count {
        String::new()
    } else {
        let end = (offset + limit).min(line_count);
        all_lines[offset..end].join("\n")
    };

    Ok(FileContent {
        path: req.path,
        content: if req.offset_line.is_some() || req.limit_lines.is_some() {
            selected
        } else {
            content
        },
        size,
        modified_at,
        language,
        line_count: if req.offset_line.is_some() || req.limit_lines.is_some() {
            (limit).min(line_count.saturating_sub(offset))
        } else {
            line_count
        },
    })
}

pub fn write_file(req: WriteFileRequest) -> Result<(), AppError> {
    let path = Path::new(&req.path);

    if req.create_dirs.unwrap_or(true) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::FileSystem(e))?;
        }
    }

    fs::write(path, &req.content).map_err(|e| AppError::FileSystem(e))?;
    Ok(())
}

pub fn create_item(req: CreateItemRequest) -> Result<(), AppError> {
    let parent = Path::new(&req.parent_path);
    if !parent.exists() || !parent.is_dir() {
        return Err(AppError::Validation("父路径不存在或不是目录".into()));
    }

    let full_path = parent.join(&req.name);
    if full_path.exists() {
        return Err(AppError::Validation("文件或目录已存在".into()));
    }

    if req.is_dir {
        fs::create_dir(&full_path).map_err(|e| AppError::FileSystem(e))?;
    } else {
        fs::write(&full_path, "").map_err(|e| AppError::FileSystem(e))?;
    }

    Ok(())
}

pub fn delete_item(req: DeleteItemRequest) -> Result<(), AppError> {
    let path = Path::new(&req.path);
    if !path.exists() {
        return Err(AppError::NotFound);
    }

    let permanently = req.permanently.unwrap_or(false);

    if permanently {
        if path.is_dir() {
            fs::remove_dir_all(path).map_err(|e| AppError::FileSystem(e))?;
        } else {
            fs::remove_file(path).map_err(|e| AppError::FileSystem(e))?;
        }
    } else {
        trash::delete(path).map_err(|e| {
            AppError::Internal(format!("移入回收站失败: {}", e))
        })?;
    }

    Ok(())
}

pub fn rename_item(req: RenameItemRequest) -> Result<(), AppError> {
    let path = Path::new(&req.path);
    if !path.exists() {
        return Err(AppError::NotFound);
    }

    let parent = path.parent().unwrap_or(Path::new("."));
    let new_path = parent.join(&req.new_name);
    if new_path.exists() {
        return Err(AppError::Validation("目标名称已存在".into()));
    }

    fs::rename(path, &new_path).map_err(|e| AppError::FileSystem(e))?;
    Ok(())
}

pub fn highlight_code(req: HighlightRequest) -> Result<HighlightResult, AppError> {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ss
        .find_syntax_by_token(&req.language)
        .or_else(|| ss.find_syntax_by_extension(&req.language))
        .ok_or_else(|| AppError::Validation(format!("不支持的语言: {}", req.language)))?;

    let theme = &ts.themes["InspiredGitHub"];
    let mut result_lines = Vec::new();

    for (line_idx, line) in req.content.lines().enumerate() {
        let mut h = HighlightLines::new(syntax, theme);
        let regions = h.highlight_line(line, &ss)
            .map_err(|e| AppError::Validation(format!("高亮解析失败: {}", e)))?;
        let mut tokens = Vec::new();
        for &(style, text) in &regions {
            let fg = style.foreground;
            tokens.push(HighlightToken {
                text: text.to_string(),
                scope: String::new(),
                color: Some(format!("#{:02x}{:02x}{:02x}", fg.r, fg.g, fg.b)),
                bold: Some(style.font_style.contains(FontStyle::BOLD)),
                italic: Some(style.font_style.contains(FontStyle::ITALIC)),
                underline: Some(style.font_style.contains(FontStyle::UNDERLINE)),
            });
        }
        result_lines.push(HighlightLine {
            line_number: line_idx + 1,
            tokens,
        });
    }

    Ok(HighlightResult {
        language: req.language,
        lines: result_lines,
    })
}

pub async fn execute_code(req: CodeExecutionRequest) -> Result<CodeExecutionResult, AppError> {
    let timeout_secs = req.timeout_seconds.unwrap_or(30).min(300);
    let working_dir = req.working_dir.as_deref().unwrap_or(".");

    let (program, args) = match req.language.to_lowercase().as_str() {
        "python" | "py" => determine_python_command(),
        "javascript" | "js" => ("node".to_string(), vec!["-e".to_string(), req.code.clone()]),
        "typescript" | "ts" => ("npx".to_string(), vec!["ts-node".to_string(), "-e".to_string(), req.code.clone()]),
        "rust" | "rs" => return execute_rust_code(&req).await,
        "go" => {
            let tmp = std::env::temp_dir().join(format!("yuan_code_{}.go", Uuid::new_v4()));
            fs::write(&tmp, &req.code).map_err(|e| AppError::FileSystem(e))?;
            ("go".to_string(), vec!["run".to_string(), tmp.to_string_lossy().to_string()])
        }
        "sh" | "bash" => ("bash".to_string(), vec!["-c".to_string(), req.code.clone()]),
        "bat" | "batch" | "cmd" => ("cmd".to_string(), vec!["/C".to_string(), req.code.clone()]),
        "powershell" | "ps1" => ("powershell".to_string(), vec!["-Command".to_string(), req.code.clone()]),
        "java" => return execute_java_code(&req).await,
        _ => return Err(AppError::Validation(format!("不支持的运行语言: {}", req.language))),
    };

    run_command(&program, &args, working_dir, timeout_secs, &req.env_vars).await
}

fn determine_python_command() -> (String, Vec<String>) {
    if Command::new("python3").arg("--version").output().is_ok() {
        ("python3".into(), vec!["-c".to_string()])
    } else if Command::new("python").arg("--version").output().is_ok() {
        ("python".into(), vec!["-c".to_string()])
    } else {
        ("python".into(), vec!["-c".to_string()])
    }
}

async fn execute_rust_code(req: &CodeExecutionRequest) -> Result<CodeExecutionResult, AppError> {
    let tmp_dir = std::env::temp_dir().join(format!("yuan_rust_{}", Uuid::new_v4()));
    fs::create_dir_all(&tmp_dir).map_err(|e| AppError::FileSystem(e))?;

    let src_dir = tmp_dir.join("src");
    fs::create_dir_all(&src_dir).map_err(|e| AppError::FileSystem(e))?;

    fs::write(src_dir.join("main.rs"), &req.code).map_err(|e| AppError::FileSystem(e))?;

    let cargo_toml = r#"[package]
name = "yuan_temp"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(tmp_dir.join("Cargo.toml"), cargo_toml).map_err(|e| AppError::FileSystem(e))?;

    let timeout_secs = req.timeout_seconds.unwrap_or(60).min(300);

    let result = run_command(
        "cargo",
        &["run".into(), "--quiet".into()],
        &tmp_dir.to_string_lossy(),
        timeout_secs,
        &req.env_vars,
    ).await;

    let _ = fs::remove_dir_all(&tmp_dir);
    result
}

async fn execute_java_code(req: &CodeExecutionRequest) -> Result<CodeExecutionResult, AppError> {
    let tmp_dir = std::env::temp_dir().join(format!("yuan_java_{}", Uuid::new_v4()));
    fs::create_dir_all(&tmp_dir).map_err(|e| AppError::FileSystem(e))?;

    let java_file = tmp_dir.join("Main.java");
    fs::write(&java_file, &req.code).map_err(|e| AppError::FileSystem(e))?;

    let timeout_secs = req.timeout_seconds.unwrap_or(30).min(300);
    let working_dir = tmp_dir.to_string_lossy().to_string();

    let compile = run_command("javac", &["Main.java".into()], &working_dir, 30, &req.env_vars).await?;
    if compile.exit_code != 0 {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Ok(compile);
    }

    let result = run_command("java", &["Main".into()], &working_dir, timeout_secs, &req.env_vars).await;
    let _ = fs::remove_dir_all(&tmp_dir);
    result
}

const DEFAULT_MAX_OUTPUT_BYTES: usize = 100_000;

async fn read_stream_capped(
    stream: Option<impl AsyncReadExt + Unpin>,
    max_len: usize,
) -> (String, usize, bool) {
    let mut reader = match stream {
        Some(r) => r,
        None => return (String::new(), 0, false),
    };

    let mut buf = vec![0u8; 8192];
    let mut data = Vec::new();
    let mut truncated = false;

    loop {
        match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                let remaining = max_len.saturating_sub(data.len());
                if remaining == 0 {
                    truncated = true;
                    continue;
                }
                let to_take = n.min(remaining);
                data.extend_from_slice(&buf[..to_take]);
                if to_take < n {
                    truncated = true;
                }
            }
            Err(_) => break,
        }
    }

    let total_read = data.len();
    (String::from_utf8_lossy(&data).to_string(), total_read, truncated)
}

async fn run_command(
    program: &str,
    args: &[String],
    working_dir: &str,
    timeout_secs: u64,
    env_vars: &Option<HashMap<String, String>>,
) -> Result<CodeExecutionResult, AppError> {
    let start = Instant::now();

    let mut cmd = TokioCommand::new(program);
    cmd.args(args);
    cmd.current_dir(working_dir);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.stdin(std::process::Stdio::null());
    if let Some(env) = env_vars {
        for (k, v) in env {
            cmd.env(k, v);
        }
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return Err(AppError::TerminalError(format!("启动进程失败: {}", e))),
    };

    let child_stdout = child.stdout.take();
    let child_stderr = child.stderr.take();

    let max_len = DEFAULT_MAX_OUTPUT_BYTES;
    let (stdout_result, stderr_result, wait_result) = tokio::join!(
        read_stream_capped(child_stdout, max_len),
        read_stream_capped(child_stderr, max_len),
        child.wait(),
    );

    let (stdout, _stdout_bytes, stdout_truncated) = stdout_result;
    let (stderr, _stderr_bytes, stderr_truncated) = stderr_result;

    let duration_ms = start.elapsed().as_millis() as u64;

    let timed_out = if duration_ms >= timeout_secs * 1000 {
        true
    } else {
        false
    };

    match wait_result {
        Ok(status) => {
            let exit_code = status.code().unwrap_or(-1);
            Ok(CodeExecutionResult {
                exit_code,
                stdout,
                stderr,
                duration_ms,
                timed_out,
                truncated: stdout_truncated || stderr_truncated,
            })
        }
        Err(e) => Err(AppError::TerminalError(format!("等待进程退出失败: {}", e))),
    }
}

pub async fn ai_complete(
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    user_id: i64,
    req: CodeCompletionRequest,
) -> Result<CodeCompletionResult, AppError> {
    // D1.3：查询项目级上下文（imports + 相关符号）
    let project_ctx = match &req.file_path {
        Some(path) => fetch_project_context(pool, path).await.ok().flatten(),
        None => None,
    };

    let prompt = build_completion_prompt(&req, &project_ctx);
    let system_prompt = "你是一个专业的代码补全助手。根据上下文给出简洁的代码补全建议。每个补全建议用 ``` 代码块包裹，一行一个。只补全光标位置。优先复用项目内已有符号，遵循项目代码风格。";

    let model_id = resolve_model_id(pool, user_id, req.model_id).await?;
    let ai_service = crate::services::ai_model_service::AiModelService::new();
    let response = ai_service
        .call_model(pool, mek_manager, user_id, model_id, &prompt, Some(system_prompt))
        .await?;

    let completions = parse_completions(&response, &req);

    let model_name = crate::db::repositories::ai_repo::get_model_by_id(pool, model_id, user_id)
        .await?
        .map(|m| m.name)
        .unwrap_or_else(|| "unknown".to_string());

    Ok(CodeCompletionResult {
        completions,
        model_used: model_name,
    })
}

/// D1.2 流式补全
///
/// 设计：复用 AiModelService::call_model_stream，emit 事件 `ai-stream`，
///      使用 conversation_id = -1 标识 yuan code 流式补全（前端据此过滤）。
///
/// 流程：
/// 1. 查询项目级上下文（同 ai_complete）
/// 2. 构造 prompt + system_prompt
/// 3. 调用 call_model_stream，逐 chunk emit 到前端
/// 4. 返回完整响应（前端在流式过程中已渲染 ghost text）
pub async fn ai_complete_stream(
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    user_id: i64,
    req: CodeCompletionRequest,
    app_handle: &tauri::AppHandle,
) -> Result<CodeCompletionResult, AppError> {
    // 1. 项目级上下文
    let project_ctx = match &req.file_path {
        Some(path) => fetch_project_context(pool, path).await.ok().flatten(),
        None => None,
    };

    // 2. 构造 prompt
    let prompt = build_completion_prompt(&req, &project_ctx);
    let system_prompt = "你是一个专业的代码补全助手。根据上下文给出简洁的代码补全建议。每个补全建议用 ``` 代码块包裹，一行一个。只补全光标位置。优先复用项目内已有符号，遵循项目代码风格。";

    // 3. 解析 model_id
    let model_id = resolve_model_id(pool, user_id, req.model_id).await?;

    // 4. 调用流式 API（conversation_id = -1 标识 yuan code）
    let ai_service = crate::services::ai_model_service::AiModelService::new();
    let full_response = ai_service
        .call_model_stream(pool, mek_manager, user_id, model_id, &prompt, Some(system_prompt), app_handle, -1, None)
        .await?;

    // 5. 解析补全项
    let completions = parse_completions(&full_response, &req);

    let model_name = crate::db::repositories::ai_repo::get_model_by_id(pool, model_id, user_id)
        .await?
        .map(|m| m.name)
        .unwrap_or_else(|| "unknown".to_string());

    // 6. emit 完成事件
    let _ = app_handle.emit("yuan-code-stream-done", serde_json::json!({
        "event": "done",
        "model_used": model_name,
    }));

    Ok(CodeCompletionResult {
        completions,
        model_used: model_name,
    })
}

/// D1.3 项目级上下文：当前文件 imports + 项目内可引用符号
#[derive(Debug, Clone, Default)]
struct ProjectContext {
    /// 当前文件依赖的路径（imports）
    imports: Vec<String>,
    /// 项目内可引用的 top-level 符号（name/kind/file_path/line）
    symbols: Vec<ProjectSymbolHint>,
}

#[derive(Debug, Clone)]
struct ProjectSymbolHint {
    name: String,
    kind: String,
    file_path: String,
    line: i64,
    /// 保留字段：从 SQL 读取但当前未在补全逻辑中消费，保留以便未来按导出状态过滤
    #[allow(dead_code)]
    is_exported: bool,
}

/// D1.3 查询项目上下文
///
/// 流程：
/// 1. 通过 file_path 反查 project_index_files → project_id
/// 2. 加载当前文件的 imports（提示 AI 项目内已有依赖）
/// 3. 加载项目内导出符号（is_exported=1），最多 30 个，过滤当前文件自身的符号
///
/// 失败时静默返回 None（项目索引是可选的增强，不应阻塞补全）
async fn fetch_project_context(
    pool: &SqlitePool,
    file_path: &str,
) -> Result<Option<ProjectContext>, AppError> {
    // 标准化路径（Windows 反斜杠 → 正斜杠）
    let normalized = file_path.replace('\\', "/");

    // 1. 查找文件对应的 project_id + file_id
    let file_row: Option<(i64, i64, String)> = sqlx::query_as::<_, (i64, i64, String)>(
        "SELECT id, project_id, path FROM project_index_files WHERE path = ? OR absolute_path = ? LIMIT 1",
    )
    .bind(&normalized)
    .bind(file_path)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("查询文件索引失败: {}", e)))?;

    let (file_id, project_id, _path) = match file_row {
        Some(row) => row,
        None => return Ok(None), // 文件未被索引
    };

    // 2. 加载当前文件的 imports（imported_path 列表）
    let imports: Vec<String> = sqlx::query_as::<_, (String,)>(
        "SELECT DISTINCT imported_path FROM project_index_imports WHERE file_id = ? ORDER BY line_number LIMIT 50",
    )
    .bind(file_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(format!("查询文件 imports 失败: {}", e)))?
    .into_iter()
    .map(|(p,)| p)
    .collect();

    // 3. 加载项目内导出符号（排除当前文件，最多 30 个）
    let symbol_rows: Vec<(String, String, String, i64, i64)> = sqlx::query_as::<_, (String, String, String, i64, i64)>(
        "SELECT s.name, s.kind, f.path, s.line_start, s.is_exported \
         FROM project_index_symbols s \
         JOIN project_index_files f ON s.file_id = f.id \
         WHERE f.project_id = ? AND s.file_id != ? AND s.is_exported = 1 \
         ORDER BY s.name LIMIT 30",
    )
    .bind(project_id)
    .bind(file_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(format!("查询项目符号失败: {}", e)))?;

    let symbols = symbol_rows
        .into_iter()
        .map(|(name, kind, file_path, line, is_exported)| ProjectSymbolHint {
            name,
            kind,
            file_path,
            line,
            is_exported: is_exported != 0,
        })
        .collect();

    Ok(Some(ProjectContext { imports, symbols }))
}

/// 解析 model_id：若提供则直接使用，否则回退到统一模型管理中第一个可用模型
///
/// 安全审计修复（多用户隔离批次 1）：添加 user_id 参数，确保只查询当前用户的模型。
async fn resolve_model_id(
    pool: &SqlitePool,
    user_id: i64,
    model_id: Option<i64>,
) -> Result<i64, AppError> {
    if let Some(id) = model_id {
        return Ok(id);
    }
    let models = crate::db::repositories::ai_repo::get_all_models(pool, user_id).await?;
    let model = models.first().ok_or_else(|| {
        AppError::Validation("未配置任何 AI 模型，请先在统一模型管理中添加".into())
    })?;
    Ok(model.id)
}

fn build_completion_prompt(req: &CodeCompletionRequest, project_ctx: &Option<ProjectContext>) -> String {
    let mut prompt = String::new();
    prompt.push_str("你是一个代码补全助手。根据上下文给出简洁的代码补全建议。\n");
    prompt.push_str("每个补全建议用 ``` 代码块包裹，一行一个。只补全光标位置。\n\n");

    // === 文件元信息 ===
    if let Some(path) = &req.file_path {
        prompt.push_str(&format!("文件路径: {}\n", path));
    }
    prompt.push_str(&format!("语言: {}\n", req.language));
    prompt.push_str(&format!("光标位置: 第{}行, 第{}列\n\n", req.cursor_line + 1, req.cursor_column + 1));

    // === D1.3 项目级上下文 ===
    if let Some(ctx) = project_ctx {
        if !ctx.imports.is_empty() {
            prompt.push_str("--- 当前文件依赖（imports）---\n");
            for imp in &ctx.imports {
                prompt.push_str(&format!("- {}\n", imp));
            }
            prompt.push_str("\n");
        }

        if !ctx.symbols.is_empty() {
            prompt.push_str("--- 项目内可引用符号（优先复用）---\n");
            for sym in &ctx.symbols {
                prompt.push_str(&format!(
                    "- {} ({}) @ {}:{}\n",
                    sym.name, sym.kind, sym.file_path, sym.line
                ));
            }
            prompt.push_str("\n");
        }
    }

    // === 上下文代码 ===
    if let Some(ctx) = &req.context_before {
        prompt.push_str("--- 光标前代码 ---\n");
        prompt.push_str(ctx);
        prompt.push_str("\n");
    }

    prompt.push_str("【光标在此】\n");

    if let Some(ctx) = &req.context_after {
        prompt.push_str("--- 光标后代码 ---\n");
        prompt.push_str(ctx);
        prompt.push_str("\n");
    }

    prompt.push_str("\n请给出 1-3 个补全建议：\n");
    prompt.push_str("1. 优先复用上方「项目内可引用符号」中已存在的符号\n");
    prompt.push_str("2. 遵循项目代码风格（命名/缩进/导入方式）\n");
    prompt.push_str("3. 保持简洁，只补全光标位置\n");
    prompt
}

fn parse_completions(response: &str, req: &CodeCompletionRequest) -> Vec<CompletionItem> {
    let mut completions = Vec::new();

    let mut in_code_block = false;
    let mut current = String::new();

    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_code_block {
                if !current.trim().is_empty() {
                    completions.push(CompletionItem {
                        text: current.trim().to_string(),
                        display_text: None,
                        description: None,
                        replace_range: Some(ReplaceRange {
                            start_line: req.cursor_line,
                            start_column: req.cursor_column,
                            end_line: req.cursor_line,
                            end_column: req.cursor_column,
                        }),
                    });
                }
                current.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        } else {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("补全") && !trimmed.starts_with("建议") && !trimmed.starts_with("以下") {
                if trimmed.len() < 200 && !trimmed.starts_with("```") {
                    completions.push(CompletionItem {
                        text: trimmed.to_string(),
                        display_text: None,
                        description: None,
                        replace_range: Some(ReplaceRange {
                            start_line: req.cursor_line,
                            start_column: req.cursor_column,
                            end_line: req.cursor_line,
                            end_column: req.cursor_column,
                        }),
                    });
                }
            }
        }
    }

    if !current.trim().is_empty() {
        completions.push(CompletionItem {
            text: current.trim().to_string(),
            display_text: None,
            description: None,
            replace_range: Some(ReplaceRange {
                start_line: req.cursor_line,
                start_column: req.cursor_column,
                end_line: req.cursor_line,
                end_column: req.cursor_column,
            }),
        });
    }

    completions
}

pub async fn ai_inline_edit(
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    user_id: i64,
    selected_code: &str,
    instruction: &str,
    language: &str,
    file_path: &str,
    model_id: Option<i64>,
) -> Result<String, AppError> {
    let prompt = build_inline_edit_prompt(selected_code, instruction, language, file_path);
    let system_prompt = "You are a code editor. Return only the modified code. Do not add explanations, comments, or markdown code blocks. Just output the raw modified code.";

    let model_id = resolve_model_id(pool, user_id, model_id).await?;
    let ai_service = crate::services::ai_model_service::AiModelService::new();
    let response = ai_service
        .call_model(pool, mek_manager, user_id, model_id, &prompt, Some(system_prompt))
        .await?;

    extract_code_from_response(&response)
}

fn build_inline_edit_prompt(
    selected_code: &str,
    instruction: &str,
    language: &str,
    file_path: &str,
) -> String {
    format!(
        "You are a code editor. Given the selected code and the user's instruction, return only the modified code. Do not add explanations, comments about the changes, or markdown code blocks. Just output the raw modified code.\n\n\
         File: {file_path}\n\
         Language: {language}\n\n\
         --- Original Code ---\n\
         {selected_code}\n\
         --- End Original Code ---\n\n\
         Instruction: {instruction}\n\n\
         Return ONLY the modified code (no markdown, no explanations):",
        file_path = file_path,
        language = language,
        selected_code = selected_code,
        instruction = instruction,
    )
}

fn extract_code_from_response(response: &str) -> Result<String, AppError> {
    let trimmed = response.trim();

    // Try to extract code from markdown code blocks
    if let Some(start) = trimmed.find("```") {
        let after_start = &trimmed[start + 3..];
        // Skip optional language identifier
        let code_start = after_start.find('\n').map(|n| n + 1).unwrap_or(0);
        let code_body = &after_start[code_start..];
        if let Some(end) = code_body.rfind("```") {
            return Ok(code_body[..end].trim().to_string());
        }
    }

    // No code block found, return the entire response as-is
    if trimmed.is_empty() {
        return Err(AppError::AiApi("AI 返回内容为空".into()));
    }

    Ok(trimmed.to_string())
}

pub async fn analyze_code(
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    user_id: i64,
    req: CodeAnalysisRequest,
) -> Result<CodeAnalysisResult, AppError> {
    let prompt = format!(
        "请分析以下 {} 代码，进行 {} 分析。\n\
         用 JSON 格式返回：\n\
         {{\n\
           \"summary\": \"整体分析摘要\",\n\
           \"issues\": [\n\
             {{\"line\": 行号或null, \"severity\": \"error/warning/info\", \"message\": \"问题描述\"}}\n\
           ],\n\
           \"suggestions\": [\"改进建议1\", \"改进建议2\"]\n\
         }}\n\n\
         代码：\n{}",
        req.language, req.analysis_type, req.code
    );
    let system_prompt = "你是一个代码分析专家。请用 JSON 格式返回分析结果，不要添加额外说明。";

    let model_id = resolve_model_id(pool, user_id, req.model_id).await?;
    let ai_service = crate::services::ai_model_service::AiModelService::new();
    let response = ai_service
        .call_model(pool, mek_manager, user_id, model_id, &prompt, Some(system_prompt))
        .await?;

    parse_analysis_response(&response)
}

/// Phase 3 §2.2.6 流式代码分析（新增）
///
/// 与 `analyze_code` 等价，但通过 `call_model_stream` 逐 chunk emit `ai-stream` 事件，
/// 前端可实时显示分析进度（JSON 逐步生成）。
///
/// 事件约定：
/// - 流式 chunk：emit `ai-stream` { conversation_id: -2, event: "chunk", chunk: "<delta>" }
///   （conversation_id = -2 标识 yuan code 代码分析流）
/// - 完成：emit `yuan-code-analyze-done` { event: "done" }
///
/// 参数：与 `analyze_code` 相同，额外传入 `app_handle` 用于 emit。
pub async fn analyze_code_stream(
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    user_id: i64,
    req: CodeAnalysisRequest,
    app_handle: &tauri::AppHandle,
) -> Result<CodeAnalysisResult, AppError> {
    let prompt = format!(
        "请分析以下 {} 代码，进行 {} 分析。\n\
         用 JSON 格式返回：\n\
         {{\n\
           \"summary\": \"整体分析摘要\",\n\
           \"issues\": [\n\
             {{\"line\": 行号或null, \"severity\": \"error/warning/info\", \"message\": \"问题描述\"}}\n\
           ],\n\
           \"suggestions\": [\"改进建议1\", \"改进建议2\"]\n\
         }}\n\n\
         代码：\n{}",
        req.language, req.analysis_type, req.code
    );
    let system_prompt = "你是一个代码分析专家。请用 JSON 格式返回分析结果，不要添加额外说明。";

    let model_id = resolve_model_id(pool, user_id, req.model_id).await?;
    let ai_service = crate::services::ai_model_service::AiModelService::new();
    // conversation_id = -2 标识 yuan code 代码分析流（与 ai_complete_stream 的 -1 区分）
    let response = ai_service
        .call_model_stream(
            pool,
            mek_manager,
            user_id,
            model_id,
            &prompt,
            Some(system_prompt),
            app_handle,
            -2,
            None,
        )
        .await?;

    let _ = app_handle.emit("yuan-code-analyze-done", serde_json::json!({
        "event": "done",
    }));

    parse_analysis_response(&response)
}

fn parse_analysis_response(response: &str) -> Result<CodeAnalysisResult, AppError> {
    let json_str = if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            &response[start..=end]
        } else {
            response
        }
    } else {
        response
    };

    let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap_or(serde_json::json!({
        "summary": response,
        "issues": [],
        "suggestions": []
    }));

    let issues = parsed["issues"].as_array()
        .map(|arr| {
            arr.iter().map(|item| CodeIssue {
                line: item["line"].as_u64().map(|n| n as usize),
                column: item["column"].as_u64().map(|n| n as usize),
                severity: item["severity"].as_str().unwrap_or("info").to_string(),
                message: item["message"].as_str().unwrap_or("").to_string(),
                rule: item["rule"].as_str().map(String::from),
            }).collect()
        })
        .unwrap_or_default();

    let suggestions = parsed["suggestions"].as_array()
        .map(|arr| {
            arr.iter().filter_map(|s| s.as_str().map(String::from)).collect()
        })
        .unwrap_or_default();

    Ok(CodeAnalysisResult {
        analysis_type: "general".to_string(),
        summary: parsed["summary"].as_str().unwrap_or(response).to_string(),
        issues,
        suggestions,
    })
}

pub async fn save_snippet(
    pool: &SqlitePool,
    user_id: i64,
    req: SaveSnippetRequest,
) -> Result<CodeSnippet, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let tags_str = req.tags.map(|t| t.join(","));
    yuancode_repo::create_snippet(
        pool,
        user_id,
        &req.name,
        &req.language,
        &req.code,
        req.description.as_deref(),
        tags_str.as_deref(),
        now,
    ).await
}

pub async fn get_snippets(
    pool: &SqlitePool,
    user_id: i64,
    language: Option<&str>,
) -> Result<Vec<CodeSnippet>, AppError> {
    yuancode_repo::get_snippets(pool, user_id, language).await
}

pub async fn update_snippet(
    pool: &SqlitePool,
    user_id: i64,
    req: UpdateSnippetRequest,
) -> Result<Option<CodeSnippet>, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let tags_str = req.tags.map(|t| t.join(","));
    yuancode_repo::update_snippet(
        pool,
        user_id,
        req.id,
        req.name.as_deref(),
        req.language.as_deref(),
        req.code.as_deref(),
        req.description.as_deref(),
        tags_str.as_deref(),
        now,
    ).await
}

pub async fn delete_snippet(pool: &SqlitePool, user_id: i64, id: i64) -> Result<bool, AppError> {
    yuancode_repo::delete_snippet(pool, user_id, id).await
}

pub async fn search_snippets(
    pool: &SqlitePool,
    user_id: i64,
    query: &str,
) -> Result<Vec<CodeSnippet>, AppError> {
    yuancode_repo::search_snippets(pool, user_id, query).await
}

pub fn compute_diff(req: DiffRequest) -> Result<DiffResult, AppError> {
    let context_lines = req.context_lines.unwrap_or(3);
    let old_lines: Vec<&str> = req.original_content.lines().collect();
    let new_lines: Vec<&str> = req.modified_content.lines().collect();

    let lcs = longest_common_subsequence(&old_lines, &new_lines);
    let hunks = build_diff_hunks(&old_lines, &new_lines, &lcs, context_lines);
    let unified_diff = format_unified_diff(&req.file_path, &hunks);

    Ok(DiffResult { hunks, unified_diff })
}

fn longest_common_subsequence(a: &[&str], b: &[&str]) -> Vec<(usize, usize)> {
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];

    for i in 1..=n {
        for j in 1..=m {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    let mut result = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            result.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.reverse();
    result
}

fn build_diff_hunks(
    old_lines: &[&str],
    new_lines: &[&str],
    lcs: &[(usize, usize)],
    context: usize,
) -> Vec<DiffHunk> {
    let mut ops = Vec::new();
    let mut oi = 0usize;
    let mut ni = 0usize;

    for &(ol, nl) in lcs {
        while oi < ol {
            ops.push(('D', Some(oi), None));
            oi += 1;
        }
        while ni < nl {
            ops.push(('A', None, Some(ni)));
            ni += 1;
        }
        ops.push(('E', Some(oi), Some(ni)));
        oi = ol + 1;
        ni = nl + 1;
    }

    while oi < old_lines.len() {
        ops.push(('D', Some(oi), None));
        oi += 1;
    }
    while ni < new_lines.len() {
        ops.push(('A', None, Some(ni)));
        ni += 1;
    }

    let mut hunks = Vec::new();
    let mut i = 0;
    while i < ops.len() {
        let start = i;
        let mut seen_change = ops[i].0 != 'E';
        while i < ops.len() {
            if ops[i].0 != 'E' {
                seen_change = true;
            }
            if i + 1 < ops.len() && ops[i].0 == 'E' && ops[i + 1].0 == 'E'
                && (i + 2 >= ops.len() || ops[i + 2].0 == 'E')
            {
                let ctx_count = i - start + 1;
                if ctx_count > context * 2 + 3 && seen_change {
                    break;
                }
            }
            i += 1;
        }

        let mut hunk_old_start = usize::MAX;
        let mut hunk_old_count = 0usize;
        let mut hunk_new_start = usize::MAX;
        let mut hunk_new_count = 0usize;
        let mut hunk_lines = Vec::new();

        for j in start..i {
            match ops[j] {
                ('D', Some(ol), None) => {
                    if hunk_old_start == usize::MAX { hunk_old_start = ol; }
                    hunk_old_count += 1;
                    hunk_lines.push(DiffLine {
                        kind: "delete".into(),
                        content: old_lines[ol].to_string(),
                        old_line: Some(ol + 1),
                        new_line: None,
                    });
                }
                ('A', None, Some(nl)) => {
                    if hunk_new_start == usize::MAX { hunk_new_start = nl; }
                    hunk_new_count += 1;
                    hunk_lines.push(DiffLine {
                        kind: "add".into(),
                        content: new_lines[nl].to_string(),
                        old_line: None,
                        new_line: Some(nl + 1),
                    });
                }
                ('E', Some(ol), Some(nl)) => {
                    if hunk_old_start == usize::MAX { hunk_old_start = ol; }
                    if hunk_new_start == usize::MAX { hunk_new_start = nl; }
                    hunk_old_count += 1;
                    hunk_new_count += 1;
                    hunk_lines.push(DiffLine {
                        kind: "equal".into(),
                        content: old_lines[ol].to_string(),
                        old_line: Some(ol + 1),
                        new_line: Some(nl + 1),
                    });
                }
                _ => {}
            }
        }

        if !hunk_lines.is_empty() {
            hunks.push(DiffHunk {
                old_start: hunk_old_start + 1,
                old_count: hunk_old_count,
                new_start: hunk_new_start + 1,
                new_count: hunk_new_count,
                lines: hunk_lines,
            });
        }
    }
    hunks
}

fn format_unified_diff(file_path: &Option<String>, hunks: &[DiffHunk]) -> String {
    let mut diff = String::new();
    let path = file_path.as_deref().unwrap_or("file");
    diff.push_str(&format!("--- a/{}\n", path));
    diff.push_str(&format!("+++ b/{}\n", path));

    for hunk in hunks {
        diff.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
        ));
        for line in &hunk.lines {
            match line.kind.as_str() {
                "delete" => diff.push_str(&format!("-{}\n", line.content)),
                "add" => diff.push_str(&format!("+{}\n", line.content)),
                _ => diff.push_str(&format!(" {}\n", line.content)),
            }
        }
    }
    diff
}

pub fn search_files(req: &SearchFilesRequest) -> Result<Vec<SearchMatch>, AppError> {
    use regex::Regex;

    let case_sensitive = req.case_sensitive.unwrap_or(false);
    let use_regex = req.use_regex.unwrap_or(false);
    let whole_word = req.whole_word.unwrap_or(false);
    let max_results = req.max_results.unwrap_or(200);

    let pattern = if use_regex {
        req.query.clone()
    } else {
        regex::escape(&req.query)
    };

    let pattern = if whole_word {
        format!(r"\b{}\b", pattern)
    } else {
        pattern
    };

    let re = if case_sensitive {
        Regex::new(&pattern)
    } else {
        Regex::new(&format!("(?i){}", pattern))
    }
    .map_err(|e| AppError::Validation(format!("无效的搜索模式: {}", e)))?;

    let include_filter = req.include_pattern.as_deref().map(|p| glob::Pattern::new(p)).transpose()
        .map_err(|e| AppError::Validation(format!("无效的包含模式: {}", e)))?;
    let exclude_filter = req.exclude_pattern.as_deref().map(|p| glob::Pattern::new(p)).transpose()
        .map_err(|e| AppError::Validation(format!("无效的排除模式: {}", e)))?;

    let mut results = Vec::new();
    search_dir(
        &req.workspace_path,
        &re,
        &include_filter,
        &exclude_filter,
        &mut results,
        max_results,
    )?;

    Ok(results)
}

fn search_dir(
    dir: &str,
    re: &regex::Regex,
    include_filter: &Option<glob::Pattern>,
    exclude_filter: &Option<glob::Pattern>,
    results: &mut Vec<SearchMatch>,
    max: usize,
) -> Result<(), AppError> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        if results.len() >= max {
            return Ok(());
        }

        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();

        if DEFAULT_EXCLUDE_PATTERNS.iter().any(|p| name == *p) {
            continue;
        }

        if path.is_dir() {
            search_dir(
                &path.to_string_lossy(),
                re,
                include_filter,
                exclude_filter,
                results,
                max,
            )?;
        } else if path.is_file() {
            // Check include filter
            if let Some(ref filter) = include_filter {
                if !filter.matches(&name) {
                    continue;
                }
            }
            // Check exclude filter
            if let Some(ref filter) = exclude_filter {
                if filter.matches(&name) {
                    continue;
                }
            }

            if let Ok(content) = fs::read_to_string(&path) {
                let file_path_str = path.to_string_lossy().to_string();
                for (line_idx, line) in content.lines().enumerate() {
                    if results.len() >= max {
                        return Ok(());
                    }
                    for mat in re.find_iter(line) {
                        if results.len() >= max {
                            return Ok(());
                        }
                        results.push(SearchMatch {
                            file: file_path_str.clone(),
                            file_path: file_path_str.clone(),
                            line: line_idx + 1,
                            line_number: line_idx + 1,
                            column: mat.start() + 1,
                            content: line.to_string(),
                            line_content: line.to_string(),
                            match_text: mat.as_str().to_string(),
                            match_start: mat.start(),
                            match_end: mat.end(),
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

/// 跨文件替换
pub fn replace_files(req: &ReplaceFilesRequest) -> Result<usize, AppError> {
    use regex::Regex;

    let case_sensitive = req.case_sensitive.unwrap_or(false);
    let use_regex = req.use_regex.unwrap_or(false);
    let whole_word = req.whole_word.unwrap_or(false);

    let pattern = if use_regex {
        req.query.clone()
    } else {
        regex::escape(&req.query)
    };

    let pattern = if whole_word {
        format!(r"\b{}\b", pattern)
    } else {
        pattern
    };

    let re = if case_sensitive {
        Regex::new(&pattern)
    } else {
        Regex::new(&format!("(?i){}", pattern))
    }
    .map_err(|e| AppError::Validation(format!("无效的替换模式: {}", e)))?;

    let include_filter = req.include_pattern.as_deref().map(|p| glob::Pattern::new(p)).transpose()
        .map_err(|e| AppError::Validation(format!("无效的包含模式: {}", e)))?;
    let exclude_filter = req.exclude_pattern.as_deref().map(|p| glob::Pattern::new(p)).transpose()
        .map_err(|e| AppError::Validation(format!("无效的排除模式: {}", e)))?;

    let mut total_replacements = 0;
    replace_in_dir(
        &req.workspace_path,
        &re,
        &req.replacement,
        &include_filter,
        &exclude_filter,
        &mut total_replacements,
    )?;

    Ok(total_replacements)
}

fn replace_in_dir(
    dir: &str,
    re: &regex::Regex,
    replacement: &str,
    include_filter: &Option<glob::Pattern>,
    exclude_filter: &Option<glob::Pattern>,
    total: &mut usize,
) -> Result<(), AppError> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();

        if DEFAULT_EXCLUDE_PATTERNS.iter().any(|p| name == *p) {
            continue;
        }

        if path.is_dir() {
            replace_in_dir(
                &path.to_string_lossy(),
                re,
                replacement,
                include_filter,
                exclude_filter,
                total,
            )?;
        } else if path.is_file() {
            if let Some(ref filter) = include_filter {
                if !filter.matches(&name) {
                    continue;
                }
            }
            if let Some(ref filter) = exclude_filter {
                if filter.matches(&name) {
                    continue;
                }
            }

            if let Ok(content) = fs::read_to_string(&path) {
                let new_content = re.replace_all(&content, replacement);
                if new_content != content {
                    fs::write(&path, new_content.as_bytes())
                        .map_err(|e| AppError::Internal(format!("写入文件失败: {}", e)))?;
                    *total += re.find_iter(&content).count();
                }
            }
        }
    }
    Ok(())
}

pub fn copy_move_item(req: &CopyMoveRequest) -> Result<(), AppError> {
    let src = Path::new(&req.source_path);
    if !src.exists() {
        return Err(AppError::FileSystem(
            std::io::Error::new(std::io::ErrorKind::NotFound, "源文件不存在")
        ));
    }

    let dest = Path::new(&req.dest_path);
    let overwrite = req.overwrite.unwrap_or(false);

    if dest.exists() && !overwrite {
        return Err(AppError::Validation("目标文件已存在".into()));
    }

    if let Some(parent) = dest.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(AppError::FileSystem)?;
        }
    }

    if req.is_move {
        fs::rename(src, dest).map_err(AppError::FileSystem)
    } else {
        if src.is_dir() {
            copy_dir_recursive(src, dest)?;
        } else {
            fs::copy(src, dest).map_err(AppError::FileSystem)?;
        }
        Ok(())
    }
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dest).map_err(AppError::FileSystem)?;
    for entry in fs::read_dir(src).map_err(AppError::FileSystem)? {
        let entry = entry.map_err(AppError::FileSystem)?;
        let dest_child = dest.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir_recursive(&entry.path(), &dest_child)?;
        } else {
            fs::copy(entry.path(), &dest_child).map_err(AppError::FileSystem)?;
        }
    }
    Ok(())
}

pub fn get_file_info(req: &FileInfoRequest) -> Result<FileInfoResult, AppError> {
    let path = Path::new(&req.path);
    if !path.exists() {
        return Err(AppError::FileSystem(
            std::io::Error::new(std::io::ErrorKind::NotFound, "路径不存在")
        ));
    }

    let metadata = fs::metadata(path).map_err(AppError::FileSystem)?;
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let modified_at = metadata.modified().ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);
    let created_at = metadata.created().ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);

    let (line_count, language) = if path.is_file() {
        let lc = fs::read_to_string(path).ok().map(|s| s.lines().count());
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let lang = LANGUAGE_EXTENSIONS.iter()
            .find(|(e, _)| *e == ext)
            .map(|(_, l)| l.to_string());
        (lc, lang)
    } else {
        (None, None)
    };

    Ok(FileInfoResult {
        path: req.path.clone(),
        name,
        is_dir: metadata.is_dir(),
        size: metadata.len(),
        modified_at,
        created_at,
        is_readonly: metadata.permissions().readonly(),
        line_count,
        language,
    })
}

pub async fn save_workspace(
    pool: &SqlitePool,
    user_id: i64,
    req: SaveWorkspaceRequest,
) -> Result<i64, AppError> {
    let tabs_json = serde_json::to_string(&req.tabs)
        .map_err(|e| AppError::Validation(format!("序列化失败: {}", e)))?;
    yuancode_repo::save_workspace(
        pool,
        user_id,
        &req.name,
        &req.workspace_path,
        &tabs_json,
        req.active_tab_index,
    )
    .await
}

pub async fn load_workspace(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
) -> Result<Option<WorkspaceSession>, AppError> {
    let row = yuancode_repo::load_workspace(pool, user_id, id).await?;
    match row {
        Some((id, name, workspace_path, tabs_json, active_tab_index, created_at, updated_at)) => {
            let tabs: Vec<WorkspaceTab> = serde_json::from_str(&tabs_json)
                .map_err(|e| AppError::Validation(format!("反序列化失败: {}", e)))?;
            Ok(Some(WorkspaceSession {
                id: Some(id),
                name,
                workspace_path,
                tabs,
                active_tab_index,
                created_at: Some(created_at),
                updated_at: Some(updated_at),
            }))
        }
        None => Ok(None),
    }
}

pub async fn list_workspaces(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<WorkspaceSession>, AppError> {
    let rows = yuancode_repo::list_workspaces(pool, user_id).await?;
    rows.into_iter()
        .map(|(id, name, workspace_path, created_at, updated_at)| {
            Ok(WorkspaceSession {
                id: Some(id),
                name,
                workspace_path,
                tabs: vec![],
                active_tab_index: 0,
                created_at: Some(created_at),
                updated_at: Some(updated_at),
            })
        })
        .collect()
}

pub async fn delete_workspace(pool: &SqlitePool, user_id: i64, id: i64) -> Result<bool, AppError> {
    yuancode_repo::delete_workspace(pool, user_id, id).await
}

/// 代码格式化：根据语言调用对应格式化器
pub async fn format_code(req: FormatRequest) -> Result<FormatResult, AppError> {
    let formatted = match req.language.as_str() {
        "rust" => format_with_command(&req.content, "rustfmt", &["--edition", "2021"])?,
        "python" => format_with_command(&req.content, "black", &["-c", "-"])?,
        "javascript" | "typescript" | "jsx" | "tsx" | "json" | "css" | "scss" | "html" | "markdown" => {
            format_with_prettier(&req.content, &req.language)?
        }
        "go" => format_with_command(&req.content, "gofmt", &[])?,
        _ => return Err(AppError::Validation(format!(
            "不支持格式化语言: {}", req.language
        ))),
    };

    let changed = formatted != req.content;

    Ok(FormatResult { formatted, changed })
}

/// 使用命令行格式化器（stdin -> stdout）
fn format_with_command(content: &str, cmd: &str, args: &[&str]) -> Result<String, AppError> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::Internal(format!("格式化器 {} 未找到: {}", cmd, e)))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(content.as_bytes())
            .map_err(|e| AppError::Internal(format!("写入格式化器失败: {}", e)))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| AppError::Internal(format!("格式化器执行失败: {}", e)))?;

    if !output.status.success() {
        let _stderr = String::from_utf8_lossy(&output.stderr);
        return Ok(content.to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// 使用 Prettier 格式化
fn format_with_prettier(content: &str, language: &str) -> Result<String, AppError> {
    let parser = match language {
        "typescript" | "tsx" => "typescript",
        "javascript" | "jsx" => "babel",
        "json" => "json",
        "css" => "css",
        "scss" => "scss",
        "html" => "html",
        "markdown" => "markdown",
        _ => "babel",
    };

    format_with_command(content, "prettier", &["--parser", parser, "--stdin-filepath", &format!("file.{}", language)])
}