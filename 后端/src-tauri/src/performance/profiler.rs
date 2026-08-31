//! 性能分析器

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// 性能分析器
#[derive(Debug)]
pub struct Profiler {
    /// 分析记录 (名称 -> 总耗时)
    records: Mutex<HashMap<String, ProfileRecord>>,
    /// 是否启用
    enabled: bool,
}

/// 分析记录
#[derive(Debug, Clone)]
struct ProfileRecord {
    total_duration_ms: u64,
    count: u64,
    min_ms: u64,
    max_ms: u64,
}

impl ProfileRecord {
    fn new(duration_ms: u64) -> Self {
        Self {
            total_duration_ms: duration_ms,
            count: 1,
            min_ms: duration_ms,
            max_ms: duration_ms,
        }
    }

    fn record(&mut self, duration_ms: u64) {
        self.total_duration_ms += duration_ms;
        self.count += 1;
        if duration_ms < self.min_ms {
            self.min_ms = duration_ms;
        }
        if duration_ms > self.max_ms {
            self.max_ms = duration_ms;
        }
    }

    fn avg_ms(&self) -> f64 {
        if self.count > 0 {
            self.total_duration_ms as f64 / self.count as f64
        } else {
            0.0
        }
    }
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            records: Mutex::new(HashMap::new()),
            enabled: true,
        }
    }

    /// 启用/禁用分析器
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 记录操作耗时
    pub fn record(&self, name: &str, duration_ms: u64) {
        if !self.enabled {
            return;
        }
        let mut records = self.records.lock().unwrap();
        records
            .entry(name.to_string())
            .and_modify(|r| r.record(duration_ms))
            .or_insert_with(|| ProfileRecord::new(duration_ms));
    }

    /// 创建分析作用域
    pub fn scope(&self, name: &str) -> ProfileScope<'_> {
        ProfileScope {
            profiler: self,
            name: name.to_string(),
            start: Instant::now(),
        }
    }

    /// 获取分析报告
    pub fn report(&self) -> ProfileReport {
        let records = self.records.lock().unwrap();
        let mut entries: Vec<ProfileEntry> = records
            .iter()
            .map(|(name, record)| ProfileEntry {
                name: name.clone(),
                total_ms: record.total_duration_ms,
                count: record.count,
                avg_ms: record.avg_ms(),
                min_ms: record.min_ms,
                max_ms: record.max_ms,
            })
            .collect();

        // 按总耗时降序排序
        entries.sort_by(|a, b| b.total_ms.cmp(&a.total_ms));

        ProfileReport { entries }
    }

    /// 重置分析数据
    pub fn reset(&self) {
        self.records.lock().unwrap().clear();
    }
}

/// 分析作用域
pub struct ProfileScope<'a> {
    profiler: &'a Profiler,
    name: String,
    start: Instant,
}

impl<'a> Drop for ProfileScope<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_millis() as u64;
        self.profiler.record(&self.name, duration);
    }
}

/// 分析报告
#[derive(Debug, Clone)]
pub struct ProfileReport {
    pub entries: Vec<ProfileEntry>,
}

/// 分析条目
#[derive(Debug, Clone)]
pub struct ProfileEntry {
    pub name: String,
    pub total_ms: u64,
    pub count: u64,
    pub avg_ms: f64,
    pub min_ms: u64,
    pub max_ms: u64,
}