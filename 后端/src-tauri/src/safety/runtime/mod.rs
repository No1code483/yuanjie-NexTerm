//! 运行时安全监控 — 对标 Codex Guardian
//!
//! 提供代码执行时的实时安全监控：
//! - 系统调用追踪
//! - 资源限制
//! - 异常行为检测

pub mod anomaly;
pub mod resource_limit;
pub mod syscall_trace;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// 运行时监控事件
#[derive(Debug, Clone)]
pub enum RuntimeEvent {
    /// 资源使用警告
    ResourceWarning {
        resource: String,
        current: u64,
        limit: u64,
    },
    /// 异常行为检测
    AnomalyDetected {
        anomaly_type: String,
        description: String,
        severity: AnomalySeverity,
    },
    /// 超时
    Timeout { elapsed_ms: u64, timeout_ms: u64 },
    /// 资源耗尽
    ResourceExhausted {
        resource: String,
        used: u64,
        limit: u64,
    },
}

/// 异常严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalySeverity {
    Info,
    Warning,
    Critical,
}

/// 运行时监控器
#[derive(Debug)]
pub struct RuntimeMonitor {
    /// 监控是否活跃
    active: Arc<AtomicBool>,
    /// 开始时间
    start_time: Option<Instant>,
    /// 资源限制
    limits: Arc<ResourceLimits>,
    /// 事件回调
    _events: Vec<RuntimeEvent>,
}

/// 资源限制
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// 最大内存 (MB)
    pub max_memory_mb: u64,
    /// 最大 CPU 时间 (ms)
    pub max_cpu_time_ms: u64,
    /// 最大磁盘 IO (MB)
    pub max_disk_io_mb: u64,
    /// 最大进程数
    pub max_processes: u32,
    /// 当前内存使用
    pub current_memory: Arc<AtomicU64>,
    /// 当前 CPU 时间
    pub current_cpu_time: Arc<AtomicU64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            max_cpu_time_ms: 30_000,
            max_disk_io_mb: 100,
            max_processes: 10,
            current_memory: Arc::new(AtomicU64::new(0)),
            current_cpu_time: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl ResourceLimits {
    /// 检查内存是否超限
    pub fn check_memory(&self) -> Option<RuntimeEvent> {
        let current = self.current_memory.load(Ordering::Relaxed);
        if current > self.max_memory_mb {
            Some(RuntimeEvent::ResourceExhausted {
                resource: "memory".into(),
                used: current,
                limit: self.max_memory_mb,
            })
        } else if current > self.max_memory_mb * 80 / 100 {
            Some(RuntimeEvent::ResourceWarning {
                resource: "memory".into(),
                current,
                limit: self.max_memory_mb,
            })
        } else {
            None
        }
    }

    /// 检查 CPU 时间是否超限
    pub fn check_cpu(&self) -> Option<RuntimeEvent> {
        let current = self.current_cpu_time.load(Ordering::Relaxed);
        if current > self.max_cpu_time_ms {
            Some(RuntimeEvent::ResourceExhausted {
                resource: "cpu_time".into(),
                used: current,
                limit: self.max_cpu_time_ms,
            })
        } else if current > self.max_cpu_time_ms * 80 / 100 {
            Some(RuntimeEvent::ResourceWarning {
                resource: "cpu_time".into(),
                current,
                limit: self.max_cpu_time_ms,
            })
        } else {
            None
        }
    }
}

impl RuntimeMonitor {
    pub fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            start_time: None,
            limits: Arc::new(ResourceLimits::default()),
            _events: Vec::new(),
        }
    }

    pub fn with_limits(limits: ResourceLimits) -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            start_time: None,
            limits: Arc::new(limits),
            _events: Vec::new(),
        }
    }

    /// 开始监控
    pub fn start(&mut self) {
        self.active.store(true, Ordering::SeqCst);
        self.start_time = Some(Instant::now());
    }

    /// 停止监控
    pub fn stop(&mut self) {
        self.active.store(false, Ordering::SeqCst);
    }

    /// 检查是否活跃
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// 获取已运行时间
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0)
    }

    /// 检查所有限制
    pub fn check_limits(&self) -> Vec<RuntimeEvent> {
        let mut events = Vec::new();

        if let Some(event) = self.limits.check_memory() {
            events.push(event);
        }
        if let Some(event) = self.limits.check_cpu() {
            events.push(event);
        }

        events
    }

    /// 获取资源限制引用
    pub fn limits(&self) -> &Arc<ResourceLimits> {
        &self.limits
    }

    /// 检查是否超时
    pub fn check_timeout(&self, timeout_ms: u64) -> Option<RuntimeEvent> {
        let elapsed = self.elapsed_ms();
        if elapsed > timeout_ms {
            Some(RuntimeEvent::Timeout {
                elapsed_ms: elapsed,
                timeout_ms,
            })
        } else {
            None
        }
    }
}

/// 监控守卫 — 自动在 drop 时停止监控
pub struct MonitorGuard {
    monitor: Option<RuntimeMonitor>,
}

impl MonitorGuard {
    pub fn new(monitor: RuntimeMonitor) -> Self {
        Self {
            monitor: Some(monitor),
        }
    }

    pub fn monitor(&self) -> Option<&RuntimeMonitor> {
        self.monitor.as_ref()
    }
}

impl Drop for MonitorGuard {
    fn drop(&mut self) {
        if let Some(ref mut monitor) = self.monitor {
            monitor.stop();
        }
    }
}