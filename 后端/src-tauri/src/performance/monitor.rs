//! 性能监控器

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Instant;

use super::PerformanceMetrics;

/// 性能监控器
#[derive(Debug)]
pub struct PerformanceMonitor {
    /// 指标历史
    metrics: Mutex<VecDeque<PerformanceMetrics>>,
    /// 最大历史记录数
    max_history: usize,
    /// 启动时间
    start_time: Instant,
    /// 总操作数
    total_ops: Mutex<u64>,
    /// 总耗时
    total_duration: Mutex<u64>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics: Mutex::new(VecDeque::new()),
            max_history: 1000,
            start_time: Instant::now(),
            total_ops: Mutex::new(0),
            total_duration: Mutex::new(0),
        }
    }

    /// 记录性能指标
    pub fn record(&self, metrics: PerformanceMetrics) {
        let mut metrics_queue = self.metrics.lock().unwrap();

        if metrics_queue.len() >= self.max_history {
            metrics_queue.pop_front();
        }
        metrics_queue.push_back(metrics.clone());

        *self.total_ops.lock().unwrap() += 1;
        *self.total_duration.lock().unwrap() += metrics.duration_ms;
    }

    /// 获取最近的指标
    pub fn recent_metrics(&self, count: usize) -> Vec<PerformanceMetrics> {
        let metrics = self.metrics.lock().unwrap();
        metrics
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// 获取统计信息
    pub fn stats(&self) -> PerformanceStats {
        let total_ops = *self.total_ops.lock().unwrap();
        let total_duration = *self.total_duration.lock().unwrap();

        let avg_duration = if total_ops > 0 {
            total_duration / total_ops
        } else {
            0
        };

        let ops_per_sec = if self.start_time.elapsed().as_secs() > 0 {
            total_ops as f64 / self.start_time.elapsed().as_secs_f64()
        } else {
            0.0
        };

        let metrics = self.metrics.lock().unwrap();
        let cache_hit_rate = if !metrics.is_empty() {
            let cache_hits = metrics.iter().filter(|m| m.from_cache).count();
            cache_hits as f64 / metrics.len() as f64
        } else {
            0.0
        };

        PerformanceStats {
            total_ops,
            total_duration_ms: total_duration,
            avg_duration_ms: avg_duration,
            ops_per_second: ops_per_sec,
            cache_hit_rate,
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }

    /// 重置统计
    pub fn reset(&self) {
        self.metrics.lock().unwrap().clear();
        *self.total_ops.lock().unwrap() = 0;
        *self.total_duration.lock().unwrap() = 0;
    }
}

/// 性能统计信息
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub total_ops: u64,
    pub total_duration_ms: u64,
    pub avg_duration_ms: u64,
    pub ops_per_second: f64,
    pub cache_hit_rate: f64,
    pub uptime_seconds: u64,
}