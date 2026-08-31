use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::Serialize;
use tauri::Emitter;

use crate::error::app_error::AppError;
use crate::models::terminal::{BuiltinCommandResult, DirectoryEntry, DirectoryListing, SystemInfo, WslDistribution, WslStatus};

#[derive(Debug, Serialize, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub session_type: String,
    pub cols: u16,
    pub rows: u16,
}

pub struct PtySession {
    pub master: Box<dyn portable_pty::MasterPty + Send>,
    pub writer: Mutex<Option<Box<dyn Write + Send>>>,
    pub child: Box<dyn portable_pty::Child + Send + Sync + 'static>,
    pub session_type: String,
    pub cols: u16,
    pub rows: u16,
}

pub struct TerminalService {
    pub sessions: Mutex<HashMap<String, PtySession>>,
    pub working_dir: Mutex<PathBuf>,
}

impl TerminalService {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            working_dir: Mutex::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))),
        }
    }

    pub fn create_session(
        &self,
        session_type: &str,
        app_handle: tauri::AppHandle,
    ) -> Result<String, AppError> {
        self.create_session_with_opts(session_type, None, None, app_handle)
    }

    /// 创建 PTY 会话（增强版：支持 WSL 发行版和自定义 Shell）
    pub fn create_session_with_opts(
        &self,
        session_type: &str,
        wsl_distro: Option<&str>,
        wsl_shell: Option<&str>,
        app_handle: tauri::AppHandle,
    ) -> Result<String, AppError> {
        let pty_system = native_pty_system();

        let cols: u16 = 80;
        let rows: u16 = 24;

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| AppError::TerminalError(format!("创建 PTY 失败: {}", e)))?;

        let cmd = match session_type {
            "cmd" => CommandBuilder::new("cmd.exe"),
            "powershell" => CommandBuilder::new("powershell.exe"),
            "wsl" => {
                // 参考 Windows Terminal / Codex CLI 的 WSL 启动方式
                let mut c = if let Some(distro) = wsl_distro {
                    // 精确指定发行版：wsl.exe -d Ubuntu -- bash -l
                    let mut base = CommandBuilder::new("wsl.exe");
                    base.arg("-d");
                    base.arg(distro);
                    base.arg("--");
                    base
                } else {
                    CommandBuilder::new("wsl.exe")
                };

                // 设置 Shell（默认 bash -l 加载完整环境）
                let shell = wsl_shell.unwrap_or("bash");
                if session_type == "wsl" {
                    c.arg(shell);
                    if shell == "bash" || shell == "zsh" || shell == "sh" {
                        c.arg("-l"); // login shell，加载 .profile/.zshrc
                    }
                }

                // 配置终端环境变量（参考 Warp Shell Integration）
                c.env("TERM", "xterm-256color");
                c.env("COLORTERM", "truecolor");
                c.env("TERM_PROGRAM", "nexterm");
                c.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));

                c
            }
            _ => CommandBuilder::new("cmd.exe"),
        };

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| AppError::TerminalError(format!("启动 Shell 失败: {}", e)))?;

        let session_id = uuid::Uuid::new_v4().to_string();
        let master = pair.master;

        let writer = master
            .take_writer()
            .map_err(|e| AppError::TerminalError(format!("获取 PTY 写入器失败: {}", e)))?;

        let sid = session_id.clone();
        let mut reader = master
            .try_clone_reader()
            .map_err(|e| AppError::TerminalError(format!("克隆 PTY 读取器失败: {}", e)))?;

        tokio::task::spawn_blocking(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buf[..n]).to_string();
                        let _ = app_handle.emit(
                            "terminal-output",
                            serde_json::json!({
                                "session_id": sid,
                                "data": output,
                            }),
                        );
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!(session_id = %sid, error = %e, "PTY 读取错误，线程退出");
                        let _ = app_handle.emit(
                            "terminal-output",
                            serde_json::json!({
                                "session_id": sid,
                                "data": format!("\r\n[PTY 会话已断开: {}]\r\n", e),
                            }),
                        );
                        break;
                    }
                }
            }
            tracing::info!(session_id = %sid, "PTY 读取线程结束");
        });

        let mut sessions = self.sessions.lock().expect("sessions lock failed");
        sessions.insert(
            session_id.clone(),
            PtySession {
                master,
                writer: Mutex::new(Some(writer)),
                child,
                session_type: session_type.to_string(),
                cols,
                rows,
            },
        );

        tracing::info!(session_id = %session_id, session_type, "PTY 会话已创建");
        Ok(session_id)
    }

    pub fn write_input(&self, session_id: &str, input: &str) -> Result<(), AppError> {
        let sessions = self.sessions.lock().expect("sessions lock failed");
        let session = sessions
            .get(session_id)
            .ok_or_else(|| AppError::TerminalError("PTY 会话不存在".into()))?;

        let mut writer_guard = session
            .writer
            .lock()
            .map_err(|e| AppError::TerminalError(format!("获取写入锁失败: {}", e)))?;

        if let Some(ref mut writer) = *writer_guard {
            writer
                .write_all(input.as_bytes())
                .map_err(|e| AppError::TerminalError(format!("写入 PTY 失败: {}", e)))?;
            writer
                .flush()
                .map_err(|e| AppError::TerminalError(format!("刷新 PTY 缓冲区失败: {}", e)))?;
        } else {
            return Err(AppError::TerminalError("PTY 写入器已被消费".into()));
        }

        Ok(())
    }

    pub fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<(), AppError> {
        let mut sessions = self.sessions.lock().expect("sessions lock failed");
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::TerminalError("PTY 会话不存在".into()))?;

        session
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| AppError::TerminalError(format!("调整 PTY 尺寸失败: {}", e)))?;

        session.cols = cols;
        session.rows = rows;
        tracing::info!(session_id, cols, rows, "PTY 尺寸已调整");
        Ok(())
    }

    pub fn kill_session(&self, session_id: &str) -> Result<i32, AppError> {
        let mut sessions = self.sessions.lock().expect("sessions lock failed");
        if let Some(mut session) = sessions.remove(session_id) {
            let exit_code = session.child.wait().ok().map(|s| s.exit_code() as i32).unwrap_or(0);
            let _ = session.child.kill();
            tracing::info!(session_id, exit_code, "PTY 会话已终止");
            return Ok(exit_code);
        }
        Ok(0)
    }

    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.lock().expect("sessions lock failed");
        sessions
            .iter()
            .map(|(id, s)| SessionInfo {
                session_id: id.clone(),
                session_type: s.session_type.clone(),
                cols: s.cols,
                rows: s.rows,
            })
            .collect()
    }

    pub fn kill_all(&self) {
        let mut sessions = self.sessions.lock().expect("sessions lock failed");
        for (sid, mut session) in sessions.drain() {
            let exit_code = session.child.wait().ok().map(|s| s.exit_code() as i32).unwrap_or(0);
            let _ = session.child.kill();
            tracing::info!(session_id = %sid, exit_code, "PTY 会话已清理");
        }
    }

    fn resolve_alias<'a>(&self, cmd_name: &'a str) -> &'a str {
        match cmd_name {
            "ll" => "ls",
            "la" => "ls",
            "cls" => "clear",
            "dir" => "ls",
            "type" => "cat",
            "copy" => "cp",
            "move" => "mv",
            "del" => "rm",
            "erase" => "rm",
            "ren" => "mv",
            "md" => "mkdir",
            "rd" => "rm",
            "minimize" => "hide",
            "top" => "alwaysontop",
            _ => cmd_name,
        }
    }

    pub fn execute_builtin(&self, command: &str) -> BuiltinCommandResult {
        let cmd = command.trim().to_lowercase();
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let cmd_name_raw = parts.first().map(|s| *s).unwrap_or("");
        let cmd_name = self.resolve_alias(cmd_name_raw);

        match cmd_name {
            "help" => self.builtin_help(),
            "ls" => self.builtin_ls(parts.get(1).copied()),
            "pwd" => self.builtin_pwd(),
            "cd" => self.builtin_cd(parts.get(1).copied()),
            "mkdir" => self.builtin_mkdir(parts.get(1).copied()),
            "cat" => self.builtin_cat(parts.get(1).copied()),
            "echo" => self.builtin_echo(&parts[1..]),
            "date" => self.builtin_date(),
            "whoami" => self.builtin_whoami(),
            "clear" => BuiltinCommandResult {
                output: String::new(),
                exit_code: 0,
            },
            "version" => self.builtin_version(),
            "env" => self.builtin_env(),
            "sysinfo" => self.builtin_sysinfo(),
            "history" => self.builtin_history(),
            "tree" => self.builtin_tree(parts.get(1).copied()),
            "grep" => self.builtin_grep(parts.get(1).copied(), parts.get(2).copied()),
            "find" => self.builtin_find(parts.get(1).copied()),
            "wc" => self.builtin_wc(parts.get(1).copied()),
            "head" => self.builtin_head(parts.get(1).copied(), parts.get(2).copied()),
            "tail" => self.builtin_tail(parts.get(1).copied(), parts.get(2).copied()),
            "cp" => self.builtin_cp(parts.get(1).copied(), parts.get(2).copied()),
            "mv" => self.builtin_mv(parts.get(1).copied(), parts.get(2).copied()),
            "rm" => self.builtin_rm(parts.get(1).copied()),
            "touch" => self.builtin_touch(parts.get(1).copied()),
            "clearscrollback" => self.builtin_clearscrollback(),
            "reset" => self.builtin_reset(),
            "fontsize" => self.builtin_fontsize(parts.get(1).copied()),
            "fullscreen" => self.builtin_fullscreen(),
            "reload" => self.builtin_reload(),
            "scroll" => self.builtin_scroll(parts.get(1).copied(), parts.get(2).copied()),
            "hide" => self.builtin_hide(),
            "quit" => self.builtin_quit(),
            "alwaysontop" => self.builtin_alwaysontop(),
            "cmd" => BuiltinCommandResult {
                output: ">>> 正在切换到 Windows CMD 模式 <<<\n>>> 后端将启动 ConPTY 进程并实时同步输出 <<<\n>>> 输入 exit 返回内置终端 <<<".into(),
                exit_code: 0,
            },
            "powershell" => BuiltinCommandResult {
                output: ">>> 正在切换到 PowerShell 模式 <<<\n>>> 后端将启动 PowerShell ConPTY 进程 <<<\n>>> 输入 exit 返回内置终端 <<<".into(),
                exit_code: 0,
            },
            "exit" => BuiltinCommandResult {
                output: ">>> 已返回应用内置终端 <<<".into(),
                exit_code: 0,
            },
            _ => {
                let suggestions: Vec<&str> = [
                    "help", "ls", "pwd", "cd", "mkdir", "cat", "echo",
                    "date", "whoami", "clear", "version", "env", "sysinfo",
                    "history", "tree", "grep", "find", "wc", "head", "tail",
                    "cp", "mv", "rm", "touch", "clearscrollback", "reset",
                    "fontsize", "fullscreen", "reload", "scroll", "search",
                    "hide", "quit", "alwaysontop", "cmd", "powershell",
                ]
                .iter()
                .filter(|s| s.starts_with(cmd_name_raw))
                .copied()
                .collect();

                if !suggestions.is_empty() && !cmd.is_empty() {
                    BuiltinCommandResult {
                        output: format!(
                            "命令未找到: {}\n您是否要找: {} ?\n输入 \"help\" 查看可用命令",
                            command,
                            suggestions.join(", ")
                        ),
                        exit_code: 127,
                    }
                } else {
                    BuiltinCommandResult {
                        output: format!("命令未找到: {}\n输入 \"help\" 查看可用命令", command),
                        exit_code: 127,
                    }
                }
            }
        }
    }

    fn builtin_help(&self) -> BuiltinCommandResult {
        BuiltinCommandResult {
            output: "\
═══ 文件与目录 ═══
  ls [路径]     列出目录内容（目录以 / 结尾）
  pwd           显示当前工作目录
  cd  <目录>    切换工作目录
  mkdir <目录>  创建新目录
  cat <文件>    查看文件内容
  tree [路径]   以树形结构显示目录
  cp  <源> <目标>  复制文件
  mv  <源> <目标>  移动/重命名文件
  rm  <文件>    删除文件或目录
  touch <文件>  创建空文件或更新时间戳
  grep <模式> <文件>  在文件中搜索文本
  find <模式>   递归搜索文件名
  wc  <文件>    统计文件行数/单词数/字符数
  head <文件> [行数]  查看文件头部
  tail <文件> [行数]  查看文件尾部

═══ 系统信息 ═══
  date          显示当前日期时间
  whoami        显示当前用户@主机名
  env           显示环境变量
  sysinfo       显示完整系统信息
  version       显示终端版本信息

═══ 终端控制 ═══
  echo <文本>   输出文本到终端
  clear         清空终端 (Ctrl+L)
  clearscrollback  仅清除回滚缓冲区 (Ctrl+Shift+K)
  history       显示命令历史记录
  help          显示此帮助信息
  reset         重置终端状态（恢复工作目录）
  fontsize +/-/0  调整字体大小 (+放大/-缩小/0重置)
  fullscreen    切换全屏模式 (Alt+Enter)
  reload        重载终端配置
  scroll top/bottom/up/down  终端滚动控制
  search <关键词>  搜索命令历史记录
  hide          最小化窗口
  quit          退出应用
  alwaysontop   切换窗口置顶

═══ 系统终端 ═══
  cmd           切换到 Windows CMD 模式
  powershell    切换到 PowerShell 模式
  exit          退出系统终端模式

═══ 命令别名 ═══
  ll/la → ls    cls → clear    dir → ls
  type → cat    copy → cp      move/ren → mv
  del/erase/rd → rm    md → mkdir
  minimize → hide    top → alwaysontop

快捷键: ↑↓ 历史 | Tab 补全 | Ctrl+L 清屏 | Ctrl+C 中断 | Ctrl+A 全选 | Ctrl+U 清行"
                .into(),
            exit_code: 0,
        }
    }

    fn builtin_ls(&self, path: Option<&str>) -> BuiltinCommandResult {
        let dir = match path {
            Some(p) => {
                let wd = self.working_dir.lock().expect("working_dir lock failed");
                let target = wd.join(p);
                target
            }
            None => self.working_dir.lock().expect("working_dir lock failed").clone(),
        };
        match std::fs::read_dir(&dir) {
            Ok(entries) => {
                let mut output = String::new();
                let mut items: Vec<String> = Vec::new();
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    if is_dir {
                        items.push(format!("{:<30} <DIR>", format!("{}/", name)));
                    } else {
                        items.push(format!("{:<30} {:>8} B", name, size));
                    }
                }
                items.sort_by(|a, b| {
                    let a_dir = a.contains("<DIR>");
                    let b_dir = b.contains("<DIR>");
                    if a_dir && !b_dir {
                        std::cmp::Ordering::Less
                    } else if !a_dir && b_dir {
                        std::cmp::Ordering::Greater
                    } else {
                        a.cmp(b)
                    }
                });
                for item in &items {
                    output.push_str(item);
                    output.push('\n');
                }
                if output.is_empty() {
                    output = "(空目录)".into();
                }
                BuiltinCommandResult {
                    output,
                    exit_code: 0,
                }
            }
            Err(e) => BuiltinCommandResult {
                output: format!("无法列出目录: {}", e),
                exit_code: 1,
            },
        }
    }

    fn builtin_pwd(&self) -> BuiltinCommandResult {
        let wd = self.working_dir.lock().expect("working_dir lock failed");
        BuiltinCommandResult {
            output: wd.to_string_lossy().to_string(),
            exit_code: 0,
        }
    }

    fn builtin_date(&self) -> BuiltinCommandResult {
        let now = chrono::Local::now();
        BuiltinCommandResult {
            output: now.format("%Y-%m-%d %H:%M:%S").to_string(),
            exit_code: 0,
        }
    }

    fn builtin_whoami(&self) -> BuiltinCommandResult {
        let username = whoami::username();
        let hostname = whoami::fallible::hostname().unwrap_or_else(|_| "unknown".into());
        BuiltinCommandResult {
            output: format!("{}@{}", username, hostname),
            exit_code: 0,
        }
    }

    fn builtin_cd(&self, path: Option<&str>) -> BuiltinCommandResult {
        let target = match path {
            Some(p) => {
                let wd = self.working_dir.lock().expect("working_dir lock failed");
                let new_path = wd.join(p);
                match new_path.canonicalize() {
                    Ok(canonical) => canonical,
                    Err(e) => {
                        return BuiltinCommandResult {
                            output: format!("无法切换到目录: {}", e),
                            exit_code: 1,
                        }
                    }
                }
            }
            None => {
                match dirs_next::home_dir() {
                    Some(home) => home,
                    None => {
                        return BuiltinCommandResult {
                            output: "无法获取用户主目录".into(),
                            exit_code: 1,
                        }
                    }
                }
            }
        };

        if !target.is_dir() {
            return BuiltinCommandResult {
                output: format!("不是一个目录: {}", target.display()),
                exit_code: 1,
            };
        }

        let mut wd = self.working_dir.lock().expect("working_dir lock failed");
        *wd = target;
        BuiltinCommandResult {
            output: String::new(),
            exit_code: 0,
        }
    }

    fn builtin_mkdir(&self, name: Option<&str>) -> BuiltinCommandResult {
        let dir_name = match name {
            Some(n) => n,
            None => {
                return BuiltinCommandResult {
                    output: "用法: mkdir <目录名>".into(),
                    exit_code: 1,
                }
            }
        };

        let wd = self.working_dir.lock().expect("working_dir lock failed");
        let target = wd.join(dir_name);

        match std::fs::create_dir(&target) {
            Ok(_) => BuiltinCommandResult {
                output: format!("已创建目录: {}", dir_name),
                exit_code: 0,
            },
            Err(e) => BuiltinCommandResult {
                output: format!("创建目录失败: {}", e),
                exit_code: 1,
            },
        }
    }

    fn builtin_cat(&self, file: Option<&str>) -> BuiltinCommandResult {
        let file_name = match file {
            Some(f) => f,
            None => {
                return BuiltinCommandResult {
                    output: "用法: cat <文件名>".into(),
                    exit_code: 1,
                }
            }
        };

        let wd = self.working_dir.lock().expect("working_dir lock failed");
        let target = wd.join(file_name);

        match std::fs::read_to_string(&target) {
            Ok(content) => {
                let preview = if content.lines().count() > 50 {
                    let truncated: String = content.lines().take(50).collect::<Vec<&str>>().join("\n");
                    format!("{}\n\n... (共 {} 行，仅显示前 50 行)", truncated, content.lines().count())
                } else {
                    content
                };
                BuiltinCommandResult {
                    output: preview,
                    exit_code: 0,
                }
            }
            Err(e) => BuiltinCommandResult {
                output: format!("读取文件失败: {}", e),
                exit_code: 1,
            },
        }
    }

    fn builtin_echo(&self, args: &[&str]) -> BuiltinCommandResult {
        let output = args.join(" ");
        BuiltinCommandResult {
            output,
            exit_code: 0,
        }
    }

    fn builtin_env(&self) -> BuiltinCommandResult {
        let mut output = String::new();
        let mut vars: Vec<(String, String)> = std::env::vars().collect();
        vars.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        for (key, value) in &vars {
            output.push_str(&format!("{}={}\n", key, value));
        }
        BuiltinCommandResult {
            output,
            exit_code: 0,
        }
    }

    fn builtin_version(&self) -> BuiltinCommandResult {
        BuiltinCommandResult {
            output: "\
NexTerm · 元界 终端模块
版本：V1.09
更新日期：2026-05-17
状态：38个内置命令 + 12个命令别名 + 系统终端调用 + 命令手册 + 历史记录持久化 + 前后端联调 + 终端控制命令(clearscrollback/reset/fontsize/fullscreen/reload/scroll/search/hide/quit/alwaysontop)"
                .into(),
            exit_code: 0,
        }
    }

    fn builtin_sysinfo(&self) -> BuiltinCommandResult {
        let info = self.get_system_info();
        let wd = self.working_dir.lock().expect("working_dir lock failed");
        let total_mem = sysinfo::System::new_all().total_memory();
        let used_mem = sysinfo::System::new_all().used_memory();
        let cpu_count = num_cpus::get();

        BuiltinCommandResult {
            output: format!(
                "\
═══ 系统信息 ═══
操作系统: {}
主机名:   {}
用户名:   {}
当前目录: {}
CPU 核心: {}
总内存:   {} MB
已用内存: {} MB
可用内存: {} MB",
                info.os,
                info.hostname,
                info.username,
                wd.display(),
                cpu_count,
                total_mem / 1024 / 1024,
                used_mem / 1024 / 1024,
                (total_mem - used_mem) / 1024 / 1024,
            ),
            exit_code: 0,
        }
    }

    fn builtin_history(&self) -> BuiltinCommandResult {
        BuiltinCommandResult {
            output: "命令历史记录请通过前端界面查看（↑↓ 键浏览）\n历史记录已持久化到 SQLite terminal_history 表".into(),
            exit_code: 0,
        }
    }

    fn builtin_tree(&self, path: Option<&str>) -> BuiltinCommandResult {
        let dir = match path {
            Some(p) => {
                let wd = self.working_dir.lock().expect("working_dir lock failed");
                wd.join(p)
            }
            None => self.working_dir.lock().expect("working_dir lock failed").clone(),
        };

        let mut output = format!("{}\n", dir.display());
        self.tree_recurse(&dir, "", &mut output, 3);

        BuiltinCommandResult {
            output,
            exit_code: 0,
        }
    }

    fn tree_recurse(&self, dir: &PathBuf, prefix: &str, output: &mut String, max_depth: usize) {
        if max_depth == 0 {
            output.push_str(&format!("{}...\n", prefix));
            return;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        let mut items: Vec<_> = entries.flatten().collect();
        items.sort_by(|a, b| {
            let a_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let b_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
            match (a_dir, b_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });

        let len = items.len();
        for (i, entry) in items.iter().enumerate() {
            let is_last = i == len - 1;
            let connector = if is_last { "└── " } else { "├── " };
            let child_prefix = if is_last { "    " } else { "│   " };

            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

            if is_dir {
                output.push_str(&format!("{}{}{}/\n", prefix, connector, name));
                let child_dir = entry.path();
                self.tree_recurse(
                    &child_dir,
                    &format!("{}{}", prefix, child_prefix),
                    output,
                    max_depth - 1,
                );
            } else {
                output.push_str(&format!("{}{}{}\n", prefix, connector, name));
            }
        }
    }

    fn builtin_grep(&self, pattern: Option<&str>, file: Option<&str>) -> BuiltinCommandResult {
        let pattern = match pattern {
            Some(p) => p,
            None => return BuiltinCommandResult {
                output: "用法: grep <模式> <文件>".into(),
                exit_code: 1,
            },
        };
        let file = match file {
            Some(f) => f,
            None => return BuiltinCommandResult {
                output: "用法: grep <模式> <文件>".into(),
                exit_code: 1,
            },
        };

        let wd = self.working_dir.lock().expect("working_dir lock failed");
        let target = wd.join(file);

        match std::fs::read_to_string(&target) {
            Ok(content) => {
                let matches: Vec<String> = content
                    .lines()
                    .enumerate()
                    .filter(|(_, line)| line.contains(pattern))
                    .map(|(i, line)| format!("{}: {}", i + 1, line))
                    .collect();

                if matches.is_empty() {
                    BuiltinCommandResult {
                        output: format!("未找到匹配: {}", pattern),
                        exit_code: 1,
                    }
                } else {
                    BuiltinCommandResult {
                        output: matches.join("\n"),
                        exit_code: 0,
                    }
                }
            }
            Err(e) => BuiltinCommandResult {
                output: format!("读取文件失败: {}", e),
                exit_code: 1,
            },
        }
    }

    fn builtin_find(&self, pattern: Option<&str>) -> BuiltinCommandResult {
        let pattern = match pattern {
            Some(p) => p,
            None => return BuiltinCommandResult {
                output: "用法: find <文件名模式>".into(),
                exit_code: 1,
            },
        };

        let wd = self.working_dir.lock().expect("working_dir lock failed");
        let mut results: Vec<String> = Vec::new();
        self.find_recurse(&wd, pattern, &mut results, 5);

        if results.is_empty() {
            BuiltinCommandResult {
                output: format!("未找到匹配: {}", pattern),
                exit_code: 1,
            }
        } else {
            BuiltinCommandResult {
                output: results.join("\n"),
                exit_code: 0,
            }
        }
    }

    fn find_recurse(&self, dir: &PathBuf, pattern: &str, results: &mut Vec<String>, max_depth: usize) {
        if max_depth == 0 {
            return;
        }
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let relative = path.strip_prefix(&*self.working_dir.lock().expect("working_dir lock failed"))
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.to_string_lossy().to_string());

            if name.to_lowercase().contains(&pattern.to_lowercase()) {
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                if is_dir {
                    results.push(format!("{}/", relative));
                } else {
                    results.push(relative);
                }
            }
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                self.find_recurse(&path, pattern, results, max_depth - 1);
            }
        }
    }

    fn builtin_wc(&self, file: Option<&str>) -> BuiltinCommandResult {
        let file = match file {
            Some(f) => f,
            None => return BuiltinCommandResult {
                output: "用法: wc <文件>".into(),
                exit_code: 1,
            },
        };

        let wd = self.working_dir.lock().expect("working_dir lock failed");
        let target = wd.join(file);

        match std::fs::read_to_string(&target) {
            Ok(content) => {
                let lines = content.lines().count();
                let words = content.split_whitespace().count();
                let chars = content.chars().count();
                BuiltinCommandResult {
                    output: format!("{:>8} {:>8} {:>8} {}", lines, words, chars, file),
                    exit_code: 0,
                }
            }
            Err(e) => BuiltinCommandResult {
                output: format!("读取文件失败: {}", e),
                exit_code: 1,
            },
        }
    }

    fn builtin_head(&self, file: Option<&str>, lines_arg: Option<&str>) -> BuiltinCommandResult {
        let file = match file {
            Some(f) => f,
            None => return BuiltinCommandResult {
                output: "用法: head <文件> [行数]".into(),
                exit_code: 1,
            },
        };

        let n: usize = lines_arg
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);

        let wd = self.working_dir.lock().expect("working_dir lock failed");
        let target = wd.join(file);

        match std::fs::read_to_string(&target) {
            Ok(content) => {
                let preview: String = content.lines().take(n).collect::<Vec<&str>>().join("\n");
                BuiltinCommandResult {
                    output: preview,
                    exit_code: 0,
                }
            }
            Err(e) => BuiltinCommandResult {
                output: format!("读取文件失败: {}", e),
                exit_code: 1,
            },
        }
    }

    fn builtin_tail(&self, file: Option<&str>, lines_arg: Option<&str>) -> BuiltinCommandResult {
        let file = match file {
            Some(f) => f,
            None => return BuiltinCommandResult {
                output: "用法: tail <文件> [行数]".into(),
                exit_code: 1,
            },
        };

        let n: usize = lines_arg
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);

        let wd = self.working_dir.lock().expect("working_dir lock failed");
        let target = wd.join(file);

        match std::fs::read_to_string(&target) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let start = if lines.len() > n { lines.len() - n } else { 0 };
                let preview: String = lines[start..].join("\n");
                BuiltinCommandResult {
                    output: preview,
                    exit_code: 0,
                }
            }
            Err(e) => BuiltinCommandResult {
                output: format!("读取文件失败: {}", e),
                exit_code: 1,
            },
        }
    }

    fn builtin_cp(&self, src: Option<&str>, dst: Option<&str>) -> BuiltinCommandResult {
        let src = match src {
            Some(s) => s,
            None => return BuiltinCommandResult {
                output: "用法: cp <源文件> <目标路径>".into(),
                exit_code: 1,
            },
        };
        let dst = match dst {
            Some(d) => d,
            None => return BuiltinCommandResult {
                output: "用法: cp <源文件> <目标路径>".into(),
                exit_code: 1,
            },
        };

        let wd = self.working_dir.lock().expect("working_dir lock failed");
        let src_path = wd.join(src);
        let dst_path = wd.join(dst);

        if !src_path.exists() {
            return BuiltinCommandResult {
                output: format!("源文件不存在: {}", src),
                exit_code: 1,
            };
        }

        if src_path.is_dir() {
            return BuiltinCommandResult {
                output: format!("源路径是目录，请使用递归复制: {}", src),
                exit_code: 1,
            };
        }

        let dst_target = if dst_path.is_dir() {
            dst_path.join(src_path.file_name().unwrap_or_default())
        } else {
            dst_path
        };

        match std::fs::copy(&src_path, &dst_target) {
            Ok(bytes) => BuiltinCommandResult {
                output: format!("已复制: {} -> {} ({} 字节)", src, dst_target.display(), bytes),
                exit_code: 0,
            },
            Err(e) => BuiltinCommandResult {
                output: format!("复制失败: {}", e),
                exit_code: 1,
            },
        }
    }

    fn builtin_mv(&self, src: Option<&str>, dst: Option<&str>) -> BuiltinCommandResult {
        let src = match src {
            Some(s) => s,
            None => return BuiltinCommandResult {
                output: "用法: mv <源文件> <目标路径>".into(),
                exit_code: 1,
            },
        };
        let dst = match dst {
            Some(d) => d,
            None => return BuiltinCommandResult {
                output: "用法: mv <源文件> <目标路径>".into(),
                exit_code: 1,
            },
        };

        let wd = self.working_dir.lock().expect("working_dir lock failed");
        let src_path = wd.join(src);
        let dst_path = wd.join(dst);

        if !src_path.exists() {
            return BuiltinCommandResult {
                output: format!("源文件不存在: {}", src),
                exit_code: 1,
            };
        }

        let dst_target = if dst_path.is_dir() {
            dst_path.join(src_path.file_name().unwrap_or_default())
        } else {
            dst_path
        };

        match std::fs::rename(&src_path, &dst_target) {
            Ok(_) => BuiltinCommandResult {
                output: format!("已移动: {} -> {}", src, dst_target.display()),
                exit_code: 0,
            },
            Err(e) => BuiltinCommandResult {
                output: format!("移动失败: {}", e),
                exit_code: 1,
            },
        }
    }

    fn builtin_rm(&self, target: Option<&str>) -> BuiltinCommandResult {
        let target = match target {
            Some(t) => t,
            None => return BuiltinCommandResult {
                output: "用法: rm <文件或目录>".into(),
                exit_code: 1,
            },
        };

        let wd = self.working_dir.lock().expect("working_dir lock failed");
        let target_path = wd.join(target);

        if !target_path.exists() {
            return BuiltinCommandResult {
                output: format!("文件或目录不存在: {}", target),
                exit_code: 1,
            };
        }

        let result = if target_path.is_dir() {
            std::fs::remove_dir_all(&target_path)
        } else {
            std::fs::remove_file(&target_path)
        };

        match result {
            Ok(_) => BuiltinCommandResult {
                output: format!("已删除: {}", target),
                exit_code: 0,
            },
            Err(e) => BuiltinCommandResult {
                output: format!("删除失败: {}", e),
                exit_code: 1,
            },
        }
    }

    fn builtin_touch(&self, file: Option<&str>) -> BuiltinCommandResult {
        let file = match file {
            Some(f) => f,
            None => return BuiltinCommandResult {
                output: "用法: touch <文件名>".into(),
                exit_code: 1,
            },
        };

        let wd = self.working_dir.lock().expect("working_dir lock failed");
        let target = wd.join(file);

        if target.exists() {
            match filetime::set_file_mtime(
                &target,
                filetime::FileTime::now(),
            ) {
                Ok(_) => BuiltinCommandResult {
                    output: format!("已更新时间戳: {}", file),
                    exit_code: 0,
                },
                Err(e) => BuiltinCommandResult {
                    output: format!("更新时间戳失败: {}", e),
                    exit_code: 1,
                },
            }
        } else {
            match std::fs::write(&target, "") {
                Ok(_) => BuiltinCommandResult {
                    output: format!("已创建文件: {}", file),
                    exit_code: 0,
                },
                Err(e) => BuiltinCommandResult {
                    output: format!("创建文件失败: {}", e),
                    exit_code: 1,
                },
            }
        }
    }

    fn builtin_clearscrollback(&self) -> BuiltinCommandResult {
        BuiltinCommandResult {
            output: "__CLEAR_SCROLLBACK__".into(),
            exit_code: 0,
        }
    }

    fn builtin_reset(&self) -> BuiltinCommandResult {
        let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let mut wd = self.working_dir.lock().expect("working_dir lock failed");
        *wd = home.clone();
        BuiltinCommandResult {
            output: format!("终端状态已重置\n工作目录已恢复: {}", home.display()),
            exit_code: 0,
        }
    }

    fn builtin_fontsize(&self, arg: Option<&str>) -> BuiltinCommandResult {
        match arg {
            Some("+") | Some("up") | Some("increase") => BuiltinCommandResult {
                output: "__FONTSIZE_INCREASE__".into(),
                exit_code: 0,
            },
            Some("-") | Some("down") | Some("decrease") => BuiltinCommandResult {
                output: "__FONTSIZE_DECREASE__".into(),
                exit_code: 0,
            },
            Some("0") | Some("reset") => BuiltinCommandResult {
                output: "__FONTSIZE_RESET__".into(),
                exit_code: 0,
            },
            _ => BuiltinCommandResult {
                output: "用法: fontsize <+/up/increase | -/down/decrease | 0/reset>\n\n  +  增大字体 (+10%)\n  -  减小字体 (-10%)\n  0  重置字体大小\n\n快捷键: Ctrl+= 增大 | Ctrl+- 减小 | Ctrl+0 重置".into(),
                exit_code: 1,
            },
        }
    }

    fn builtin_fullscreen(&self) -> BuiltinCommandResult {
        BuiltinCommandResult {
            output: "__FULLSCREEN_TOGGLE__".into(),
            exit_code: 0,
        }
    }

    fn builtin_reload(&self) -> BuiltinCommandResult {
        BuiltinCommandResult {
            output: "__RELOAD_CONFIG__".into(),
            exit_code: 0,
        }
    }

    fn builtin_scroll(&self, direction: Option<&str>, amount: Option<&str>) -> BuiltinCommandResult {
        match direction {
            Some("top") => BuiltinCommandResult {
                output: "__SCROLL_TOP__".into(),
                exit_code: 0,
            },
            Some("bottom") => BuiltinCommandResult {
                output: "__SCROLL_BOTTOM__".into(),
                exit_code: 0,
            },
            Some("up") => {
                let n = amount.and_then(|s| s.parse::<u16>().ok()).unwrap_or(1);
                BuiltinCommandResult {
                    output: format!("__SCROLL_UP_{}__", n),
                    exit_code: 0,
                }
            }
            Some("down") => {
                let n = amount.and_then(|s| s.parse::<u16>().ok()).unwrap_or(1);
                BuiltinCommandResult {
                    output: format!("__SCROLL_DOWN_{}__", n),
                    exit_code: 0,
                }
            }
            _ => BuiltinCommandResult {
                output: "用法: scroll <top | bottom | up [行数] | down [行数]>\n\n  top      滚动到回滚顶部\n  bottom   滚动到回滚底部\n  up N     向上滚动 N 行（默认1）\n  down N   向下滚动 N 行（默认1）\n\n快捷键: Shift+PageUp 上翻页 | Shift+PageDown 下翻页".into(),
                exit_code: 1,
            },
        }
    }

    fn builtin_hide(&self) -> BuiltinCommandResult {
        BuiltinCommandResult {
            output: "__WINDOW_HIDE__".into(),
            exit_code: 0,
        }
    }

    fn builtin_quit(&self) -> BuiltinCommandResult {
        BuiltinCommandResult {
            output: "__APP_QUIT__".into(),
            exit_code: 0,
        }
    }

    fn builtin_alwaysontop(&self) -> BuiltinCommandResult {
        BuiltinCommandResult {
            output: "__ALWAYS_ON_TOP__".into(),
            exit_code: 0,
        }
    }

    pub fn get_system_info(&self) -> SystemInfo {
        SystemInfo {
            current_dir: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "/".into()),
            username: whoami::username(),
            os: std::env::consts::OS.to_string(),
            hostname: whoami::fallible::hostname().unwrap_or_else(|_| "unknown".into()),
        }
    }

    pub fn list_directory(&self, path: Option<&str>) -> Result<DirectoryListing, AppError> {
        let dir = path.unwrap_or(".");
        let entries = std::fs::read_dir(dir)
            .map_err(|e| AppError::TerminalError(format!("无法读取目录: {}", e)))?;

        let mut result = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let metadata = entry.metadata().ok();
            result.push(DirectoryEntry {
                is_dir: metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false),
                name,
                size: metadata.map(|m| m.len()).unwrap_or(0),
            });
        }

        Ok(DirectoryListing {
            path: std::fs::canonicalize(dir)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| dir.to_string()),
            entries: result,
        })
    }

    pub fn detect_wsl() -> WslStatus {
        let wsl_exe = std::process::Command::new("wsl.exe")
            .args(["--status"])
            .output();

        let mut status = WslStatus {
            installed: false,
            running: false,
            default_distro: None,
            distributions: Vec::new(),
            wsl_version: None,
        };

        match wsl_exe {
            Ok(output) => {
                status.installed = true;
                let stdout = String::from_utf8_lossy(&output.stdout);

                for line in stdout.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("默认版本:") || trimmed.starts_with("Default Version:") {
                        status.wsl_version = Some(
                            trimmed
                                .split(':')
                                .nth(1)
                                .unwrap_or("")
                                .trim()
                                .to_string(),
                        );
                    }
                    if trimmed.starts_with("默认分发:") || trimmed.starts_with("Default Distribution:") {
                        status.default_distro = Some(
                            trimmed
                                .split(':')
                                .nth(1)
                                .unwrap_or("")
                                .trim()
                                .to_string(),
                        );
                    }
                }
            }
            Err(_) => return status,
        }

        let list_output = std::process::Command::new("wsl.exe")
            .args(["-l", "-v", "--all"])
            .output();

        if let Ok(output) = list_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut distros = Vec::new();

            for line in stdout.lines().skip(1) {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3 {
                    let name = parts[0].trim_matches('*').to_string();
                    let is_default = trimmed.starts_with('*');
                    let running = parts[1].trim().to_lowercase() == "running";
                    let version = parts[2].parse::<u32>().unwrap_or(2);

                    distros.push(WslDistribution {
                        name,
                        running,
                        version,
                        default: is_default,
                    });
                }
            }

            status.running = distros.iter().any(|d| d.running);
            status.distributions = distros;
        }

        status
    }
}