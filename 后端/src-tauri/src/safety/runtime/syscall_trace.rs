//! 系统调用追踪
//!
//! 监控和追踪子进程的系统调用，检测可疑行为。
//! 在 Linux 上通过 ptrace，在 Windows 上通过 ETW。

use super::{AnomalySeverity, RuntimeEvent};

/// 系统调用事件
#[derive(Debug, Clone)]
pub struct SyscallEvent {
    /// 系统调用名称
    pub name: String,
    /// 参数
    pub args: Vec<String>,
    /// 返回值
    pub result: i64,
    /// 时间戳
    pub timestamp_ms: u64,
}

/// 系统调用追踪器
#[derive(Debug)]
pub struct SyscallTracer {
    /// 允许的系统调用白名单
    allowed_syscalls: Vec<String>,
    /// 禁止的系统调用黑名单
    blocked_syscalls: Vec<String>,
    /// 记录的事件
    events: Vec<SyscallEvent>,
    /// 是否启用追踪
    enabled: bool,
}

impl SyscallTracer {
    pub fn new() -> Self {
        Self {
            allowed_syscalls: vec![
                "read".into(),
                "write".into(),
                "open".into(),
                "close".into(),
                "stat".into(),
                "fstat".into(),
                "lseek".into(),
                "mmap".into(),
                "mprotect".into(),
                "brk".into(),
                "rt_sigaction".into(),
                "exit_group".into(),
            ],
            blocked_syscalls: vec![
                "fork".into(),
                "clone".into(),
                "execve".into(),
                "ptrace".into(),
                "mount".into(),
                "umount2".into(),
                "reboot".into(),
                "kexec_load".into(),
                "init_module".into(),
                "delete_module".into(),
            ],
            events: Vec::new(),
            enabled: false,
        }
    }

    /// 启用追踪
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// 禁用追踪
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// 记录系统调用
    pub fn record(&mut self, event: SyscallEvent) -> Option<RuntimeEvent> {
        if !self.enabled {
            return None;
        }

        self.events.push(event.clone());

        // 检查黑名单
        if self.blocked_syscalls.contains(&event.name.to_lowercase()) {
            return Some(RuntimeEvent::AnomalyDetected {
                anomaly_type: "blocked_syscall".into(),
                description: format!("检测到禁止的系统调用: {}", event.name),
                severity: AnomalySeverity::Critical,
            });
        }

        // 检查白名单
        if !self.allowed_syscalls.is_empty()
            && !self
                .allowed_syscalls
                .contains(&event.name.to_lowercase())
        {
            return Some(RuntimeEvent::AnomalyDetected {
                anomaly_type: "unlisted_syscall".into(),
                description: format!("检测到未在白名单中的系统调用: {}", event.name),
                severity: AnomalySeverity::Warning,
            });
        }

        None
    }

    /// 获取记录的事件数量
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// 获取最近的系统调用
    pub fn recent_events(&self, count: usize) -> &[SyscallEvent] {
        let start = self.events.len().saturating_sub(count);
        &self.events[start..]
    }

    /// 清空事件记录
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

/// 常用的危险系统调用模式
pub const DANGEROUS_SYSCALL_PATTERNS: &[(&str, &str)] = &[
    ("execve", "执行外部程序"),
    ("fork", "创建子进程"),
    ("clone", "克隆进程"),
    ("ptrace", "追踪其他进程"),
    ("mount", "挂载文件系统"),
    ("reboot", "重启系统"),
    ("kexec_load", "加载新内核"),
    ("init_module", "加载内核模块"),
    ("delete_module", "卸载内核模块"),
    ("ioperm", "修改I/O权限"),
    ("iopl", "修改I/O权限级别"),
];