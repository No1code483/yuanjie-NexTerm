use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use sysinfo::{Disks, System};
use uuid::Uuid;

use crate::models::intelligence::{
    AnomalyAlert, AnomalyDetectionResult, AutonomousDecisionResult, KnowledgeAssociation,
    KnowledgeOrganizationResult, ScheduledTask, SystemResourceSnapshot, TaskExecutionResult,
    UserActivity,
};

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn parse_timestamp(ts: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.timestamp())
        .unwrap_or(0)
}

fn uuid_v4() -> String {
    Uuid::new_v4().to_string()
}

pub struct AnomalyDetector;

impl AnomalyDetector {
    pub fn snapshot_resources() -> SystemResourceSnapshot {
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu = sys.global_cpu_usage() as f64;
        let total_mem = sys.total_memory() / 1024;
        let used_mem = sys.used_memory() / 1024;
        let mem_pct = if total_mem > 0 {
            (used_mem as f64 / total_mem as f64) * 100.0
        } else {
            0.0
        };

        let disks = Disks::new_with_refreshed_list();
        let mut disk_total_gb = 0u64;
        let mut disk_free_gb = 0u64;
        for disk in &disks {
            if disk.mount_point().to_string_lossy().starts_with('/')
                || disk.mount_point().to_string_lossy().contains(":\\")
            {
                disk_total_gb += disk.total_space() / (1024 * 1024 * 1024);
                disk_free_gb += disk.available_space() / (1024 * 1024 * 1024);
            }
        }
        let disk_pct = if disk_total_gb > 0 {
            ((disk_total_gb - disk_free_gb) as f64 / disk_total_gb as f64) * 100.0
        } else {
            0.0
        };

        let process_count = sys.processes().len();
        let uptime = System::uptime();

        SystemResourceSnapshot {
            cpu_usage_percent: (cpu * 100.0).round(),
            memory_used_mb: used_mem,
            memory_total_mb: total_mem,
            memory_usage_percent: (mem_pct * 10.0).round() / 10.0,
            disk_free_gb: disk_free_gb as f64,
            disk_total_gb: disk_total_gb as f64,
            disk_usage_percent: (disk_pct * 10.0).round() / 10.0,
            process_count,
            uptime_secs: uptime,
            timestamp: now_iso(),
        }
    }

    pub fn detect(
        activities: &[UserActivity],
        window_secs: u64,
    ) -> AnomalyDetectionResult {
        let resource = Self::snapshot_resources();
        let mut alerts: Vec<AnomalyAlert> = Vec::new();

        let now_ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let window_start = now_ts.saturating_sub(window_secs);

        if resource.cpu_usage_percent > 90.0 {
            alerts.push(AnomalyAlert {
                id: uuid_v4(),
                alert_type: "resource_cpu".into(),
                severity: "critical".into(),
                title: "CPU 使用率过高".into(),
                message: format!(
                    "当前 CPU 使用率 {:.0}%，已达临界水平。建议关闭不必要的后台进程。",
                    resource.cpu_usage_percent
                ),
                resource_snapshot: Some(resource.clone()),
                suggested_action: Some("close_heavy_processes".into()),
                triggered_by: "cpu_usage > 90%".into(),
                timestamp: now_iso(),
            });
        } else if resource.cpu_usage_percent > 75.0 {
            alerts.push(AnomalyAlert {
                id: uuid_v4(),
                alert_type: "resource_cpu".into(),
                severity: "warning".into(),
                title: "CPU 使用率偏高".into(),
                message: format!(
                    "当前 CPU 使用率 {:.0}%，建议留意系统性能。",
                    resource.cpu_usage_percent
                ),
                resource_snapshot: Some(resource.clone()),
                suggested_action: Some("monitor_performance".into()),
                triggered_by: "cpu_usage > 75%".into(),
                timestamp: now_iso(),
            });
        }

        if resource.memory_usage_percent > 90.0 {
            alerts.push(AnomalyAlert {
                id: uuid_v4(),
                alert_type: "resource_memory".into(),
                severity: "critical".into(),
                title: "内存使用率过高".into(),
                message: format!(
                    "当前内存使用 {}/{} MB ({:.0}%)，可能影响应用稳定性。",
                    resource.memory_used_mb,
                    resource.memory_total_mb,
                    resource.memory_usage_percent
                ),
                resource_snapshot: Some(resource.clone()),
                suggested_action: Some("free_memory".into()),
                triggered_by: "memory_usage > 90%".into(),
                timestamp: now_iso(),
            });
        } else if resource.memory_usage_percent > 80.0 {
            alerts.push(AnomalyAlert {
                id: uuid_v4(),
                alert_type: "resource_memory".into(),
                severity: "warning".into(),
                title: "内存使用率偏高".into(),
                message: format!(
                    "当前内存使用 {:.0}%，建议关闭不用的标签页和程序。",
                    resource.memory_usage_percent
                ),
                resource_snapshot: Some(resource.clone()),
                suggested_action: Some("optimize_memory".into()),
                triggered_by: "memory_usage > 80%".into(),
                timestamp: now_iso(),
            });
        }

        if resource.disk_usage_percent > 95.0 {
            alerts.push(AnomalyAlert {
                id: uuid_v4(),
                alert_type: "resource_disk".into(),
                severity: "critical".into(),
                title: "磁盘空间严重不足".into(),
                message: format!(
                    "磁盘已使用 {:.0}%，仅剩 {} GB 可用空间。建议清理回收站和临时文件。",
                    resource.disk_usage_percent, resource.disk_free_gb
                ),
                resource_snapshot: Some(resource.clone()),
                suggested_action: Some("cleanup_disk".into()),
                triggered_by: "disk_usage > 95%".into(),
                timestamp: now_iso(),
            });
        } else if resource.disk_usage_percent > 85.0 {
            alerts.push(AnomalyAlert {
                id: uuid_v4(),
                alert_type: "resource_disk".into(),
                severity: "warning".into(),
                title: "磁盘空间偏低".into(),
                message: format!(
                    "磁盘已使用 {:.0}%，可用空间 {} GB。建议定期清理。",
                    resource.disk_usage_percent, resource.disk_free_gb
                ),
                resource_snapshot: Some(resource.clone()),
                suggested_action: Some("schedule_cleanup".into()),
                triggered_by: "disk_usage > 85%".into(),
                timestamp: now_iso(),
            });
        }

        let recent: Vec<&UserActivity> = activities
            .iter()
            .filter(|a| {
                let ts = parse_timestamp(&a.timestamp) as u64;
                ts >= window_start
            })
            .collect();

        let total = recent.len() as f64;
        let error_count = recent
            .iter()
            .filter(|a| {
                let d = &a.detail;
                d.contains("error")
                    || d.contains("Error")
                    || d.contains("panic")
                    || d.contains("fail")
                    || d.contains("timeout")
            })
            .count() as f64;

        let error_rate = if total > 0.0 {
            (error_count / total) * 100.0
        } else {
            0.0
        };

        if error_rate > 20.0 {
            alerts.push(AnomalyAlert {
                id: uuid_v4(),
                alert_type: "error_spike".into(),
                severity: "critical".into(),
                title: "错误率异常飙升".into(),
                message: format!(
                    "检测到错误率 {:.1}%（{}/{} 条事件），建议检查日志排查问题。",
                    error_rate,
                    error_count as u32,
                    total as u32
                ),
                resource_snapshot: None,
                suggested_action: Some("check_logs".into()),
                triggered_by: "error_rate > 20%".into(),
                timestamp: now_iso(),
            });
        } else if error_rate > 10.0 {
            alerts.push(AnomalyAlert {
                id: uuid_v4(),
                alert_type: "error_spike".into(),
                severity: "warning".into(),
                title: "错误率偏高".into(),
                message: format!(
                    "错误率 {:.1}%，建议关注运行状态。",
                    error_rate
                ),
                resource_snapshot: None,
                suggested_action: Some("review_errors".into()),
                triggered_by: "error_rate > 10%".into(),
                timestamp: now_iso(),
            });
        }

        let idle_count = recent
            .iter()
            .filter(|a| a.action == "idle" || a.action == "inactive")
            .count();

        if recent.len() > 100 && idle_count as f64 / recent.len() as f64 > 0.9 {
            alerts.push(AnomalyAlert {
                id: uuid_v4(),
                alert_type: "idle".into(),
                severity: "info".into(),
                title: "长时间无操作".into(),
                message: "检测到应用长时间处于空闲状态，是否进入省电模式？".into(),
                resource_snapshot: None,
                suggested_action: Some("enter_idle_mode".into()),
                triggered_by: "idle_ratio > 90%".into(),
                timestamp: now_iso(),
            });
        }

        alerts.sort_by(|a, b| {
            let severity_order = |s: &str| match s {
                "critical" => 0,
                "warning" => 1,
                "info" => 2,
                _ => 3,
            };
            severity_order(&a.severity).cmp(&severity_order(&b.severity))
        });

        let overall_status = if alerts.iter().any(|a| a.severity == "critical") {
            "critical"
        } else if alerts.iter().any(|a| a.severity == "warning") {
            "warning"
        } else if !alerts.is_empty() {
            "info"
        } else {
            "healthy"
        }
        .to_string();

        AnomalyDetectionResult {
            alerts,
            resource_snapshot: resource,
            error_rate: (error_rate * 10.0).round() / 10.0,
            overall_status,
            detected_at: now_iso(),
        }
    }
}

pub struct DataIntelligence;

impl DataIntelligence {
    pub fn organize(
        entries: &[KnowledgeEntry],
        existing_categories: &[String],
    ) -> KnowledgeOrganizationResult {
        let mut result = KnowledgeOrganizationResult {
            entries_classified: 0,
            categories_created: Vec::new(),
            entries_tagged: 0,
            duplicate_groups: Vec::new(),
            associations_found: Vec::new(),
            unorganized_count: 0,
            organized_at: now_iso(),
        };

        let category_keywords: HashMap<&str, Vec<&str>> = HashMap::from([
            (
                "Rust",
                vec!["rust", "cargo", "tokio", "actix", "tauri", "serde", "async"],
            ),
            (
                "TypeScript",
                vec![
                    "typescript", "tsx", "react", "vue", "next", "node", "bun",
                ],
            ),
            (
                "Python",
                vec!["python", "django", "flask", "fastapi", "pytorch", "tensorflow"],
            ),
            (
                "AI/ML",
                vec!["ai", "llm", "gpt", "transformer", "neural", "deep learning", "openai"],
            ),
            (
                "DevOps",
                vec!["docker", "kubernetes", "ci/cd", "jenkins", "terraform", "ansible"],
            ),
            (
                "数据库",
                vec!["sql", "sqlite", "postgres", "mysql", "mongodb", "redis"],
            ),
            (
                "网络安全",
                vec!["security", "encrypt", "aes", "hash", "auth", "jwt"],
            ),
        ]);

        let mut title_set: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, entry) in entries.iter().enumerate() {
            let key = entry.title.to_lowercase().trim().to_string();
            title_set.entry(key).or_default().push(i);
        }

        for indices in title_set.values() {
            if indices.len() > 1 {
                let group: Vec<String> = indices
                    .iter()
                    .map(|&i| entries[i].title.clone())
                    .collect();
                if !group.is_empty() {
                    result.duplicate_groups.push(group);
                }
            }
        }

        for (i, entry) in entries.iter().enumerate() {
            let text = format!(
                "{} {} {}",
                entry.title, entry.content, entry.tags.join(" ")
            )
            .to_lowercase();

            let mut matched_categories: Vec<(String, u32)> = Vec::new();
            for (cat, keywords) in &category_keywords {
                let score = keywords.iter().filter(|kw| text.contains(*kw)).count() as u32;
                if score > 0 {
                    matched_categories.push((cat.to_string(), score));
                }
            }
            matched_categories.sort_by(|a, b| b.1.cmp(&a.1));

            if !matched_categories.is_empty() {
                let suggested = &matched_categories[0].0;
                if !existing_categories.contains(suggested)
                    && !result.categories_created.contains(suggested)
                {
                    result.categories_created.push(suggested.clone());
                }
                result.entries_classified += 1;
            }

            if entry.tags.is_empty() {
                for (_cat, _) in &matched_categories {
                    result.entries_tagged += 1;
                    break;
                }
            }

            for (j, other) in entries.iter().enumerate() {
                if j <= i {
                    continue;
                }

                let similarity = Self::text_similarity(
                    &entry.title.to_lowercase(),
                    &other.title.to_lowercase(),
                );

                if similarity > 0.6 {
                    result.associations_found.push(KnowledgeAssociation {
                        source_title: entry.title.clone(),
                        target_title: other.title.clone(),
                        association_type: if similarity > 0.85 {
                            "strong_related"
                        } else {
                            "related"
                        }
                        .into(),
                        confidence: (similarity * 100.0).round() / 100.0,
                        reason: format!(
                            "标题相似度 {:.0}%",
                            similarity * 100.0
                        ),
                    });
                }

                let shared_tags: Vec<&str> = entry
                    .tags
                    .iter()
                    .filter(|t| other.tags.contains(t))
                    .map(|s| s.as_str())
                    .collect();
                if shared_tags.len() >= 2 {
                    let already_has = result
                        .associations_found
                        .iter()
                        .any(|a| {
                            (a.source_title == entry.title
                                && a.target_title == other.title)
                                || (a.source_title == other.title
                                    && a.target_title == entry.title)
                        });
                    if !already_has {
                        result.associations_found.push(KnowledgeAssociation {
                            source_title: entry.title.clone(),
                            target_title: other.title.clone(),
                            association_type: "shared_tags".into(),
                            confidence: (shared_tags.len() as f64 * 0.4).min(0.95),
                            reason: format!("共享 {} 个标签: {}", shared_tags.len(), shared_tags.join(", ")),
                        });
                    }
                }
            }
        }

        result.associations_found.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        result.associations_found.truncate(50);

        result.unorganized_count = entries
            .iter()
            .filter(|e| e.tags.is_empty())
            .count() as u32;

        result
    }

    fn text_similarity(a: &str, b: &str) -> f64 {
        let a_words: Vec<&str> = a.split_whitespace().collect();
        let b_words: Vec<&str> = b.split_whitespace().collect();

        if a_words.is_empty() || b_words.is_empty() {
            return 0.0;
        }

        let mut matches = 0;
        for aw in &a_words {
            for bw in &b_words {
                if aw == bw || (aw.len() >= 3 && bw.len() >= 3 && (aw.contains(bw) || bw.contains(aw)))
                {
                    matches += 1;
                    break;
                }
            }
        }
        let similarity = matches as f64 / a_words.len().max(b_words.len()) as f64;

        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let max_len = a_chars.len().max(b_chars.len());
        if max_len == 0 {
            return similarity;
        }
        let mut char_matches = 0;
        for (ac, bc) in a_chars.iter().zip(b_chars.iter()) {
            if ac == bc {
                char_matches += 1;
            }
        }
        let char_sim = char_matches as f64 / max_len as f64;

        similarity * 0.6 + char_sim * 0.4
    }
}

pub struct AutonomousDecisionEngine {
    tasks: HashMap<String, ScheduledTask>,
}

impl AutonomousDecisionEngine {
    pub fn new() -> Self {
        let mut tasks: HashMap<String, ScheduledTask> = HashMap::new();
        let scheduled = vec![
            ScheduledTask {
                id: "recycle_cleanup".into(),
                task_type: "cleanup".into(),
                name: "回收站自动清理".into(),
                cron_expression: None,
                interval_secs: Some(86400),
                enabled: true,
                last_run_at: None,
                next_run_at: None,
                status: "pending".into(),
            },
            ScheduledTask {
                id: "data_archival".into(),
                task_type: "archival".into(),
                name: "过期数据归档".into(),
                cron_expression: None,
                interval_secs: Some(604800),
                enabled: true,
                last_run_at: None,
                next_run_at: None,
                status: "pending".into(),
            },
            ScheduledTask {
                id: "health_check".into(),
                task_type: "monitoring".into(),
                name: "系统健康检查".into(),
                cron_expression: None,
                interval_secs: Some(3600),
                enabled: true,
                last_run_at: None,
                next_run_at: None,
                status: "pending".into(),
            },
            ScheduledTask {
                id: "session_cleanup".into(),
                task_type: "cleanup".into(),
                name: "过期会话清理".into(),
                cron_expression: None,
                interval_secs: Some(43200),
                enabled: true,
                last_run_at: None,
                next_run_at: None,
                status: "pending".into(),
            },
            ScheduledTask {
                id: "log_rotation".into(),
                task_type: "maintenance".into(),
                name: "日志轮转".into(),
                cron_expression: None,
                interval_secs: Some(86400),
                enabled: true,
                last_run_at: None,
                next_run_at: None,
                status: "pending".into(),
            },
        ];
        for t in scheduled {
            tasks.insert(t.id.clone(), t);
        }
        Self { tasks }
    }

    pub fn list_tasks(&self) -> Vec<ScheduledTask> {
        self.tasks.values().cloned().collect()
    }

    pub fn update_task(&mut self, task: ScheduledTask) {
        self.tasks.insert(task.id.clone(), task);
    }

    pub fn toggle_task(&mut self, task_id: &str, enabled: bool) {
        if let Some(t) = self.tasks.get_mut(task_id) {
            t.enabled = enabled;
            t.status = if enabled { "pending".into() } else { "disabled".into() };
        }
    }

    pub fn execute_due_tasks(
        &mut self,
        data_dir: &Path,
    ) -> AutonomousDecisionResult {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut results = Vec::new();
        let mut pending = Vec::new();

        let task_ids: Vec<String> = self.tasks.keys().cloned().collect();
        for task_id in &task_ids {
            let should_run = {
                let task = self.tasks.get(task_id).expect("task should exist in map");
                if !task.enabled {
                    continue;
                }
                match &task.last_run_at {
                    None => true,
                    Some(last) => {
                        let last_ts = parse_timestamp(last) as u64;
                        let interval = task.interval_secs.unwrap_or(3600);
                        now.saturating_sub(last_ts) >= interval
                    }
                }
            };

            if should_run {
                let result = match task_id.as_str() {
                    "recycle_cleanup" => Self::run_recycle_cleanup(data_dir),
                    "data_archival" => Self::run_data_archival(data_dir),
                    "health_check" => Self::run_health_check(),
                    "session_cleanup" => Self::run_session_cleanup(data_dir),
                    "log_rotation" => Self::run_log_rotation(data_dir),
                    _ => TaskExecutionResult {
                        task_id: task_id.clone(),
                        task_name: "未知任务".into(),
                        success: false,
                        items_processed: 0,
                        message: "未识别的任务类型".into(),
                        executed_at: now_iso(),
                    },
                };

                if let Some(task) = self.tasks.get_mut(task_id) {
                    task.last_run_at = Some(now_iso());
                    task.status = if result.success {
                        "completed".into()
                    } else {
                        "failed".into()
                    };
                    if let Some(interval) = task.interval_secs {
                        let next = now + interval;
                        task.next_run_at = Some(
                            chrono::DateTime::from_timestamp(next as i64, 0)
                                .unwrap_or_default()
                                .to_rfc3339(),
                        );
                    }
                }

                results.push(result);
            } else {
                if let Some(task) = self.tasks.get(task_id) {
                    pending.push(format!("{}: {}", task.name, task.status));
                }
            }
        }

        AutonomousDecisionResult {
            task_results: results,
            scheduled_tasks: self.list_tasks(),
            pending_actions: pending,
            generated_at: now_iso(),
        }
    }

    fn run_recycle_cleanup(data_dir: &Path) -> TaskExecutionResult {
        let trash_dir = data_dir.join("trash");
        if !trash_dir.exists() {
            return TaskExecutionResult {
                task_id: "recycle_cleanup".into(),
                task_name: "回收站自动清理".into(),
                success: true,
                items_processed: 0,
                message: "回收站目录不存在，无需清理".into(),
                executed_at: now_iso(),
            };
        }

        let now = SystemTime::now();
        let retention = Duration::from_secs(86400 * 10);
        let mut cleaned = 0u32;

        match fs::read_dir(&trash_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(elapsed) = now.duration_since(modified) {
                                if elapsed > retention {
                                    let path = entry.path();
                                    let remove_result = if path.is_dir() {
                                        fs::remove_dir_all(&path)
                                    } else {
                                        fs::remove_file(&path)
                                    };
                                    if remove_result.is_ok() {
                                        cleaned += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                return TaskExecutionResult {
                    task_id: "recycle_cleanup".into(),
                    task_name: "回收站自动清理".into(),
                    success: false,
                    items_processed: 0,
                    message: format!("读取回收站目录失败: {}", e),
                    executed_at: now_iso(),
                };
            }
        }

        TaskExecutionResult {
            task_id: "recycle_cleanup".into(),
            task_name: "回收站自动清理".into(),
            success: true,
            items_processed: cleaned,
            message: format!("已清理 {} 个过期回收站项目（保留期10天）", cleaned),
            executed_at: now_iso(),
        }
    }

    fn run_data_archival(data_dir: &Path) -> TaskExecutionResult {
        let archive_dir = data_dir.join("archive");
        fs::create_dir_all(&archive_dir).ok();

        let log_dir = data_dir.join("logs");
        let mut archived = 0u32;

        if log_dir.exists() {
            let threshold = SystemTime::now()
                .checked_sub(Duration::from_secs(86400 * 30))
                .unwrap_or(SystemTime::UNIX_EPOCH);

            if let Ok(entries) = fs::read_dir(&log_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "log" || ext == "old") {
                        if let Ok(meta) = entry.metadata() {
                            if let Ok(modified) = meta.modified() {
                                if modified < threshold {
                                    let file_name = path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("unknown");
                                    let dest = archive_dir.join(format!(
                                        "{}_{}",
                                        chrono::Utc::now().format("%Y%m%d"),
                                        file_name
                                    ));
                                    if fs::rename(&path, &dest).is_ok() {
                                        archived += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        TaskExecutionResult {
            task_id: "data_archival".into(),
            task_name: "过期数据归档".into(),
            success: true,
            items_processed: archived,
            message: format!("已归档 {} 个过期文件", archived),
            executed_at: now_iso(),
        }
    }

    fn run_health_check() -> TaskExecutionResult {
        let resource = AnomalyDetector::snapshot_resources();
        let mut issues: Vec<String> = Vec::new();

        if resource.cpu_usage_percent > 80.0 {
            issues.push(format!("CPU 使用率 {:.0}%", resource.cpu_usage_percent));
        }
        if resource.memory_usage_percent > 80.0 {
            issues.push(format!(
                "内存使用率 {:.0}%",
                resource.memory_usage_percent
            ));
        }
        if resource.disk_usage_percent > 85.0 {
            issues.push(format!(
                "磁盘使用率 {:.0}%",
                resource.disk_usage_percent
            ));
        }

        TaskExecutionResult {
            task_id: "health_check".into(),
            task_name: "系统健康检查".into(),
            success: true,
            items_processed: issues.len() as u32,
            message: if issues.is_empty() {
                "系统运行状态正常".into()
            } else {
                format!("发现 {} 个潜在问题: {}", issues.len(), issues.join("; "))
            },
            executed_at: now_iso(),
        }
    }

    fn run_session_cleanup(data_dir: &Path) -> TaskExecutionResult {
        let sessions_dir = data_dir.join("sessions");
        if !sessions_dir.exists() {
            return TaskExecutionResult {
                task_id: "session_cleanup".into(),
                task_name: "过期会话清理".into(),
                success: true,
                items_processed: 0,
                message: "会话目录不存在".into(),
                executed_at: now_iso(),
            };
        }

        let threshold = SystemTime::now()
            .checked_sub(Duration::from_secs(86400 * 7))
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let mut cleaned = 0u32;

        if let Ok(entries) = fs::read_dir(&sessions_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if modified < threshold {
                            let path = entry.path();
                            let remove_result = if path.is_dir() {
                                fs::remove_dir_all(&path)
                            } else {
                                fs::remove_file(&path)
                            };
                            if remove_result.is_ok() {
                                cleaned += 1;
                            }
                        }
                    }
                }
            }
        }

        TaskExecutionResult {
            task_id: "session_cleanup".into(),
            task_name: "过期会话清理".into(),
            success: true,
            items_processed: cleaned,
            message: format!("已清理 {} 个过期会话（保留期7天）", cleaned),
            executed_at: now_iso(),
        }
    }

    fn run_log_rotation(data_dir: &Path) -> TaskExecutionResult {
        let log_dir = data_dir.join("logs");
        fs::create_dir_all(&log_dir).ok();

        let max_logs = 10;
        let max_size_bytes = 10 * 1024 * 1024;
        let mut rotated = 0u32;

        if let Ok(entries) = fs::read_dir(&log_dir) {
            let mut log_files: Vec<_> = entries
                .flatten()
                .filter(|e| {
                    e.path()
                        .extension()
                        .map_or(false, |ext| ext == "log")
                })
                .collect();
            log_files.sort_by_key(|e| {
                e.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH)
            });

            if log_files.len() > max_logs {
                for old in log_files.iter().take(log_files.len() - max_logs) {
                    if fs::remove_file(old.path()).is_ok() {
                        rotated += 1;
                    }
                }
            }

            for f in &log_files {
                if let Ok(meta) = f.metadata() {
                    if meta.len() > max_size_bytes {
                        let path = f.path();
                        let rotated_path = path.with_extension(format!(
                            "{}.old",
                            chrono::Utc::now().format("%Y%m%d")
                        ));
                        if fs::rename(&path, &rotated_path).is_ok() {
                            rotated += 1;
                        }
                    }
                }
            }
        }

        TaskExecutionResult {
            task_id: "log_rotation".into(),
            task_name: "日志轮转".into(),
            success: true,
            items_processed: rotated,
            message: format!("已轮转 {} 个日志文件", rotated),
            executed_at: now_iso(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub created_at: Option<String>,
}