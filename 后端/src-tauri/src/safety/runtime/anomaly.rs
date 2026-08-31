//! 异常行为检测
//!
//! 检测沙箱中的异常行为模式：
//! - fork bomb 检测
//! - 无限循环检测
//! - 可疑文件操作检测
//! - 可疑网络行为检测

use std::collections::VecDeque;
use std::time::Instant;

use super::{AnomalySeverity, RuntimeEvent};

/// 异常检测器
#[derive(Debug)]
pub struct AnomalyDetector {
    /// 进程创建速率记录
    process_spawn_times: VecDeque<Instant>,
    /// 文件操作速率记录
    file_op_times: VecDeque<Instant>,
    /// 网络连接记录
    network_events: VecDeque<NetworkEvent>,
    /// 磁盘写入速率记录
    disk_write_sizes: VecDeque<(Instant, u64)>,
    /// 配置
    config: AnomalyConfig,
}

/// 异常检测配置
#[derive(Debug, Clone)]
pub struct AnomalyConfig {
    /// fork bomb 检测: 时间窗口内的最大进程数
    pub fork_bomb_threshold: usize,
    /// fork bomb 检测: 时间窗口 (秒)
    pub fork_bomb_window_secs: u64,
    /// 文件操作风暴: 时间窗口内的最大文件操作数
    pub file_op_threshold: usize,
    /// 文件操作风暴: 时间窗口 (秒)
    pub file_op_window_secs: u64,
    /// 磁盘写入风暴: 最大写入速率 (bytes/sec)
    pub disk_write_rate_limit: u64,
    /// 网络连接数阈值
    pub network_conn_threshold: usize,
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self {
            fork_bomb_threshold: 50,
            fork_bomb_window_secs: 5,
            file_op_threshold: 100,
            file_op_window_secs: 5,
            disk_write_rate_limit: 10 * 1024 * 1024, // 10 MB/s
            network_conn_threshold: 20,
        }
    }
}

/// 网络事件
#[derive(Debug, Clone)]
pub struct NetworkEvent {
    pub timestamp: Instant,
    pub remote_addr: String,
    pub port: u16,
    pub event_type: NetworkEventType,
}

#[derive(Debug, Clone)]
pub enum NetworkEventType {
    Connect,
    Listen,
    Send,
    Receive,
}

impl AnomalyDetector {
    pub fn new() -> Self {
        Self {
            process_spawn_times: VecDeque::new(),
            file_op_times: VecDeque::new(),
            network_events: VecDeque::new(),
            disk_write_sizes: VecDeque::new(),
            config: AnomalyConfig::default(),
        }
    }

    pub fn with_config(config: AnomalyConfig) -> Self {
        Self {
            process_spawn_times: VecDeque::new(),
            file_op_times: VecDeque::new(),
            network_events: VecDeque::new(),
            disk_write_sizes: VecDeque::new(),
            config,
        }
    }

    /// 记录进程创建
    pub fn record_process_spawn(&mut self) -> Option<RuntimeEvent> {
        let now = Instant::now();
        self.process_spawn_times.push_back(now);

        // 清理过期记录
        let window = std::time::Duration::from_secs(self.config.fork_bomb_window_secs);
        while self
            .process_spawn_times
            .front()
            .map_or(false, |t| now.duration_since(*t) > window)
        {
            self.process_spawn_times.pop_front();
        }

        if self.process_spawn_times.len() > self.config.fork_bomb_threshold {
            Some(RuntimeEvent::AnomalyDetected {
                anomaly_type: "fork_bomb".into(),
                description: format!(
                    "检测到可疑进程创建风暴: {} 进程在 {} 秒内",
                    self.process_spawn_times.len(),
                    self.config.fork_bomb_window_secs
                ),
                severity: AnomalySeverity::Critical,
            })
        } else {
            None
        }
    }

    /// 记录文件操作
    pub fn record_file_op(&mut self) -> Option<RuntimeEvent> {
        let now = Instant::now();
        self.file_op_times.push_back(now);

        let window = std::time::Duration::from_secs(self.config.file_op_window_secs);
        while self
            .file_op_times
            .front()
            .map_or(false, |t| now.duration_since(*t) > window)
        {
            self.file_op_times.pop_front();
        }

        if self.file_op_times.len() > self.config.file_op_threshold {
            Some(RuntimeEvent::AnomalyDetected {
                anomaly_type: "file_op_storm".into(),
                description: format!(
                    "检测到可疑文件操作风暴: {} 操作在 {} 秒内",
                    self.file_op_times.len(),
                    self.config.file_op_window_secs
                ),
                severity: AnomalySeverity::Warning,
            })
        } else {
            None
        }
    }

    /// 记录磁盘写入
    pub fn record_disk_write(&mut self, bytes: u64) -> Option<RuntimeEvent> {
        let now = Instant::now();
        self.disk_write_sizes.push_back((now, bytes));

        // 只保留最近 10 秒的记录
        let window = std::time::Duration::from_secs(10);
        while self
            .disk_write_sizes
            .front()
            .map_or(false, |(t, _)| now.duration_since(*t) > window)
        {
            self.disk_write_sizes.pop_front();
        }

        // 计算写入速率
        let total_bytes: u64 = self.disk_write_sizes.iter().map(|(_, b)| b).sum();
        let rate = total_bytes / 10; // bytes/sec

        if rate > self.config.disk_write_rate_limit {
            Some(RuntimeEvent::AnomalyDetected {
                anomaly_type: "disk_write_storm".into(),
                description: format!(
                    "磁盘写入速率过高: {} bytes/sec (限制: {})",
                    rate, self.config.disk_write_rate_limit
                ),
                severity: AnomalySeverity::Warning,
            })
        } else {
            None
        }
    }

    /// 记录网络事件
    pub fn record_network_event(&mut self, event: NetworkEvent) -> Option<RuntimeEvent> {
        self.network_events.push_back(event);

        // 只保留最近 30 秒的记录
        let now = Instant::now();
        let window = std::time::Duration::from_secs(30);
        while self
            .network_events
            .front()
            .map_or(false, |e| now.duration_since(e.timestamp) > window)
        {
            self.network_events.pop_front();
        }

        if self.network_events.len() > self.config.network_conn_threshold {
            Some(RuntimeEvent::AnomalyDetected {
                anomaly_type: "network_storm".into(),
                description: format!(
                    "网络连接数异常: {} 连接在 30 秒内",
                    self.network_events.len()
                ),
                severity: AnomalySeverity::Warning,
            })
        } else {
            None
        }
    }

    /// 清除所有记录
    pub fn clear(&mut self) {
        self.process_spawn_times.clear();
        self.file_op_times.clear();
        self.network_events.clear();
        self.disk_write_sizes.clear();
    }
}