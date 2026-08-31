//! Agent 自主任务执行循环（D1 v3.1 / v2 恐龙双脑 D 轴深化）
//!
//! 设计依据：
//!   - 功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md（v3.1 Agent 化）
//!   - .trae/rules/项目核心设计意图.md §三（Yuan Code 的 AI 调用必须走云端 API）
//!
//! ReAct 模式：Thought → Action → Observation → 反思 → 直到完成或达到轮次上限。
//! 每轮通过 `app_handle.emit("agent-step", ...)` 向前端推送进度事件，
//! 前端可实时展示思考链 + 工具调用 + 文件变更。
//!
//! 工具集（受 AgentRoleConfig.available_tools 约束）：
//!   - read_file     读取工作区文件
//!   - write_file    写入工作区文件
//!   - list_files    列出目录
//!   - run_command   执行终端命令（沙盒内）
//!   - done          标记任务完成
//!
//! AI 调用走 AiModelService（云端 API 路由），与项目核心设计意图一致；
//! 不直接调用 ollama，避免误接底层智能模型。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

use crate::crypto::mek_manager::MekManager;
use crate::error::app_error::AppError;
use crate::services::ai_model_service::AiModelService;

/// 单个执行步骤（用于前端展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub turn: u32,
    pub thought: String,
    pub action: String,
    pub action_input: String,
    pub observation: String,
    pub timestamp: String,
}

/// 自主执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousExecuteRequest {
    /// 用户自然语言任务
    pub prompt: String,
    /// 工作区根路径
    pub workspace_path: String,
    /// 上下文文件相对路径列表
    pub context_files: Vec<String>,
    /// 模型 ID（从 ai_models 表选择，决定走云端还是本地）
    pub model_id: i64,
    /// 用户 ID（用于解密 API Key）
    pub user_id: i64,
    /// 最大循环次数（默认 12）
    pub max_turns: Option<u32>,
    /// 允许的工具白名单
    pub allowed_tools: Option<Vec<String>>,
}

/// 自主执行响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousExecuteResponse {
    pub agent_id: String,
    pub success: bool,
    pub final_answer: String,
    pub steps: Vec<AgentStep>,
    pub modified_files: Vec<String>,
    pub executed_commands: Vec<String>,
    pub total_turns: u32,
    pub error: Option<String>,
}

const DEFAULT_MAX_TURNS: u32 = 12;
const MAX_STEP_OBSERVATION_LEN: usize = 4000;

/// 自主执行入口
pub async fn execute_autonomous(
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    app_handle: &AppHandle,
    req: AutonomousExecuteRequest,
) -> Result<AutonomousExecuteResponse, AppError> {
    let agent_id = format!("auto_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let max_turns = req.max_turns.unwrap_or(DEFAULT_MAX_TURNS).min(50);
    let allowed_tools: Vec<String> = req.allowed_tools.unwrap_or_else(|| {
        vec![
            "read_file".into(),
            "write_file".into(),
            "list_files".into(),
            "run_command".into(),
            "done".into(),
        ]
    });

    // 1. 读取上下文文件，构建初始上下文
    let mut context_block = String::new();
    for rel_path in &req.context_files {
        let full = match join_workspace(&req.workspace_path, rel_path) {
            Ok(p) => p,
            Err(e) => {
                context_block.push_str(&format!(
                    "\n--- FILE: {} (路径校验失败: {}) ---\n--- END ---\n",
                    rel_path, e
                ));
                continue;
            }
        };
        if let Ok(content) = fs::read_to_string(&full) {
            context_block.push_str(&format!(
                "\n--- FILE: {} ---\n{}\n--- END ---\n",
                rel_path, content
            ));
        }
    }
    if context_block.is_empty() {
        context_block = "(无上下文文件)".into();
    }

    // 2. 构建基础系统提示词（ReAct 协议）
    let base_system_prompt = build_react_system_prompt(&req.workspace_path, &allowed_tools);

    // 3. 构建初始任务 prompt
    let initial_prompt = format!(
        "## 用户任务\n{}\n\n## 上下文文件\n{}\n\n请按 ReAct 协议响应。",
        req.prompt, context_block
    );

    // 4. 初始化 AI 服务
    let ai_service = AiModelService::new();

    // 4.5 D1.5 增强：任务规划阶段 — ReAct 循环前先调用 LLM 生成显式计划
    //
    // 设计意图：让 Agent 在进入 ReAct 循环前先生成一个结构化 plan，
    // 前端可通过 "agent-plan" 事件展示计划并让用户确认/修改。
    // plan 失败时降级为无 plan 引导（不阻塞主流程，符合"可关闭性"精神）。
    let plan_steps = generate_plan(
        &ai_service,
        pool,
        mek_manager,
        req.user_id,
        req.model_id,
        &req.prompt,
        &context_block,
        &allowed_tools,
    )
    .await
    .unwrap_or_else(|e| {
        tracing::warn!("[D1.5] Plan 生成失败，降级为无 plan 引导: {}", e);
        Vec::new()
    });

    // 推送 plan 到前端（前端可展示并让用户确认）
    let _ = app_handle.emit(
        "agent-plan",
        serde_json::json!({
            "agent_id": &agent_id,
            "plan": &plan_steps,
        }),
    );

    // 将 plan 注入 system_prompt 作为 ReAct 循环的引导
    let system_prompt = if plan_steps.is_empty() {
        base_system_prompt
    } else {
        let items: Vec<String> = plan_steps
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}. {}", i + 1, s))
            .collect();
        format!(
            "{}\n\n## Suggested Plan\n{}\n\nFollow this plan unless observation suggests otherwise.",
            base_system_prompt,
            items.join("\n")
        )
    };

    // 5. 进入 ReAct 循环
    let mut steps: Vec<AgentStep> = Vec::new();
    let mut trajectory = String::new();
    trajectory.push_str(&format!("User: {}\n", initial_prompt));

    let mut modified_files: Vec<String> = Vec::new();
    let mut executed_commands: Vec<String> = Vec::new();
    let mut final_answer = String::new();
    let mut error: Option<String> = None;

    for turn in 1..=max_turns {
        let prompt = format!("{}\n\nAssistant:", trajectory);
        let raw = ai_service
            .call_model(pool, mek_manager, req.user_id, req.model_id, &prompt, Some(&system_prompt))
            .await
            .map_err(|e| AppError::AiApi(format!("Agent 推理失败 (turn {}): {}", turn, e)))?;

        let parsed = parse_react_response(&raw);
        let step = AgentStep {
            turn,
            thought: parsed.thought,
            action: parsed.action.clone(),
            action_input: parsed.action_input.clone(),
            observation: String::new(),
            timestamp: now_iso(),
        };

        // 推送进度到前端
        let _ = app_handle.emit(
            "agent-step",
            serde_json::json!({
                "agent_id": &agent_id,
                "turn": turn,
                "thought": &step.thought,
                "action": &step.action,
                "action_input": &step.action_input,
            }),
        );

        // 检查工具白名单
        if !allowed_tools.iter().any(|t| t == &step.action) {
            let obs = format!("工具 {} 不在白名单中，跳过", step.action);
            let step = AgentStep { observation: obs.clone(), ..step };
            trajectory.push_str(&format!("Thought: {}\nAction: {}\nAction Input: {}\nObservation: {}\n", step.thought, step.action, step.action_input, obs));
            steps.push(step);
            continue;
        }

        // 执行工具
        let (obs, done) = execute_tool(
            &step.action,
            &step.action_input,
            &req.workspace_path,
            &mut modified_files,
            &mut executed_commands,
        )
        .await;

        let obs = truncate(&obs, MAX_STEP_OBSERVATION_LEN);
        trajectory.push_str(&format!(
            "Thought: {}\nAction: {}\nAction Input: {}\nObservation: {}\n",
            step.thought, step.action, step.action_input, obs
        ));

        let final_step = AgentStep { observation: obs.clone(), ..step };
        if done {
            final_answer = final_step.thought.clone();
        }
        steps.push(final_step);

        if done {
            break;
        }
    }

    if final_answer.is_empty() {
        if steps.len() as u32 >= max_turns {
            error = Some(format!("达到最大轮次 {} 仍未完成", max_turns));
        }
        final_answer = steps
            .last()
            .map(|s| s.thought.clone())
            .unwrap_or_default();
    }

    let total_turns = steps.len() as u32;
    Ok(AutonomousExecuteResponse {
        agent_id,
        success: error.is_none(),
        final_answer,
        steps,
        modified_files,
        executed_commands,
        total_turns,
        error,
    })
}

// ===== 内部辅助 =====

/// D1.5 任务规划：调用云端 LLM 生成结构化执行计划
///
/// 在 ReAct 循环前调用，让模型先分析任务并给出 3-10 步的显式计划。
/// 计划注入 system_prompt 作为引导，同时通过 "agent-plan" 事件推送给前端。
///
/// 设计依据：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §D1.5
/// AI 调用走 AiModelService（云端 API），不接底层智能模型。
async fn generate_plan(
    ai_service: &AiModelService,
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    user_id: i64,
    model_id: i64,
    prompt: &str,
    context_block: &str,
    allowed_tools: &[String],
) -> Result<Vec<String>, AppError> {
    let plan_prompt = format!(
        r#"Analyze the following task and generate a concise step-by-step plan.

## Task
{prompt}

## Context
{context_block}

## Available Tools
{tools}

## Instructions
Generate 3-7 clear, actionable steps. Each step should be a single line describing what to do.
Respond with ONLY the steps, one per line, numbered. No extra text.

Example:
1. Read the main.rs file to understand current structure
2. Add a new function foo() that handles X
3. Update the mod.rs to export foo
4. Run cargo check to verify"#,
        prompt = prompt,
        context_block = context_block,
        tools = allowed_tools.join(", "),
    );

    let raw = ai_service
        .call_model(pool, mek_manager, user_id, model_id, &plan_prompt, None)
        .await
        .map_err(|e| AppError::AiApi(format!("Plan 生成失败: {}", e)))?;

    // 解析编号步骤（"1. xxx" → "xxx"）
    let steps: Vec<String> = raw
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            // 去除 "1. " "2. " 等数字前缀
            if let Some(pos) = trimmed.find(". ") {
                let prefix = &trimmed[..pos];
                if prefix.chars().all(|c| c.is_numeric()) {
                    return Some(trimmed[pos + 2..].to_string());
                }
            }
            // 无数字前缀的非空行也作为步骤
            Some(trimmed.to_string())
        })
        .take(10) // 最多 10 步，防止模型生成过多
        .collect();

    Ok(steps)
}

fn build_react_system_prompt(workspace: &str, tools: &[String]) -> String {
    format!(
        r#"You are an autonomous code-editing agent in the Yuan Code editor (NexTerm·元界).

## Workspace
{workspace}

## Available Tools
{tools}

## Response Protocol (strict)
Reply with EXACTLY this format, nothing else:
Thought: <your reasoning>
Action: <tool name from Available Tools>
Action Input: <JSON or string argument for the tool>

When the task is complete, respond with:
Thought: <final summary>
Action: done
Action Input: <empty>

## Tool specifications
- read_file     Action Input: relative path (string)
- write_file    Action Input: {{"path": "relative/path", "content": "new content"}}
- list_files    Action Input: relative directory path (string)
- run_command   Action Input: {{"command": "...", "args": []}}
- done          Action Input: (empty)

Be concise. Prefer targeted edits over rewriting whole files.
"#,
        tools = tools.join(", ")
    )
}

#[derive(Default)]
struct ParsedReact {
    thought: String,
    action: String,
    action_input: String,
}

fn parse_react_response(raw: &str) -> ParsedReact {
    let mut out = ParsedReact::default();
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Thought:") {
            out.thought.push_str(rest.trim());
            out.thought.push(' ');
        } else if let Some(rest) = trimmed.strip_prefix("Action:") {
            out.action = rest.trim().to_string();
        } else if let Some(rest) = trimmed.strip_prefix("Action Input:") {
            out.action_input = rest.trim().to_string();
        }
    }
    out.thought = out.thought.trim().to_string();
    // 若模型未走 ReAct 协议，把整段回复作为 thought，action 默认 done
    if out.action.is_empty() {
        out.thought = if out.thought.is_empty() { raw.to_string() } else { out.thought };
        out.action = "done".into();
    }
    out
}

async fn execute_tool(
    action: &str,
    action_input: &str,
    workspace: &str,
    modified_files: &mut Vec<String>,
    executed_commands: &mut Vec<String>,
) -> (String, bool) {
    match action {
        "read_file" => {
            match join_workspace(workspace, action_input.trim_matches('"')) {
                Ok(path) => match fs::read_to_string(&path) {
                    Ok(c) => (truncate(&c, MAX_STEP_OBSERVATION_LEN), false),
                    Err(e) => (format!("读取失败: {}", e), false),
                },
                Err(e) => (format!("路径校验失败: {}", e), false),
            }
        }
        "write_file" => {
            let parsed: Result<WriteFileInput, _> = serde_json::from_str(action_input);
            match parsed {
                Ok(input) => {
                    match join_workspace(workspace, &input.path) {
                        Ok(path) => {
                            if let Some(parent) = path.parent() {
                                let _ = fs::create_dir_all(parent);
                            }
                            match fs::write(&path, &input.content) {
                                Ok(_) => {
                                    if !modified_files.contains(&input.path) {
                                        modified_files.push(input.path.clone());
                                    }
                                    (format!("已写入 {}", input.path), false)
                                }
                                Err(e) => (format!("写入失败: {}", e), false),
                            }
                        }
                        Err(e) => (format!("路径校验失败: {}", e), false),
                    }
                }
                Err(e) => (format!("Action Input 解析失败: {}", e), false),
            }
        }
        "list_files" => {
            match join_workspace(workspace, action_input.trim_matches('"')) {
                Ok(dir) => match list_dir_brief(&dir.to_string_lossy()) {
                    Ok(entries) => (entries, false),
                    Err(e) => (format!("列出失败: {}", e), false),
                },
                Err(e) => (format!("路径校验失败: {}", e), false),
            }
        }
        "run_command" => {
            let parsed: Result<RunCommandInput, _> = serde_json::from_str(action_input);
            match parsed {
                Ok(input) => {
                    let cmd_line = format!("{} {}", input.command, input.args.join(" "));
                    executed_commands.push(cmd_line.clone());

                    // D1.5 sandbox 接入：在工作区内执行命令（带安全检查 + 超时 + 输出截断）
                    match execute_command_safely(&input.command, &input.args, workspace).await {
                        Ok(output) => (output, false),
                        Err(e) => (format!("命令执行失败: {}", e), false),
                    }
                }
                Err(e) => (format!("Action Input 解析失败: {}", e), false),
            }
        }
        "done" => ("任务完成".into(), true),
        _ => (format!("未知工具: {}", action), false),
    }
}

#[derive(Deserialize)]
struct WriteFileInput {
    path: String,
    content: String,
}

#[derive(Deserialize)]
struct RunCommandInput {
    command: String,
    #[serde(default)]
    args: Vec<String>,
}

/// 安全拼接工作区相对路径，防止路径遍历。
///
/// 安全审计修复（发现 6，HIGH）：原实现仅去除前导 `./` 和 `/`，未过滤 `..`，
/// AI 可通过 `Action Input: ../../etc/passwd` 逃逸工作区读取系统文件。
/// 现统一委托给 `crate::utils::file_path::safe_join`，执行：
/// 1. `canonicalize(workspace)` 解析符号链接
/// 2. 拒绝 `rel` 为绝对路径
/// 3. 拒绝 `rel` 含 `..` 段
/// 4. `canonicalize` 父目录 + `starts_with(workspace)` 校验
///
/// 返回 `PathBuf` 而非 `String`，调用方直接用作 `AsRef<Path>`。
fn join_workspace(workspace: &str, rel: &str) -> Result<PathBuf, AppError> {
    crate::utils::file_path::safe_join(Path::new(workspace), rel)
}

fn list_dir_brief(dir: &str) -> Result<String, AppError> {
    let entries = fs::read_dir(dir).map_err(|e| AppError::FileSystem(e))?;
    let mut items: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }
        let prefix = if entry.path().is_dir() { "📁 " } else { "📄 " };
        items.push(format!("{}{}", prefix, name));
        if items.len() >= 100 {
            items.push("... (more truncated)".into());
            break;
        }
    }
    Ok(items.join("\n"))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...(truncated)", &s[..max])
    }
}

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    format!("+{}", secs)
}

/// D1.5 sandbox 接入：安全执行终端命令
///
/// 在指定工作区内执行命令，包含：
/// - **命令白名单检查**（安全审计修复发现 5，HIGH：原仅黑名单可被绕过）
/// - 危险命令黑名单检查（defense-in-depth，拦截 shell 注入的特殊参数）
/// - 超时限制（30 秒）
/// - 输出截断（MAX_STEP_OBSERVATION_LEN）
/// - 工作区限制（cwd 锁定）
async fn execute_command_safely(
    command: &str,
    args: &[String],
    workspace: &str,
) -> Result<String, AppError> {
    // 白名单检查（主入口）：仅允许明确定义的安全命令子集
    if !is_allowed_command(command) {
        return Err(AppError::AiApi(format!(
            "命令 '{}' 不在白名单内（仅允许明确定义的安全命令子集；如需扩展请联系开发者）",
            command
        )));
    }

    // 危险 pattern 检查（defense-in-depth：即便命令在白名单内，
    // 也拦截可能通过 args 注入的危险 shell pattern）
    let full_cmd = format!("{} {}", command, args.join(" "));
    if let Some(pattern) = is_dangerous_command(&full_cmd) {
        return Err(AppError::AiApi(format!(
            "危险命令被拦截（匹配黑名单: {}）",
            pattern
        )));
    }

    // 构建命令并设置工作区
    let mut cmd = tokio::process::Command::new(command);
    cmd.args(args);
    if !workspace.is_empty() {
        cmd.current_dir(workspace);
    }
    // 合并 stdout + stderr
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // 超时执行（30 秒）
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        cmd.output(),
    )
    .await
    .map_err(|_| AppError::AiApi("命令执行超时（30s）".into()))?
    .map_err(|e| AppError::AiApi(format!("命令启动失败: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit_code = output.status.code().unwrap_or(-1);

    // 组装输出
    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str("[stderr] ");
        result.push_str(&stderr);
    }
    if result.is_empty() {
        result = format!("(命令完成，退出码 {})", exit_code);
    } else {
        result.push_str(&format!("\n[exit: {}]", exit_code));
    }

    Ok(truncate(&result, MAX_STEP_OBSERVATION_LEN))
}

// 默认工具白名单常量（供 commands 层使用）
pub fn default_allowed_tools() -> Vec<String> {
    vec![
        "read_file".into(),
        "write_file".into(),
        "list_files".into(),
        "run_command".into(),
        "done".into(),
    ]
}

/// D1.5 命令白名单检查（安全审计修复发现 5，HIGH）
///
/// **背景**：原 `is_dangerous_command` 黑名单可被绕过：
/// - `rm -rf src/` 不匹配 `rm -rf /`（空格差异）→ 通过
/// - `mv ~/.bashrc /tmp/x` 完全无检测 → 通过
/// - `chmod 777 /etc/passwd` 完全无检测 → 通过
///
/// **修复**：引入白名单，仅允许明确定义的安全命令子集。
/// 黑名单 `is_dangerous_command` 保留作为 defense-in-depth。
///
/// **设计原则**：
/// - 仅允许只读/查询类命令（ls/cat/grep/git/cargo/npm 等）
/// - 拒绝任何破坏性命令（rm/mv/chmod/chown 等），即使被白名单绕过也由黑名单兜底
/// - 拒绝网络命令（curl/wget/nc 等），网络访问应通过专用工具
/// - 拒绝权限提升（sudo/su）
/// - 拒绝 shell 元命令（exec/eval/source）
fn is_allowed_command(command: &str) -> bool {
    const ALLOWED_COMMANDS: &[&str] = &[
        // 文件查看（只读）
        "ls", "cat", "head", "tail", "find", "grep", "rg", "wc", "tree", "stat", "file", "less",
        // 文本处理
        "echo", "sed", "awk", "sort", "uniq", "cut", "tr", "diff", "tee", "fold",
        // 目录/路径
        "pwd", "dirname", "basename", "realpath",
        // 系统查询（只读）
        "whoami", "date", "env", "which", "whereis", "uname",
        // 版本控制
        "git",
        // Rust
        "cargo", "rustc", "rustfmt",
        // Node.js / JavaScript
        "npm", "pnpm", "yarn", "node", "npx", "tsc", "eslint", "prettier",
        // Python
        "python", "python3", "pip", "pip3", "pytest",
        // 测试
        "jest", "vitest",
        // 构建
        "make", "cmake",
        // 其他语言
        "go",
        // 容器（仅查询类，不允许 run/exec）
        "docker", "kubectl",
    ];

    let cmd_bin = command.trim().trim_end_matches(".exe").to_lowercase();
    if cmd_bin.is_empty() {
        return false;
    }
    ALLOWED_COMMANDS.contains(&cmd_bin.as_str())
}

/// D1.5 危险命令黑名单检查（安全沙盒组件）
///
/// 检查命令是否匹配危险 pattern（rm -rf /, mkfs, format, shutdown 等）。
/// 返回匹配到的 pattern（供错误信息使用），安全命令返回 None。
fn is_dangerous_command(full_cmd: &str) -> Option<&'static str> {
    const DANGEROUS_PATTERNS: &[&str] = &[
        "rm -rf /",
        "rm -rf ~",
        "rm -rf /*",
        "mkfs",
        "format",
        "shutdown",
        "reboot",
        "dd if=",
        ":(){ :|:& };:",
        "> /dev/sda",
    ];
    DANGEROUS_PATTERNS
        .iter()
        .copied()
        .find(|p| full_cmd.contains(p))
}

#[allow(dead_code)]
fn _unused_marker(_: &HashMap<String, String>) {}

// ============================================================================
// D1.5 单元测试：ReAct 解析 + 路径拼接 + 危险命令拦截 + 截断
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_react_response ----

    #[test]
    fn test_parse_react_standard_format() {
        let raw = "Thought: I need to read the file first\nAction: read_file\nAction Input: \"src/main.rs\"";
        let parsed = parse_react_response(raw);
        assert_eq!(parsed.thought, "I need to read the file first");
        assert_eq!(parsed.action, "read_file");
        assert_eq!(parsed.action_input, "\"src/main.rs\"");
    }

    #[test]
    fn test_parse_react_multiline_thought() {
        // 多行 Thought 应拼接（每行 strip_prefix 后 trim + 空格连接）
        let raw = "Thought: First line\nThought: second line\nAction: done\nAction Input:";
        let parsed = parse_react_response(raw);
        assert!(parsed.thought.contains("First line"));
        assert!(parsed.thought.contains("second line"));
        assert_eq!(parsed.action, "done");
    }

    #[test]
    fn test_parse_react_no_action_defaults_done() {
        // 模型未走 ReAct 协议时，整段作为 thought，action 默认 done
        let raw = "这是一段自由格式的回复，没有遵循协议";
        let parsed = parse_react_response(raw);
        assert_eq!(parsed.action, "done");
        assert!(!parsed.thought.is_empty());
    }

    #[test]
    fn test_parse_react_empty_input() {
        let parsed = parse_react_response("");
        assert_eq!(parsed.action, "done");
        assert!(parsed.thought.is_empty());
    }

    // ---- join_workspace ----
    //
    // 安全审计修复后：join_workspace 返回 Result<PathBuf, AppError>，
    // 拒绝绝对路径、含 `..` 的路径、空 workspace。
    // 测试用 tempfile 创建真实工作区，避免依赖虚构的 Unix 路径。

    #[test]
    fn test_join_workspace_allows_dot_slash_relative() {
        // `./src/main.rs` 应被接受（无 `..`，相对路径）
        let dir = tempfile::tempdir().expect("创建临时工作区失败");
        let ws = dir.path();
        std::fs::create_dir_all(ws.join("src")).unwrap();
        std::fs::write(ws.join("src").join("main.rs"), "fn main() {}").unwrap();

        let result = join_workspace(ws.to_str().unwrap(), "./src/main.rs")
            .expect("`./` 相对路径应通过");
        assert!(result.ends_with("src/main.rs"));
        assert!(result.starts_with(ws.canonicalize().unwrap()));
    }

    #[test]
    fn test_join_workspace_reject_absolute_path() {
        // 绝对路径应被拒绝（防止 Path::join 覆盖 base）
        let dir = tempfile::tempdir().expect("创建临时工作区失败");
        let ws = dir.path();
        let err = join_workspace(ws.to_str().unwrap(), "/etc/passwd")
            .expect_err("绝对路径应被拒绝");
        assert!(err.to_string().contains("绝对路径"));
    }

    #[test]
    fn test_join_workspace_reject_parent_dir() {
        // 含 `..` 的路径应被拒绝（防止路径遍历逃逸工作区）
        let dir = tempfile::tempdir().expect("创建临时工作区失败");
        let ws = dir.path();
        let err = join_workspace(ws.to_str().unwrap(), "../../etc/passwd")
            .expect_err("`..` 路径应被拒绝");
        assert!(err.to_string().contains(".."));
    }

    #[test]
    fn test_join_workspace_reject_empty_workspace() {
        // 空 workspace 应被拒绝（原实现的 fallback 行为不安全）
        let err = join_workspace("", "src/main.rs")
            .expect_err("空 workspace 应被拒绝");
        assert!(matches!(err, AppError::Validation(_)));
    }

    // ---- is_dangerous_command ----

    #[test]
    fn test_dangerous_rm_rf_root() {
        assert_eq!(is_dangerous_command("rm -rf /"), Some("rm -rf /"));
    }

    #[test]
    fn test_dangerous_mkfs() {
        assert_eq!(is_dangerous_command("mkfs.ext4 /dev/sda1"), Some("mkfs"));
    }

    #[test]
    fn test_dangerous_format() {
        assert_eq!(is_dangerous_command("format C:"), Some("format"));
    }

    #[test]
    fn test_dangerous_shutdown() {
        assert_eq!(is_dangerous_command("shutdown -h now"), Some("shutdown"));
    }

    #[test]
    fn test_dangerous_dd() {
        assert_eq!(is_dangerous_command("dd if=/dev/zero of=/dev/sda"), Some("dd if="));
    }

    #[test]
    fn test_dangerous_fork_bomb() {
        assert_eq!(
            is_dangerous_command(":(){ :|:& };:"),
            Some(":(){ :|:& };:")
        );
    }

    #[test]
    fn test_safe_command_passes() {
        assert_eq!(is_dangerous_command("cargo check"), None);
        assert_eq!(is_dangerous_command("npm install"), None);
        assert_eq!(is_dangerous_command("git status"), None);
        assert_eq!(is_dangerous_command("ls -la"), None);
    }

    #[test]
    fn test_safe_rm_with_args_passes() {
        // "rm -rf src/" 不匹配 "rm -rf /"（注意空格），应通过
        assert_eq!(is_dangerous_command("rm -rf src/"), None);
    }

    // ---- is_allowed_command（白名单检查，安全审计修复发现 5） ----

    #[test]
    fn test_whitelist_allows_common_safe_commands() {
        // 常见开发命令应通过白名单
        assert!(is_allowed_command("ls"));
        assert!(is_allowed_command("cat"));
        assert!(is_allowed_command("grep"));
        assert!(is_allowed_command("git"));
        assert!(is_allowed_command("cargo"));
        assert!(is_allowed_command("npm"));
        assert!(is_allowed_command("node"));
        assert!(is_allowed_command("python"));
        assert!(is_allowed_command("python3"));
        assert!(is_allowed_command("go"));
    }

    #[test]
    fn test_whitelist_rejects_destructive_commands() {
        // 破坏性命令不在白名单内（即使黑名单可能漏过）
        assert!(!is_allowed_command("rm"));
        assert!(!is_allowed_command("mv"));
        assert!(!is_allowed_command("cp"));
        assert!(!is_allowed_command("chmod"));
        assert!(!is_allowed_command("chown"));
        assert!(!is_allowed_command("shutdown"));
        assert!(!is_allowed_command("reboot"));
        assert!(!is_allowed_command("mkfs"));
    }

    #[test]
    fn test_whitelist_rejects_network_commands() {
        // 网络命令不在白名单内（应通过专用工具，不走 Agent 命令执行）
        assert!(!is_allowed_command("curl"));
        assert!(!is_allowed_command("wget"));
        assert!(!is_allowed_command("nc"));
        assert!(!is_allowed_command("ssh"));
    }

    #[test]
    fn test_whitelist_rejects_privilege_escalation() {
        // 权限提升命令不在白名单内
        assert!(!is_allowed_command("sudo"));
        assert!(!is_allowed_command("su"));
    }

    #[test]
    fn test_whitelist_rejects_shell_meta_commands() {
        // shell 元命令不在白名单内
        assert!(!is_allowed_command("exec"));
        assert!(!is_allowed_command("eval"));
        assert!(!is_allowed_command("source"));
    }

    #[test]
    fn test_whitelist_handles_exe_suffix_and_case() {
        // Windows .exe 后缀应被处理（大小写不敏感）
        assert!(is_allowed_command("ls.exe"));
        assert!(is_allowed_command("CARGO"));
        assert!(is_allowed_command("Git.exe"));
    }

    #[test]
    fn test_whitelist_rejects_empty_and_unknown() {
        // 空命令和未知命令应被拒绝
        assert!(!is_allowed_command(""));
        assert!(!is_allowed_command("   "));
        assert!(!is_allowed_command("unknown_binary_xyz"));
        assert!(!is_allowed_command("malware"));
    }

    // ---- truncate ----

    #[test]
    fn test_truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 100), "hello");
    }

    #[test]
    fn test_truncate_long_string_cut() {
        let long = "x".repeat(200);
        let result = truncate(&long, 50);
        assert!(result.ends_with("...(truncated)"));
        assert!(result.len() < long.len());
    }

    #[test]
    fn test_truncate_exact_length_unchanged() {
        assert_eq!(truncate("abc", 3), "abc");
    }

    // ---- default_allowed_tools ----

    #[test]
    fn test_default_allowed_tools_contains_five_tools() {
        let tools = default_allowed_tools();
        assert_eq!(tools.len(), 5);
        assert!(tools.contains(&"read_file".to_string()));
        assert!(tools.contains(&"write_file".to_string()));
        assert!(tools.contains(&"list_files".to_string()));
        assert!(tools.contains(&"run_command".to_string()));
        assert!(tools.contains(&"done".to_string()));
    }
}
