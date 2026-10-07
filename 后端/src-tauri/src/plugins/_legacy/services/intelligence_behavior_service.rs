use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use chrono::Timelike;

use crate::error::app_error::AppError;
use crate::models::intelligence::{
    BehaviorAnalysisResult, BehaviorPattern, CognitiveLoadSnapshot, ProjectTechStack,
    SmartNotification, UsageDashboard, UserActivity, WorkflowRecommendation,
};

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub struct BehaviorAnalyzer;

impl BehaviorAnalyzer {
    pub fn analyze(activities: &[UserActivity]) -> Result<BehaviorAnalysisResult, AppError> {
        if activities.is_empty() {
            return Ok(BehaviorAnalysisResult {
                patterns: vec![],
                total_activities_analyzed: 0,
                dominant_category: "none".into(),
                analyzed_at: now_iso(),
            });
        }

        let mut sequences: Vec<Vec<&UserActivity>> = vec![];
        let mut current: Vec<&UserActivity> = vec![];
        let mut last_ts = 0i64;

        for a in activities {
            let ts = parse_timestamp(&a.timestamp);
            if !current.is_empty() && ts - last_ts > 120 {
                if current.len() >= 2 {
                    sequences.push(current.clone());
                }
                current.clear();
            }
            current.push(a);
            last_ts = ts;
        }
        if current.len() >= 2 {
            sequences.push(current);
        }

        let mut pattern_freq: HashMap<String, (usize, String, f64)> = HashMap::new();

        for seq in &sequences {
            let actions: Vec<String> = seq.iter().map(|a: &&UserActivity| a.action.clone()).collect();
            let key = actions.join(" → ");

            let avg_dur = if seq.len() > 1 {
                let first_ts = parse_timestamp(&seq[0].timestamp);
                let last_ts = parse_timestamp(&seq[seq.len() - 1].timestamp);
                ((last_ts - first_ts) as f64).max(0.0)
            } else {
                0.0
            };

            let category = categorize_sequence(&actions);
            let entry = pattern_freq.entry(key).or_insert((0, category, 0.0));
            entry.0 += 1;
            entry.2 = (entry.2 + avg_dur) / 2.0;
        }

        let mut patterns: Vec<BehaviorPattern> = pattern_freq
            .into_iter()
            .map(|(name, (freq, cat, dur))| BehaviorPattern {
                pattern_name: name,
                activities: vec![],
                frequency: freq,
                avg_duration_secs: dur,
                category: cat.clone(),
                last_observed_at: now_iso(),
            })
            .collect();
        patterns.sort_by(|a, b| b.frequency.cmp(&a.frequency));
        patterns.truncate(20);

        let dominant_category = patterns
            .first()
            .map(|p| p.category.clone())
            .unwrap_or_else(|| "general".into());

        Ok(BehaviorAnalysisResult {
            patterns,
            total_activities_analyzed: activities.len(),
            dominant_category,
            analyzed_at: now_iso(),
        })
    }
}

fn parse_timestamp(ts: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.timestamp())
        .unwrap_or(0)
}

fn categorize_sequence(actions: &[String]) -> String {
    let joined = actions.join(" ");
    if joined.contains("edit") || joined.contains("open_file") {
        "coding".into()
    } else if joined.contains("terminal") || joined.contains("run_command") {
        "cli".into()
    } else if joined.contains("search") || joined.contains("browse") {
        "exploration".into()
    } else if joined.contains("debug") || joined.contains("test") {
        "debugging".into()
    } else {
        "general".into()
    }
}

pub struct WorkflowRecommender;

impl WorkflowRecommender {
    pub fn recommend(
        activities: &[UserActivity],
        current_context: &str,
    ) -> Vec<WorkflowRecommendation> {
        let mut recommendations = Vec::new();

        let recent_actions: Vec<&str> = activities
            .iter()
            .rev()
            .take(10)
            .map(|a| a.action.as_str())
            .collect();

        if recent_actions.iter().any(|a: &&str| a.contains("edit")) {
            recommendations.push(WorkflowRecommendation {
                id: uuid_v4(),
                title: "代码编辑 → 测试验证流程".into(),
                description: "检测到编辑活动，建议运行相关测试验证更改".into(),
                steps: vec![
                    "保存当前文件".into(),
                    "运行单元测试".into(),
                    "检查lint结果".into(),
                    "提交代码".into(),
                ],
                relevance_score: 0.85,
                category: "coding".into(),
                based_on_patterns: vec!["edit → test".into()],
            });
        }

        if recent_actions.iter().any(|a: &&str| a.contains("terminal")) {
            recommendations.push(WorkflowRecommendation {
                id: uuid_v4(),
                title: "终端操作 → 日志检查流程".into(),
                description: "检测到终端活动，建议检查运行日志确保无异常".into(),
                steps: vec![
                    "检查dmesg/日志输出".into(),
                    "确认进程状态".into(),
                    "清理临时文件".into(),
                ],
                relevance_score: 0.72,
                category: "cli".into(),
                based_on_patterns: vec!["terminal → check".into()],
            });
        }

        if current_context.contains("debug") || recent_actions.iter().any(|a: &&str| a.contains("error")) {
            recommendations.push(WorkflowRecommendation {
                id: uuid_v4(),
                title: "调试流程最佳实践".into(),
                description: "检测到调试场景，推荐结构化调试流程".into(),
                steps: vec![
                    "复现问题步骤".into(),
                    "检查相关日志".into(),
                    "添加断点/打印".into(),
                    "逐步缩小范围".into(),
                    "修复并验证".into(),
                ],
                relevance_score: 0.91,
                category: "debugging".into(),
                based_on_patterns: vec!["error → debug".into()],
            });
        }

        if recommendations.is_empty() {
            recommendations.push(WorkflowRecommendation {
                id: uuid_v4(),
                title: "开始新任务建议".into(),
                description: "基于当前上下文的工作建议".into(),
                steps: vec![
                    "明确任务目标".into(),
                    "搜索相关文档".into(),
                    "创建TODO列表".into(),
                ],
                relevance_score: 0.5,
                category: "general".into(),
                based_on_patterns: vec!["start → plan".into()],
            });
        }

        recommendations.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));
        recommendations
    }
}

fn uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub struct TechStackDetector;

impl TechStackDetector {
    pub fn detect(project_dir: &str) -> Result<ProjectTechStack, AppError> {
        let dir = Path::new(project_dir);
        if !dir.exists() || !dir.is_dir() {
            return Err(AppError::Validation(format!("项目目录不存在: {}", project_dir)));
        }

        let mut detected_files = Vec::new();
        let mut project_type = "unknown".to_string();
        let mut primary_language = "unknown".to_string();
        let mut frameworks = Vec::new();
        let mut build_tools = Vec::new();
        let mut package_manager = "unknown".to_string();
        let mut confidence = 0.0;

        let files = Self::list_top_level(dir);
        for f in &files {
            detected_files.push(f.clone());
        }

        if files.iter().any(|f| f == "Cargo.toml") {
            project_type = "rust".into();
            primary_language = "Rust".into();
            build_tools.push("cargo".into());
            package_manager = "cargo".into();
            confidence = 0.95;
            if let Ok(content) = fs::read_to_string(dir.join("Cargo.toml")) {
                if content.contains("tauri") { frameworks.push("Tauri".into()); }
                if content.contains("tokio") { frameworks.push("Tokio".into()); }
                if content.contains("actix") { frameworks.push("Actix".into()); }
                if content.contains("rocket") { frameworks.push("Rocket".into()); }
            }
        } else if files.iter().any(|f| f == "package.json") {
            primary_language = "JavaScript/TypeScript".into();
            package_manager = if files.iter().any(|f| f == "yarn.lock") { "yarn".into() } else { "npm".into() };
            confidence = 0.93;
            if let Ok(content) = fs::read_to_string(dir.join("package.json")) {
                if content.contains("\"react\"") { frameworks.push("React".into()); }
                if content.contains("\"vue\"") { frameworks.push("Vue".into()); }
                if content.contains("\"next\"") { frameworks.push("Next.js".into()); }
                if content.contains("\"express\"") { frameworks.push("Express".into()); }
                if content.contains("\"vite\"") { build_tools.push("Vite".into()); }
                if content.contains("\"webpack\"") { build_tools.push("Webpack".into()); }
                if content.contains("\"typescript\"") { primary_language = "TypeScript".into(); }
                if content.contains("\"tauri\"") { frameworks.push("Tauri".into()); }
            }
            project_type = if frameworks.contains(&"React".to_string()) { "react".into() }
                else if frameworks.contains(&"Vue".to_string()) { "vue".into() }
                else { "node".into() };
        } else if files.iter().any(|f| f == "requirements.txt" || f == "pyproject.toml" || f == "setup.py") {
            project_type = "python".into();
            primary_language = "Python".into();
            package_manager = "pip".into();
            confidence = 0.90;
            if let Ok(content) = fs::read_to_string(dir.join("requirements.txt")) {
                if content.contains("django") { frameworks.push("Django".into()); }
                if content.contains("flask") { frameworks.push("Flask".into()); }
                if content.contains("fastapi") { frameworks.push("FastAPI".into()); }
                if content.contains("pytest") { build_tools.push("pytest".into()); }
            }
        } else if files.iter().any(|f| f == "go.mod") {
            project_type = "go".into();
            primary_language = "Go".into();
            package_manager = "go mod".into();
            build_tools.push("go".into());
            confidence = 0.92;
            if let Ok(content) = fs::read_to_string(dir.join("go.mod")) {
                if content.contains("gin-gonic") { frameworks.push("Gin".into()); }
                if content.contains("echo") { frameworks.push("Echo".into()); }
                if content.contains("fiber") { frameworks.push("Fiber".into()); }
            }
        } else if files.iter().any(|f| f == "Makefile") {
            project_type = "c/c++".into();
            primary_language = "C/C++".into();
            build_tools.push("make".into());
            confidence = 0.70;
        } else if files.iter().any(|f| f.ends_with(".csproj")) {
            project_type = "dotnet".into();
            primary_language = "C#".into();
            build_tools.push("dotnet".into());
            package_manager = "NuGet".into();
            confidence = 0.85;
        }

        if let Ok(content) = fs::read_to_string(dir.join(".gitignore")) {
            if content.contains("node_modules") && primary_language == "unknown" {
                primary_language = "JavaScript/TypeScript".into();
                project_type = "node".into();
                confidence = 0.4;
            }
        }

        Ok(ProjectTechStack {
            project_type,
            primary_language,
            frameworks,
            build_tools,
            package_manager,
            detected_files,
            confidence,
        })
    }

    fn list_top_level(dir: &Path) -> Vec<String> {
        let mut result = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    result.push(name);
                }
            }
        }
        result
    }
}

pub struct CognitiveLoadTracker;

impl CognitiveLoadTracker {
    pub fn new() -> Self {
        Self {}
    }

    pub fn assess(activities: &[UserActivity], window_secs: u64) -> CognitiveLoadSnapshot {
        let now_ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let window_start = now_ts.saturating_sub(window_secs);
        let recent: Vec<&UserActivity> = activities
            .iter()
            .filter(|a| {
                let ts = parse_timestamp(&a.timestamp) as u64;
                ts >= window_start
            })
            .collect();

        let activity_count = recent.len() as u32;
        let context_switches = Self::count_switches(&recent);
        let active_duration = if recent.len() >= 2 {
            let first = parse_timestamp(&recent[0].timestamp) as u64;
            let last = parse_timestamp(&recent[recent.len() - 1].timestamp) as u64;
            last.saturating_sub(first)
        } else {
            0
        };

        let switch_factor = (context_switches as f64 / window_secs.max(1) as f64) * 3600.0;
        let activity_factor = (activity_count as f64 / window_secs.max(1) as f64) * 3600.0;
        let load_score = (switch_factor * 0.4 + activity_factor * 0.6).min(100.0);

        let load_level = if load_score > 80.0 {
            "critical"
        } else if load_score > 50.0 {
            "high"
        } else if load_score > 25.0 {
            "moderate"
        } else {
            "low"
        };

        let suggestion = match load_level {
            "critical" => Some("认知负载极高，建议休息5-10分钟恢复注意力".into()),
            "high" => Some("工作强度较高，考虑短暂休息或切换到低强度任务".into()),
            _ => None,
        };

        CognitiveLoadSnapshot {
            load_level: load_level.into(),
            load_score,
            active_duration_secs: active_duration,
            activity_count,
            context_switches,
            timestamp: now_iso(),
            suggestion,
        }
    }

    fn count_switches(activities: &[&UserActivity]) -> u32 {
        if activities.len() < 2 {
            return 0;
        }
        let mut switches = 0u32;
        for i in 1..activities.len() {
            if activities[i].activity_type != activities[i - 1].activity_type {
                switches += 1;
            }
        }
        switches
    }
}

pub struct SmartNotifier;

impl SmartNotifier {
    pub fn generate(activities: &[UserActivity], cognitive: &CognitiveLoadSnapshot) -> Vec<SmartNotification> {
        let mut notifications = Vec::new();

        if cognitive.load_level == "critical" {
            notifications.push(SmartNotification {
                id: uuid_v4(),
                notification_type: "cognitive_alert".into(),
                title: "认知负载过高".into(),
                message: format!("当前认知负载评分{:.1}，处于临界水平。建议休息片刻。", cognitive.load_score),
                priority: "critical".into(),
                action_type: Some("break_reminder".into()),
                action_payload: Some("take_break".into()),
                triggered_by: "cognitive_load > 80".into(),
                timestamp: now_iso(),
            });
        } else if cognitive.load_level == "high" {
            notifications.push(SmartNotification {
                id: uuid_v4(),
                notification_type: "health_reminder".into(),
                title: "工作强度提醒".into(),
                message: "已持续高强度工作一段时间，建议站起来活动或喝水".into(),
                priority: "warning".into(),
                action_type: Some("health_tip".into()),
                action_payload: Some("stretch_and_hydrate".into()),
                triggered_by: "cognitive_load > 50".into(),
                timestamp: now_iso(),
            });
        }

        let edit_count = activities
            .iter()
            .filter(|a| a.action.contains("edit"))
            .count();
        if edit_count > 20 {
            notifications.push(SmartNotification {
                id: uuid_v4(),
                notification_type: "productivity_tip".into(),
                title: "大量编辑未提交".into(),
                message: format!("检测到{}次编辑操作，建议考虑提交当前更改", edit_count),
                priority: "info".into(),
                action_type: Some("suggestion".into()),
                action_payload: Some("commit_changes".into()),
                triggered_by: "edit_count > 20".into(),
                timestamp: now_iso(),
            });
        }

        let error_count = activities
            .iter()
            .filter(|a| a.detail.contains("error") || a.detail.contains("Error"))
            .count();
        if error_count > 5 {
            notifications.push(SmartNotification {
                id: uuid_v4(),
                notification_type: "error_alert".into(),
                title: "多次错误检测".into(),
                message: format!("检测到{}次错误相关事件，建议检查系统状态", error_count),
                priority: "warning".into(),
                action_type: Some("debug_suggestion".into()),
                action_payload: Some("check_logs".into()),
                triggered_by: "error_count > 5".into(),
                timestamp: now_iso(),
            });
        }

        notifications
    }
}

pub struct DashboardBuilder;

impl DashboardBuilder {
    pub fn build(activities: &[UserActivity], period: &str) -> UsageDashboard {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let period_secs = match period {
            "hour" => 3600,
            "day" => 86400,
            "week" => 604800,
            _ => 86400,
        };

        let window_start = now.saturating_sub(period_secs);
        let filtered: Vec<&UserActivity> = activities
            .iter()
            .filter(|a| {
                let ts = parse_timestamp(&a.timestamp) as u64;
                ts >= window_start
            })
            .collect();

        let total_activities = filtered.len();

        let mut type_counts: HashMap<String, usize> = HashMap::new();
        let mut file_counts: HashMap<String, usize> = HashMap::new();
        let mut hour_counts: HashMap<u32, usize> = HashMap::new();

        for a in &filtered {
            *type_counts.entry(a.activity_type.clone()).or_insert(0) += 1;
            if let Some(ref fp) = a.file_path {
                let name = Path::new(fp)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(fp)
                    .to_string();
                *file_counts.entry(name).or_insert(0) += 1;
            }
            let ts = parse_timestamp(&a.timestamp);
            if ts > 0 {
                let dt = chrono::DateTime::from_timestamp(ts, 0)
                    .unwrap_or_default();
                *hour_counts.entry(dt.hour()).or_insert(0) += 1;
            }
        }

        let total_active_hours = filtered.len() as f64 / 60.0;
        let avg_sessions_per_day = if filtered.len() > 10 {
            (filtered.len() as f64 / period_secs.max(1) as f64) * 86400.0 / 10.0
        } else {
            0.0
        };

        let mut top_types: Vec<(String, usize)> = type_counts.into_iter().collect();
        top_types.sort_by(|a, b| b.1.cmp(&a.1));
        top_types.truncate(10);

        let mut top_files: Vec<(String, usize)> = file_counts.into_iter().collect();
        top_files.sort_by(|a, b| b.1.cmp(&a.1));
        top_files.truncate(10);

        let mut peak_hours_vec: Vec<(u32, usize)> = hour_counts.into_iter().collect();
        peak_hours_vec.sort_by(|a, b| b.1.cmp(&a.1));
        let peak_hours: Vec<String> = peak_hours_vec
            .iter()
            .take(5)
            .map(|(h, c)| format!("{:02}:00 ({}次)", h, c))
            .collect();

        let productivity_score = if total_activities > 0 {
            let variety = top_types.len() as f64 / 10.0;
            let consistency = (filtered.len() as f64 / period_secs.max(1) as f64) * 3600.0;
            (variety * 30.0 + consistency * 70.0).min(100.0)
        } else {
            0.0
        };

        UsageDashboard {
            period: period.into(),
            total_active_hours,
            total_activities,
            top_activity_types: top_types,
            top_files,
            avg_sessions_per_day,
            productivity_score,
            peak_hours,
            generated_at: now_iso(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_activity(action: &str, detail: &str, atype: &str) -> UserActivity {
        UserActivity {
            activity_type: atype.into(),
            action: action.into(),
            detail: detail.into(),
            file_path: None,
            timestamp: now_iso(),
        }
    }

    #[test]
    fn test_behavior_analysis_empty() {
        let result = BehaviorAnalyzer::analyze(&[]).unwrap();
        assert_eq!(result.total_activities_analyzed, 0);
        assert!(result.patterns.is_empty());
    }

    #[test]
    fn test_behavior_analysis_with_data() {
        let activities = vec![
            make_activity("open_file", "main.rs", "editor"),
            make_activity("edit", "added function", "editor"),
            make_activity("save", "saved main.rs", "editor"),
            make_activity("run_command", "cargo build", "terminal"),
            make_activity("edit", "fixed bug", "editor"),
            make_activity("save", "saved main.rs", "editor"),
            make_activity("run_command", "cargo test", "terminal"),
        ];
        let result = BehaviorAnalyzer::analyze(&activities).unwrap();
        assert_eq!(result.total_activities_analyzed, 7);
        assert!(!result.patterns.is_empty());
    }

    #[test]
    fn test_tech_stack_detection_not_found() {
        let dir = std::env::temp_dir().join("nexterm_empty_project");
        fs::create_dir_all(&dir).ok();
        let result = TechStackDetector::detect(&dir.to_string_lossy());
        assert!(result.is_ok());
        let stack = result.unwrap();
        assert_eq!(stack.primary_language, "unknown");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_tech_stack_rust_detection() {
        let dir = std::env::temp_dir().join("nexterm_rust_test");
        fs::create_dir_all(&dir).ok();
        fs::write(dir.join("Cargo.toml"), "[package]\nname = \"test\"\n[dependencies]\ntokio = \"1\"\n").ok();
        let result = TechStackDetector::detect(&dir.to_string_lossy());
        assert!(result.is_ok());
        let stack = result.unwrap();
        assert_eq!(stack.primary_language, "Rust");
        assert!(stack.frameworks.contains(&"Tokio".to_string()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_cognitive_load_assessment() {
        let mut activities = Vec::new();
        for i in 0..30 {
            activities.push(UserActivity {
                activity_type: if i % 3 == 0 { "editor" } else { "terminal" }.into(),
                action: if i % 3 == 0 { "edit" } else { "run_command" }.into(),
                detail: format!("action {}", i),
                file_path: None,
                timestamp: now_iso(),
            });
        }
        let snapshot = CognitiveLoadTracker::assess(&activities, 3600);
        assert!(!snapshot.load_level.is_empty());
        assert!(snapshot.load_score >= 0.0);
    }

    #[test]
    fn test_dashboard_build() {
        let activities: Vec<UserActivity> = (0..50)
            .map(|i| UserActivity {
                activity_type: if i % 2 == 0 { "editor" } else { "terminal" }.into(),
                action: if i % 2 == 0 { "edit" } else { "run_command" }.into(),
                detail: format!("task {}", i),
                file_path: if i % 5 == 0 { Some(format!("/path/to/file{}.rs", i)) } else { None },
                timestamp: now_iso(),
            })
            .collect();
        let dashboard = DashboardBuilder::build(&activities, "day");
        assert_eq!(dashboard.period, "day");
        assert!(dashboard.total_activities > 0);
    }
}