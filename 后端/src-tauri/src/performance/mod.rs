//! 性能监控 — 对标 VSCode Telemetry/Performance
//!
//! 提供请求耗时追踪、内存使用监控、性能热点分析等功能。

pub mod cache;
pub mod monitor;
pub mod profiler;

use std::time::Instant;

/// 性能计时器
#[derive(Debug)]
pub struct PerfTimer {
    name: String,
    start: Instant,
    /// 子计时器
    children: Vec<PerfTimer>,
}

impl PerfTimer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start: Instant::now(),
            children: Vec::new(),
        }
    }

    /// 创建子计时器
    pub fn child(&mut self, name: &str) -> &mut PerfTimer {
        let child = PerfTimer::new(name);
        self.children.push(child);
        self.children.last_mut().unwrap()
    }

    /// 停止计时并返回耗时
    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    /// 获取计时报告
    pub fn report(&self) -> String {
        let mut report = format!("[{}] {}ms", self.name, self.elapsed_ms());
        for child in &self.children {
            report.push_str(&format!("\n  └ {}", child.report()));
        }
        report
    }
}

/// 性能指标
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// 操作名称
    pub operation: String,
    /// 耗时 (ms)
    pub duration_ms: u64,
    /// 内存增量 (bytes)
    pub memory_delta: i64,
    /// 是否来自缓存
    pub from_cache: bool,
    /// 时间戳
    pub timestamp: i64,
}

/// 性能级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceLevel {
    Fast,
    Normal,
    Slow,
    Critical,
}

impl PerformanceLevel {
    pub fn from_duration(ms: u64) -> Self {
        match ms {
            0..=100 => PerformanceLevel::Fast,
            101..=500 => PerformanceLevel::Normal,
            501..=2000 => PerformanceLevel::Slow,
            _ => PerformanceLevel::Critical,
        }
    }
}