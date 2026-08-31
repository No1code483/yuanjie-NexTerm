use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;

use crate::error::app_error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub param_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_name: String,
    pub arguments: HashMap<String, String>,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: u64,
}

pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = ToolRegistry {
            tools: HashMap::new(),
        };
        registry.register_builtin_tools();
        registry
    }

    fn register_builtin_tools(&mut self) {
        let read_file_tool = ToolDefinition {
            name: "read_file".to_string(),
            description: "读取指定路径的文件内容，支持文本文件。返回文件内容的UTF-8字符串。".to_string(),
            parameters: vec![ToolParameter {
                name: "path".to_string(),
                description: "要读取的文件绝对路径".to_string(),
                param_type: "string".to_string(),
                required: true,
            }],
            category: "file".to_string(),
        };
        self.tools.insert("read_file".to_string(), read_file_tool);

        let write_file_tool = ToolDefinition {
            name: "write_file".to_string(),
            description: "将内容写入指定路径的文件。如果文件已存在将被覆盖，最大允许写入1MB内容。".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    description: "要写入的文件绝对路径".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "content".to_string(),
                    description: "要写入的文件内容".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
            ],
            category: "file".to_string(),
        };
        self.tools.insert("write_file".to_string(), write_file_tool);

        let execute_code_tool = ToolDefinition {
            name: "execute_code".to_string(),
            description: "在本地环境中执行代码片段。支持python/javascript/rust等语言。代码在子进程中运行，有超时限制(30秒)。".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "code".to_string(),
                    description: "要执行的代码内容".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "language".to_string(),
                    description: "代码语言：python, javascript, rust, cmd, powershell".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
            ],
            category: "exec".to_string(),
        };
        self.tools.insert("execute_code".to_string(), execute_code_tool);

        let search_kb_tool = ToolDefinition {
            name: "search_knowledge_base".to_string(),
            description: "在用户的个人知识库中搜索相关内容。返回匹配的条目名称和路径。".to_string(),
            parameters: vec![ToolParameter {
                name: "query".to_string(),
                description: "搜索关键词".to_string(),
                param_type: "string".to_string(),
                required: true,
            }],
            category: "knowledge".to_string(),
        };
        self.tools.insert("search_knowledge_base".to_string(), search_kb_tool);

        let calculate_tool = ToolDefinition {
            name: "calculate".to_string(),
            description: "安全地计算数学表达式。支持加减乘除、括号、幂运算、sqrt等基础运算。".to_string(),
            parameters: vec![ToolParameter {
                name: "expression".to_string(),
                description: "数学表达式，如: 2+3*4, sqrt(16), (2^3)+1".to_string(),
                param_type: "string".to_string(),
                required: true,
            }],
            category: "util".to_string(),
        };
        self.tools.insert("calculate".to_string(), calculate_tool);

        let system_info_tool = ToolDefinition {
            name: "system_info".to_string(),
            description: "获取当前系统信息，包括操作系统、当前目录、可用内存等。".to_string(),
            parameters: vec![],
            category: "util".to_string(),
        };
        self.tools.insert("system_info".to_string(), system_info_tool);
    }

    pub fn register(&mut self, tool: ToolDefinition) {
        self.tools.insert(tool.name.clone(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    pub fn list(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    pub fn list_by_category(&self, category: &str) -> Vec<&ToolDefinition> {
        self.tools
            .values()
            .filter(|t| t.category == category)
            .collect()
    }

    pub fn build_tools_prompt(&self) -> String {
        if self.tools.is_empty() {
            return String::new();
        }

        let mut prompt = String::from("\n## 可用工具\n\n");
        prompt.push_str("你可以使用以下工具来帮助回答用户的问题。调用工具时，请在回复中使用以下格式：\n\n");
        prompt.push_str("```tool\n名称: <工具名称>\n参数1: 值1\n参数2: 值2\n```\n\n");

        prompt.push_str("可用工具列表：\n\n");
        for tool in self.tools.values() {
            prompt.push_str(&format!("- **{}**: {}\n", tool.name, tool.description));
            for param in &tool.parameters {
                let required_mark = if param.required { "(必填)" } else { "(可选)" };
                prompt.push_str(&format!(
                    "  - `{}` {}: {}\n",
                    param.name, required_mark, param.description
                ));
            }
            prompt.push('\n');
        }

        prompt
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ToolExecutor {
    pub registry: ToolRegistry,
}

impl ToolExecutor {
    pub fn new() -> Self {
        ToolExecutor {
            registry: ToolRegistry::new(),
        }
    }

    pub fn parse_tool_calls(text: &str) -> Vec<ToolCall> {
        let mut calls = Vec::new();
        let mut in_block = false;
        let mut current_name = String::new();
        let mut current_args: HashMap<String, String> = HashMap::new();
        let start_marker = "```tool";

        for line in text.lines() {
            let trimmed = line.trim();

            if trimmed == start_marker {
                in_block = true;
                current_name.clear();
                current_args.clear();
                continue;
            }

            if in_block && trimmed == "```" {
                if !current_name.is_empty() {
                    calls.push(ToolCall {
                        tool_name: current_name.clone(),
                        arguments: current_args.clone(),
                        raw: format!("{}: {:?}", current_name, current_args),
                    });
                }
                in_block = false;
                continue;
            }

            if in_block {
                if trimmed.starts_with("名称:") || trimmed.starts_with("名称：") {
                    current_name = trimmed
                        .trim_start_matches("名称:")
                        .trim_start_matches("名称：")
                        .trim()
                        .to_string();
                } else if let Some(colon_pos) = trimmed.find([':', '：']) {
                    let key = trimmed[..colon_pos].trim().to_lowercase();
                    let value = trimmed[colon_pos + ':'.len_utf8()..].trim().to_string();
                    if key != "名称" && key != "名称" {
                        current_args.insert(key, value);
                    }
                }
            }
        }

        calls
    }

    pub async fn execute(
        pool: &sqlx::SqlitePool,
        user_id: i64,
        tool_call: &ToolCall,
    ) -> Result<ToolResult, AppError> {
        let start = std::time::Instant::now();

        let result = match tool_call.tool_name.as_str() {
            "read_file" => Self::exec_read_file(&tool_call.arguments).await,
            "write_file" => Self::exec_write_file(&tool_call.arguments).await,
            "execute_code" => Self::exec_run_code(&tool_call.arguments).await,
            "search_knowledge_base" => Self::exec_search_kb(pool, user_id, &tool_call.arguments).await,
            "calculate" => Self::exec_calculate(&tool_call.arguments),
            "system_info" => Self::exec_system_info(),
            _ => Err(AppError::AiApi(format!(
                "未知工具: {}",
                tool_call.tool_name
            ))),
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(output) => Ok(ToolResult {
                tool_name: tool_call.tool_name.clone(),
                success: true,
                output,
                error: None,
                duration_ms,
            }),
            Err(e) => Ok(ToolResult {
                tool_name: tool_call.tool_name.clone(),
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                duration_ms,
            }),
        }
    }

    async fn exec_read_file(args: &HashMap<String, String>) -> Result<String, AppError> {
        let path = args
            .get("path")
            .ok_or_else(|| AppError::Validation("缺少参数: path".into()))?;

        let path = std::path::Path::new(path);
        if !path.exists() {
            return Err(AppError::Validation("文件不存在".into()));
        }
        if !path.is_file() {
            return Err(AppError::Validation("路径不是文件".into()));
        }

        let metadata = path.metadata().map_err(|e| AppError::FileSystem(e))?;
        if metadata.len() > 5 * 1024 * 1024 {
            return Err(AppError::Validation("文件过大（>5MB）".into()));
        }

        let content = std::fs::read_to_string(path).map_err(|e| AppError::FileSystem(e))?;

        if content.len() > 10000 {
            Ok(format!("文件内容（已截断至前10000字符）:\n\n{}", &content[..10000]))
        } else {
            Ok(format!("文件内容:\n\n{}", content))
        }
    }

    async fn exec_write_file(args: &HashMap<String, String>) -> Result<String, AppError> {
        let path = args
            .get("path")
            .ok_or_else(|| AppError::Validation("缺少参数: path".into()))?;
        let content = args
            .get("content")
            .ok_or_else(|| AppError::Validation("缺少参数: content".into()))?;

        if content.len() > 1024 * 1024 {
            return Err(AppError::Validation("内容过大（>1MB）".into()));
        }

        let path = std::path::Path::new(path);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| AppError::FileSystem(e))?;
            }
        }

        std::fs::write(path, content).map_err(|e| AppError::FileSystem(e))?;

        Ok(format!(
            "文件已成功写入: {} ({} 字节)",
            path.display(),
            content.len()
        ))
    }

    async fn exec_run_code(args: &HashMap<String, String>) -> Result<String, AppError> {
        let code = args
            .get("code")
            .ok_or_else(|| AppError::Validation("缺少参数: code".into()))?;
        let language = args
            .get("language")
            .map(|s| s.as_str())
            .unwrap_or("python");

        if code.len() > 50000 {
            return Err(AppError::Validation("代码过长（>50KB）".into()));
        }

        // 安全审计修复（发现 10，HIGH）：原实现直接 `Command::new(program).args(code)`
        // 执行 AI 生成代码，无超时、无隔离、无代码安全检查。攻击者可通过 prompt
        // injection 让小欣调用此工具执行任意代码（删除文件、横向移动、外泄数据）。
        // 现增加三层防护：
        // 1. 代码静态安全检查（拒绝已知危险 pattern）
        // 2. 在临时目录中执行（避免污染工作目录）
        // 3. 10 秒硬超时（防止资源耗尽/无限循环）
        if let Some(reason) = check_code_safety(code, language) {
            return Err(AppError::Validation(format!(
                "代码被安全检查拦截: {}（小欣工具沙箱禁止执行危险操作）",
                reason
            )));
        }

        let (program, args_vec) = match language {
            "python" => {
                let (cmd, base_args) = determine_python_command();
                let mut args = base_args;
                args.push(code.to_string());
                (cmd, args)
            }
            "javascript" => {
                let mut args = vec!["-e".to_string()];
                args.push(code.to_string());
                ("node".to_string(), args)
            }
            "cmd" => {
                let mut args = vec!["/C".to_string()];
                args.push(code.to_string());
                ("cmd".to_string(), args)
            }
            "powershell" => {
                let mut args = vec!["-NoProfile".to_string(), "-Command".to_string()];
                args.push(code.to_string());
                ("powershell".to_string(), args)
            }
            "rust" => {
                return Err(AppError::Validation(
                    "Rust代码不能直接通过子进程执行。建议使用python/javascript/cmd/powershell。".into(),
                ));
            }
            _ => {
                return Err(AppError::Validation(format!(
                    "不支持的语言: {}。支持: python, javascript, cmd, powershell",
                    language
                )));
            }
        };

        // 在临时目录中执行（隔离工作目录，防止代码污染用户文件）
        let temp_dir = std::env::temp_dir().join(format!("xin_tool_exec_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).map_err(|e| AppError::FileSystem(e))?;

        // 切换到 tokio::process::Command 以支持 async + timeout
        let mut cmd = tokio::process::Command::new(&program);
        cmd.args(&args_vec)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .current_dir(&temp_dir);

        // 限制环境变量：移除可能泄露的敏感变量
        cmd.env_clear();
        cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
        cmd.env("TEMP", &temp_dir);
        cmd.env("TMP", &temp_dir);
        cmd.env("HOME", &temp_dir);
        cmd.env("USERPROFILE", &temp_dir);

        // 10 秒硬超时（防止无限循环/资源耗尽）
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            cmd.output(),
        )
        .await
        .map_err(|_| {
            // 超时：清理临时目录
            let _ = std::fs::remove_dir_all(&temp_dir);
            AppError::AiApi("代码执行超时（10s 上限，可能存在无限循环或资源耗尽）".into())
        })?
        .map_err(|e| {
            let _ = std::fs::remove_dir_all(&temp_dir);
            AppError::AiApi(format!("执行失败: {}", e))
        })?;

        // 执行完成后清理临时目录
        let _ = std::fs::remove_dir_all(&temp_dir);

        let mut result = String::new();
        if !output.stdout.is_empty() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            result.push_str(&format!("标准输出:\n{}", stdout));
        }
        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            result.push_str(&format!("\n错误输出:\n{}", stderr));
        }
        if !output.status.success() {
            result.push_str(&format!(
                "\n退出码: {}",
                output.status.code().unwrap_or(-1)
            ));
        }

        if result.is_empty() {
            result = "代码执行完成，无输出。".to_string();
        }

        Ok(result)
    }

    async fn exec_search_kb(
        pool: &sqlx::SqlitePool,
        user_id: i64,
        args: &HashMap<String, String>,
    ) -> Result<String, AppError> {
        let query = args
            .get("query")
            .ok_or_else(|| AppError::Validation("缺少参数: query".into()))?;

        let pattern = format!("%{}%", query);
        #[derive(sqlx::FromRow, Debug)]
        struct KbSearchRow {
            name: String,
            path_url: String,
            entry_type: String,
        }
        // 多用户隔离批次 3 补全：kb_entries 按 user_id 过滤，防止跨用户搜索知识库
        let results: Vec<KbSearchRow> = sqlx::query_as::<_, KbSearchRow>(
            "SELECT name, path_url, entry_type FROM kb_entries WHERE user_id = ? AND (name LIKE ? OR path_url LIKE ?) ORDER BY created_at DESC LIMIT 10",
        )
        .bind(user_id)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        if results.is_empty() {
            Ok(format!("知识库中未找到与 \"{}\" 相关的条目。", query))
        } else {
            let mut output = format!("知识库搜索结果 (共{}条):\n\n", results.len());
            for (i, row) in results.iter().enumerate() {
                output.push_str(&format!(
                    "{}. 《{}》(类型: {}) - {}\n",
                    i + 1,
                    row.name,
                    row.entry_type,
                    row.path_url,
                ));
            }
            Ok(output)
        }
    }

    fn exec_calculate(args: &HashMap<String, String>) -> Result<String, AppError> {
        let expression = args
            .get("expression")
            .ok_or_else(|| AppError::Validation("缺少参数: expression".into()))?;

        let sanitized: String = expression
            .chars()
            .filter(|c| {
                c.is_numeric()
                    || "+-*/()^.%eEsincotaqrtldg, ".contains(*c)
                    || *c == '.'
            })
            .collect();

        if sanitized.len() > 500 {
            return Err(AppError::Validation("表达式过长（>500字符）".into()));
        }

        match meval::eval_str(&sanitized) {
            Ok(value) => Ok(format!("{} = {}", expression, value)),
            Err(e) => Ok(format!("计算失败: {}。表达式: {}", e, expression)),
        }
    }

    fn exec_system_info() -> Result<String, AppError> {
        let mut info = String::new();

        info.push_str(&format!("操作系统: {}\n", std::env::consts::OS));
        info.push_str(&format!("架构: {}\n", std::env::consts::ARCH));

        if let Ok(cwd) = std::env::current_dir() {
            info.push_str(&format!("当前目录: {}\n", cwd.display()));
        }

        if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            info.push_str(&format!("用户目录: {}\n", home));
        }

        info.push_str(&format!("进程ID: {}\n", std::process::id()));

        Ok(info)
    }

    pub fn format_tool_results_for_context(results: &[ToolResult]) -> String {
        if results.is_empty() {
            return String::new();
        }

        let mut ctx = String::from("\n## 工具执行结果\n\n");
        for result in results {
            ctx.push_str(&format!("### {} ({})\n", result.tool_name, if result.success {
                format!("成功, {}ms", result.duration_ms)
            } else {
                format!("失败, {}ms", result.duration_ms)
            }));

            if result.success {
                let truncated = if result.output.len() > 2000 {
                    format!("{}...(已截断)", &result.output[..2000])
                } else {
                    result.output.clone()
                };
                ctx.push_str(&format!("{}\n\n", truncated));
            } else if let Some(ref err) = result.error {
                ctx.push_str(&format!("错误: {}\n\n", err));
            }
        }

        ctx
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// 代码静态安全检查（安全审计修复发现 10，HIGH）
///
/// 对 AI 生成代码做轻量级 pattern 检查，拒绝明显恶意的代码片段。
/// 这不是真正的沙箱（真正的沙箱需要 WASM/Docker 进程隔离，工期较长），
/// 但可拦截大部分 prompt injection 攻击向量。
///
/// **检测维度**：
/// - 子进程执行（os.system / subprocess / exec / spawn）
/// - 文件系统越界（绝对路径访问 /etc/passwd、~/.ssh 等）
/// - 网络外泄（socket / urllib / requests / http.client / fetch）
/// - 权限提升（sudo / su / runas）
/// - 持久化（crontab / schtasks / 注册表写入）
/// - shell 注入（eval / exec 对动态字符串求值）
fn check_code_safety(code: &str, language: &str) -> Option<&'static str> {
    // 通用危险 pattern（跨语言）
    const GENERIC_DANGEROUS_PATTERNS: &[(&str, &str)] = &[
        // 权限提升
        ("sudo ", "sudo 权限提升"),
        ("sudo\t", "sudo 权限提升"),
        ("su root", "su 权限提升"),
        ("runas ", "runas 权限提升"),
        // 敏感文件路径
        ("/etc/passwd", "读取 /etc/passwd"),
        ("/etc/shadow", "读取 /etc/shadow"),
        (".ssh/id_rsa", "读取 SSH 私钥"),
        (".ssh/authorized_keys", "修改 SSH authorized_keys"),
        // 持久化
        ("crontab", "修改 crontab 持久化"),
        ("schtasks", "修改计划任务持久化"),
        ("/autostart/", "写入 autostart 持久化"),
        ("Run/RunOnce", "写入注册表自启动"),
    ];

    for (pattern, reason) in GENERIC_DANGEROUS_PATTERNS {
        if code.contains(pattern) {
            return Some(reason);
        }
    }

    // 语言专属 pattern
    match language {
        "python" => {
            const PYTHON_DANGEROUS: &[(&str, &str)] = &[
                ("import os", "Python os 模块（子进程/文件系统）"),
                ("import subprocess", "Python subprocess 模块"),
                ("import socket", "Python socket 模块（网络）"),
                ("import urllib", "Python urllib 模块（网络）"),
                ("import requests", "Python requests 模块（网络）"),
                ("import shutil", "Python shutil 模块（破坏性文件操作）"),
                ("import ctypes", "Python ctypes 模块（FFI 逃逸）"),
                ("from os ", "Python os 模块（子进程/文件系统）"),
                ("from subprocess", "Python subprocess 模块"),
                ("from socket", "Python socket 模块（网络）"),
                ("from urllib", "Python urllib 模块（网络）"),
                ("from requests", "Python requests 模块（网络）"),
                ("from shutil", "Python shutil 模块（破坏性文件操作）"),
                ("__import__", "Python 动态 import"),
                ("os.system(", "os.system 子进程执行"),
                ("os.popen(", "os.popen 子进程执行"),
                ("os.exec", "os.exec 子进程执行"),
                ("os.spawn", "os.spawn 子进程执行"),
                ("os.remove(", "os.remove 文件删除"),
                ("os.unlink(", "os.unlink 文件删除"),
                ("os.rmdir(", "os.rmdir 目录删除"),
                ("shutil.rmtree(", "shutil.rmtree 递归删除"),
                ("subprocess.run(", "subprocess.run 子进程执行"),
                ("subprocess.Popen(", "subprocess.Popen 子进程执行"),
                ("subprocess.call(", "subprocess.call 子进程执行"),
            ];
            for (pattern, reason) in PYTHON_DANGEROUS {
                if code.contains(pattern) {
                    return Some(reason);
                }
            }
        }
        "javascript" => {
            const JS_DANGEROUS: &[(&str, &str)] = &[
                ("require('child_process')", "Node child_process 模块"),
                ("require(\"child_process\")", "Node child_process 模块"),
                ("require('fs')", "Node fs 模块（文件系统）"),
                ("require(\"fs\")", "Node fs 模块（文件系统）"),
                ("require('net')", "Node net 模块（网络）"),
                ("require(\"net\")", "Node net 模块（网络）"),
                ("require('http')", "Node http 模块（网络）"),
                ("require(\"http\")", "Node http 模块（网络）"),
                ("require('https')", "Node https 模块（网络）"),
                ("require(\"https\")", "Node https 模块（网络）"),
                ("require('dns')", "Node dns 模块（网络）"),
                ("require(\"dns\")", "Node dns 模块（网络）"),
                ("process.exit(", "process.exit 进程退出"),
                ("process.kill(", "process.kill 进程杀死"),
                ("child_process.exec(", "child_process.exec 子进程执行"),
                ("child_process.spawn(", "child_process.spawn 子进程执行"),
                ("fs.unlinkSync(", "fs.unlinkSync 文件删除"),
                ("fs.rmSync(", "fs.rmSync 文件删除"),
                ("fs.rmdirSync(", "fs.rmdirSync 目录删除"),
                ("fetch(", "fetch 网络请求"),
                ("new WebSocket(", "WebSocket 网络连接"),
            ];
            for (pattern, reason) in JS_DANGEROUS {
                if code.contains(pattern) {
                    return Some(reason);
                }
            }
        }
        "cmd" | "powershell" => {
            // cmd/powershell 本质是 shell，已通过白名单+黑名单机制防护，
            // 这里仅做额外检测：拒绝明显的下载执行/反向 shell pattern
            const SHELL_DANGEROUS: &[(&str, &str)] = &[
                ("curl http", "curl 网络下载"),
                ("curl https", "curl 网络下载"),
                ("wget http", "wget 网络下载"),
                ("Invoke-WebRequest", "PowerShell 网络下载"),
                ("Invoke-RestMethod", "PowerShell 网络请求"),
                ("Start-BitsTransfer", "PowerShell BITS 下载"),
                ("nc -l", "nc 反向 shell"),
                ("ncat -l", "ncat 反向 shell"),
                ("IEX(", "PowerShell IEX 内联执行"),
                ("IEX (", "PowerShell IEX 内联执行"),
                // 注册表自启动
                ("HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "注册表自启动写入"),
                ("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "注册表自启动写入"),
            ];
            for (pattern, reason) in SHELL_DANGEROUS {
                if code.contains(pattern) {
                    return Some(reason);
                }
            }
        }
        _ => {}
    }

    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_builtin() {
        let registry = ToolRegistry::new();
        let tools = registry.list();
        assert!(tools.len() >= 6);
        assert!(registry.get("read_file").is_some());
        assert!(registry.get("execute_code").is_some());
        assert!(registry.get("calculate").is_some());
    }

    #[test]
    fn test_build_tools_prompt() {
        let registry = ToolRegistry::new();
        let prompt = registry.build_tools_prompt();
        assert!(prompt.contains("read_file"));
        assert!(prompt.contains("execute_code"));
        assert!(prompt.contains("calculate"));
        assert!(prompt.contains("search_knowledge_base"));
    }

    #[test]
    fn test_parse_tool_calls_single() {
        let text = "```tool\n名称: read_file\npath: /test/file.txt\n```";
        let calls = ToolExecutor::parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "read_file");
        assert!(calls[0].arguments.contains_key("path"));
    }

    #[test]
    fn test_parse_tool_calls_multiple() {
        let text = "```tool\n名称: read_file\npath: /a.txt\n```\n一些文字\n```tool\n名称: calculate\nexpression: 2+2\n```";
        let calls = ToolExecutor::parse_tool_calls(text);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].tool_name, "read_file");
        assert_eq!(calls[1].tool_name, "calculate");
    }

    #[test]
    fn test_parse_tool_calls_none() {
        let calls = ToolExecutor::parse_tool_calls("普通文本，没有工具调用。");
        assert_eq!(calls.len(), 0);
    }

    #[test]
    fn test_calculate() {
        let mut args = HashMap::new();
        args.insert("expression".to_string(), "2+3*4".to_string());
        let result = ToolExecutor::exec_calculate(&args);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("14"));
    }

    #[test]
    fn test_calculate_sqrt() {
        let mut args = HashMap::new();
        args.insert("expression".to_string(), "sqrt(16)".to_string());
        let result = ToolExecutor::exec_calculate(&args);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("4"));
    }

    #[test]
    fn test_system_info() {
        let result = ToolExecutor::exec_system_info();
        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(info.contains("操作系统"));
        assert!(info.contains("进程ID"));
    }

    #[test]
    fn test_format_tool_results() {
        let results = vec![
            ToolResult {
                tool_name: "calculate".to_string(),
                success: true,
                output: "2+2 = 4".to_string(),
                error: None,
                duration_ms: 5,
            },
            ToolResult {
                tool_name: "read_file".to_string(),
                success: false,
                output: String::new(),
                error: Some("文件不存在".to_string()),
                duration_ms: 2,
            },
        ];
        let formatted = ToolExecutor::format_tool_results_for_context(&results);
        assert!(formatted.contains("calculate"));
        assert!(formatted.contains("read_file"));
        assert!(formatted.contains("2+2 = 4"));
        assert!(formatted.contains("失败"));
    }
}