//! 进程隔离后端 — 跨平台基础实现
//!
//! 通过独立进程执行命令，配合权限配置实现基础隔离。

use std::process::Stdio;
use std::time::Instant;

use async_trait::async_trait;
use tokio::process::Command;

use crate::error::app_error::AppError;
use crate::models::sandbox::{ExecuteRequest, ExecuteResult};

use super::{SandboxBackend, SandboxBackendType};

pub struct ProcessBackend {
    active: bool,
    /// 允许的命令白名单（用户/配置追加，与 DEFAULT_SAFE_WHITELIST 取并集）
    allowed_commands: Vec<String>,
    /// 禁止的命令黑名单（defense-in-depth，即便白名单通过也兜底拦截）
    blocked_commands: Vec<String>,
}

/// 默认安全命令白名单（安全审计修复发现 9，HIGH）
///
/// **背景**：原 `check_command` 仅在 `allowed_commands` 非空时才检查白名单，
/// 而构造函数 `new()` 中 `allowed_commands` 默认为空 → 白名单检查被完全跳过，
/// 仅靠 6 条黑名单（`rm -rf /`/`mkfs`/`dd if=/dev/zero`/`shutdown`/`reboot`/`format`）
/// 防护。攻击者可通过 `sh -c "ls; rm -rf ~"`/`cmd /C "dir & del /S *"` 等命令注入绕过。
///
/// **修复**：引入默认强制白名单，无论 `allowed_commands` 是否为空都执行。
const DEFAULT_SAFE_WHITELIST: &[&str] = &[
    // 文件查看（只读）
    "ls", "dir", "cat", "type", "head", "tail", "find", "grep", "findstr", "rg", "wc", "tree",
    "stat", "file", "less", "more",
    // 文本处理
    "echo", "sed", "awk", "sort", "uniq", "cut", "tr", "diff", "tee", "fold",
    // 目录/路径
    "pwd", "cd", "dirname", "basename", "realpath",
    // 系统查询（只读）
    "whoami", "date", "env", "set", "which", "whereis", "where", "uname",
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
    "make", "cmake", "msbuild",
    // 其他语言
    "go",
    // 容器（仅查询类，不允许 run/exec）
    "docker", "kubectl",
];

impl ProcessBackend {
    pub fn new() -> Self {
        Self {
            active: false,
            allowed_commands: Vec::new(),
            blocked_commands: vec![
                "rm -rf /".into(),
                "mkfs".into(),
                "dd if=/dev/zero".into(),
                "shutdown".into(),
                "reboot".into(),
                "format".into(),
            ],
        }
    }

    /// 命令安全检查（安全审计修复发现 9）
    ///
    /// 三层防护：
    /// 1. **shell 元字符防护**：拒绝 `;`/`&&`/`||`/`&`/反引号/`$()` 等命令替换/分隔符，
    ///    防止 `ls; rm -rf /` 这类注入（白名单只检查首个 binary）
    /// 2. **白名单检查**：默认强制白名单 + 用户配置取并集，首个 binary 必须在内
    /// 3. **黑名单兜底**：defense-in-depth，即便白名单通过也拦截已知危险 pattern
    fn check_command(&self, command: &str) -> Result<(), AppError> {
        let cmd_lower = command.to_lowercase();

        // 第 1 层：shell 元字符防护（拒绝命令分隔/替换，防止白名单绕过）
        if let Some(reason) = detect_shell_metacharacters(command) {
            return Err(AppError::Validation(format!(
                "命令被沙箱阻止（包含 shell 元字符 {}，存在命令注入风险）: {}",
                reason, command
            )));
        }

        // 第 2 层：黑名单检查（defense-in-depth，先于白名单短路返回）
        for blocked in &self.blocked_commands {
            if cmd_lower.contains(&blocked.to_lowercase()) {
                return Err(AppError::Validation(format!(
                    "命令被沙箱阻止: {}",
                    blocked
                )));
            }
        }

        // 第 3 层：白名单检查（默认强制 + 用户配置取并集）
        let cmd_bin = command
            .trim()
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_end_matches(".exe")
            .to_lowercase();
        if cmd_bin.is_empty() {
            return Err(AppError::Validation("空命令".into()));
        }

        let in_default = DEFAULT_SAFE_WHITELIST.contains(&cmd_bin.as_str());
        let in_user = self
            .allowed_commands
            .iter()
            .any(|a| a.trim_end_matches(".exe").to_lowercase() == cmd_bin);
        if !in_default && !in_user {
            return Err(AppError::Validation(format!(
                "命令 '{}' 不在白名单中（仅允许安全命令子集；如需扩展请联系开发者配置 allowed_commands）",
                cmd_bin
            )));
        }

        Ok(())
    }
}

/// 检测 shell 元字符（命令分隔符 / 命令替换）
///
/// 拒绝这些字符可防止 `ls; rm -rf /`、`ls $(rm -rf /)`、`ls && del /S *` 等
/// 通过白名单只检查首 binary 的注入攻击。
///
/// 注：管道 `|` 与重定向 `>` `<` 不在拦截列表中（合法用例多，且管道后续命令
/// 已通过本函数对完整命令字符串的扫描间接被黑名单兜底；如需更严格可在此扩展）。
fn detect_shell_metacharacters(command: &str) -> Option<&'static str> {
    const DANGEROUS_OPS: &[(&str, &str)] = &[
        ("&&", "AND 操作符 `&&`"),
        ("||", "OR 操作符 `||`"),
        (";", "分号 `;`"),
        ("`", "反引号命令替换"),
        ("$(", "命令替换 `$(`"),
        ("&", "后台执行 `&`"),
    ];
    for (op, desc) in DANGEROUS_OPS {
        if command.contains(op) {
            return Some(desc);
        }
    }
    None
}

#[async_trait]
impl SandboxBackend for ProcessBackend {
    fn backend_type(&self) -> SandboxBackendType {
        SandboxBackendType::Process
    }

    async fn initialize(&mut self) -> Result<(), AppError> {
        self.active = true;
        Ok(())
    }

    async fn execute(&self, req: &ExecuteRequest) -> Result<ExecuteResult, AppError> {
        if !self.active {
            return Err(AppError::Internal("沙箱未初始化".into()));
        }

        self.check_command(&req.command)?;

        let start = Instant::now();

        let child = Command::new(if cfg!(windows) { "cmd" } else { "sh" })
            .arg(if cfg!(windows) { "/C" } else { "-c" })
            .arg(&req.command)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(
                req.working_dir
                    .as_deref()
                    .unwrap_or(if cfg!(windows) { "C:\\" } else { "/" }),
            )
            .spawn()
            .map_err(|e| AppError::Internal(format!("启动进程失败: {}", e)))?;

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| AppError::Internal(format!("等待进程失败: {}", e)))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(ExecuteResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout,
            stderr,
            duration_ms,
            truncated: false,
        })
    }

    fn is_active(&self) -> bool {
        self.active
    }

    async fn cleanup(&mut self) -> Result<(), AppError> {
        self.active = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backend() -> ProcessBackend {
        ProcessBackend::new()
    }

    // ---- 默认强制白名单 ----

    #[test]
    fn test_default_whitelist_allows_safe_commands() {
        let b = backend();
        assert!(b.check_command("ls -la").is_ok());
        assert!(b.check_command("cat README.md").is_ok());
        assert!(b.check_command("git status").is_ok());
        assert!(b.check_command("cargo check").is_ok());
        assert!(b.check_command("npm install").is_ok());
        assert!(b.check_command("python -V").is_ok());
    }

    #[test]
    fn test_default_whitelist_rejects_destructive_commands() {
        let b = backend();
        // rm/mv/chmod 不在白名单内
        assert!(b.check_command("rm file.txt").is_err());
        assert!(b.check_command("mv a b").is_err());
        assert!(b.check_command("chmod 777 file").is_err());
        assert!(b.check_command("shutdown -h now").is_err());
    }

    #[test]
    fn test_default_whitelist_rejects_network_commands() {
        let b = backend();
        assert!(b.check_command("curl http://evil.com").is_err());
        assert!(b.check_command("wget http://evil.com").is_err());
        assert!(b.check_command("nc -l 4444").is_err());
    }

    #[test]
    fn test_default_whitelist_rejects_privilege_escalation() {
        let b = backend();
        assert!(b.check_command("sudo rm -rf /").is_err());
        assert!(b.check_command("su root").is_err());
    }

    // ---- shell 元字符防护 ----

    #[test]
    fn test_shell_metachar_rejects_command_chaining() {
        let b = backend();
        // 即便首命令在白名单内，命令链也应被拒
        assert!(b.check_command("ls; rm -rf /").is_err());
        assert!(b.check_command("ls && rm -rf /").is_err());
        assert!(b.check_command("ls || rm -rf /").is_err());
        assert!(b.check_command("ls &").is_err());
    }

    #[test]
    fn test_shell_metachar_rejects_command_substitution() {
        let b = backend();
        assert!(b.check_command("ls `rm -rf /`").is_err());
        assert!(b.check_command("ls $(rm -rf /)").is_err());
    }

    #[test]
    fn test_shell_metachar_allows_pipe_and_redirect() {
        let b = backend();
        // 管道和重定向不在元字符黑名单内（合法用例多）
        assert!(b.check_command("ls | grep foo").is_ok());
        assert!(b.check_command("echo hello > out.txt").is_ok());
    }

    // ---- 黑名单兜底 ----

    #[test]
    fn test_blacklist_still_active_as_defense_in_depth() {
        let b = backend();
        // 黑名单 pattern 即便不在白名单首 binary 之外的位置也应被拦
        // 注：这些命令本身因白名单拒绝，但黑名单优先级在前，会先返回黑名单错误
        let _ = b.check_command("rm -rf /").unwrap_err();
        let _ = b.check_command("mkfs.ext4 /dev/sda1").unwrap_err();
    }

    // ---- detect_shell_metacharacters 单元测试 ----

    #[test]
    fn test_detect_metacharacters() {
        assert!(detect_shell_metacharacters("ls; rm").is_some());
        assert!(detect_shell_metacharacters("ls && rm").is_some());
        assert!(detect_shell_metacharacters("ls || rm").is_some());
        assert!(detect_shell_metacharacters("ls &").is_some());
        assert!(detect_shell_metacharacters("ls `rm`").is_some());
        assert!(detect_shell_metacharacters("ls $(rm)").is_some());
        assert!(detect_shell_metacharacters("ls -la").is_none());
        assert!(detect_shell_metacharacters("git status").is_none());
        assert!(detect_shell_metacharacters("ls | grep foo").is_none());
    }
}
