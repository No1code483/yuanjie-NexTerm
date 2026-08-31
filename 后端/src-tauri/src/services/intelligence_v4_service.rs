use sqlx::SqlitePool;

use crate::db::repositories::activity_log_repo;
use crate::db::repositories::behavior_analysis_repo;
use crate::db::repositories::intelligence_settings_repo;
use crate::db::repositories::suggestion_repo;
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;
use crate::models::intelligence_cross_module::{
    ActivityLog, ActivityLogExport, ActivityLogExportFilter, ActivityLogPage, ActivityLogRecord,
    ActivityStats, BehaviorAnalysis, BehaviorComparison, BehaviorPattern, BehaviorTrend,
    DashboardData, IntelligenceSettings, OperationTemplate, TopFile,
    ProductivityBreakdown, RealtimeStats, Suggestion, SuggestionFeedback, SuggestionPage,
    TestConnectionResult,
};

pub async fn log_activity(
    pool: &SqlitePool,
    record: ActivityLogRecord,
) -> Result<ApiResponse<ActivityLog>, AppError> {
    let log = activity_log_repo::insert_log(
        pool,
        &record.user_id,
        &record.timestamp,
        &record.module,
        &record.operation,
        record.detail.as_deref(),
        record.remark.as_deref(),
        record.duration_secs.unwrap_or(0),
    )
    .await?;

    Ok(ApiResponse::success(log))
}

pub async fn instrument(
    pool: &SqlitePool,
    user_id: &str,
    module: &str,
    operation: &str,
    remark: Option<&str>,
) {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let _ = activity_log_repo::insert_log(
        pool,
        user_id,
        &now,
        module,
        operation,
        None,
        remark,
        0,
    )
    .await;
}

pub async fn instrument_cmd(
    state: &crate::db::connection::AppState,
    module: &str,
    operation: &str,
    remark: Option<&str>,
) {
    let uid = state
        .current_user
        .read()
        .await
        .map(|id| id.to_string())
        .unwrap_or_else(|| "anonymous".to_string());
    instrument(&state.pool, &uid, module, operation, remark).await;
}

pub async fn batch_log_activity(
    pool: &SqlitePool,
    records: Vec<ActivityLogRecord>,
) -> Result<ApiResponse<()>, AppError> {
    let tuples: Vec<_> = records
        .iter()
        .map(|r| {
            (
                r.user_id.clone(),
                r.timestamp.clone(),
                r.module.clone(),
                r.operation.clone(),
                r.detail.clone(),
                r.remark.clone(),
                r.duration_secs.unwrap_or(0),
            )
        })
        .collect();

    activity_log_repo::batch_insert_logs(pool, &tuples).await?;

    Ok(ApiResponse::success(()))
}

pub async fn query_activity_logs(
    pool: &SqlitePool,
    user_id: Option<String>,
    module: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<ApiResponse<ActivityLogPage>, AppError> {
    let limit = limit.unwrap_or(50).min(500);
    let offset = offset.unwrap_or(0);

    let (logs, total) = activity_log_repo::query_logs(
        pool,
        user_id.as_deref(),
        module.as_deref(),
        start_time.as_deref(),
        end_time.as_deref(),
        limit,
        offset,
    )
    .await?;

    let page = ActivityLogPage {
        logs,
        total,
        page: offset / limit + 1,
        page_size: limit,
    };

    Ok(ApiResponse::success(page))
}

pub async fn get_activity_stats(
    pool: &SqlitePool,
    user_id: String,
    period: String,
) -> Result<ApiResponse<ActivityStats>, AppError> {
    let stats = activity_log_repo::get_stats(pool, &user_id, &period).await?;
    Ok(ApiResponse::success(stats))
}

pub async fn clean_activity_logs(
    pool: &SqlitePool,
    retention_days: i64,
) -> Result<ApiResponse<u64>, AppError> {
    let deleted = activity_log_repo::cleanup_old_logs(pool, retention_days).await?;
    Ok(ApiResponse::success(deleted))
}

/// 安全审计修复（IDOR）：添加 user_id 参数，仅删除当前用户的活动日志
pub async fn clear_all_activity_logs(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<ApiResponse<u64>, AppError> {
    let deleted = activity_log_repo::clear_all_logs(pool, user_id).await?;
    Ok(ApiResponse::success(deleted))
}

fn resolve_date_filter(period: &str) -> &'static str {
    // 前端用 .toISOString() 存储的是 UTC 时间戳
    // 统一用 datetime(timestamp, 'localtime') 转为本地时间后再比对边界
    // 不设上界：避免 <= now 截断最新插入的记录（timestamp ≈ now 时差 <1s 被误杀）
    match period {
        "all" | "total" => "1=1",
        "daily" | "today" | "day" =>
            "datetime(timestamp, 'localtime') >= datetime('now', 'localtime', 'start of day')",
        "yesterday" =>
            "datetime(timestamp, 'localtime') >= datetime('now', 'localtime', '-1 day', 'start of day') \
             AND datetime(timestamp, 'localtime') < datetime('now', 'localtime', 'start of day')",
        "weekly" | "week" => "datetime(timestamp, 'localtime') >= datetime('now', 'localtime', '-7 days', 'start of day')",
        "monthly" | "month" => "datetime(timestamp, 'localtime') >= datetime('now', 'localtime', '-30 days', 'start of day')",
        _ => "datetime(timestamp, 'localtime') >= datetime('now', 'localtime', '-7 days', 'start of day')",
    }
}


pub async fn get_dashboard(
    pool: &SqlitePool,
    user_id: String,
    period: String,
) -> Result<ApiResponse<DashboardData>, AppError> {
    let date_filter = resolve_date_filter(&period);
    tracing::info!("[仪表盘] user_id={}, period={}, date_filter={}", user_id, period, date_filter);

    // ========== 基础数据查询 ==========
    let total_active_secs =
        activity_log_repo::get_dashboard_active_duration(pool, &user_id, date_filter).await?;
    let total_operations: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM activity_logs WHERE user_id = ? AND {}",
        date_filter
    ))
    .bind(&user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let hourly_heatmap = activity_log_repo::get_dashboard_hourly_heatmap(pool, &user_id, date_filter).await?;
    let daily_trend = activity_log_repo::get_dashboard_daily_trend(pool, &user_id, date_filter).await?;

    // ========== 高频模块（真实数据）==========
    let top_modules: Vec<(String, i64)> = sqlx::query_as::<_, (String, i64)>(&format!(
        "SELECT module, COUNT(*) as cnt FROM activity_logs WHERE user_id = ? AND {}
         GROUP BY module ORDER BY cnt DESC LIMIT 10",
        date_filter
    ))
    .bind(&user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // ========== 活跃时段：取 Top 3 高峰（SQL已转本地时区）==========
    let mut sorted_hours: Vec<&_> = hourly_heatmap.iter().filter(|h| h.count > 0).collect();
    sorted_hours.sort_by(|a, b| b.count.cmp(&a.count));
    let peak_hours: Vec<String> = sorted_hours.iter().take(3).map(|h| {
        let h_idx = h.hour[..2].parse::<usize>().unwrap_or(0);
        format!("{:02}:00-{:02}:00", h_idx, (h_idx + 1) % 24)
    }).collect();

    // ========== 高频文件：知识库中被实际操作过的文件（从活动日志统计） ==========
    // 仅统计指定时间段内知识库模块的活动记录，无操作则不显示
    let kb_activities = sqlx::query_as::<_, (String, String, i64)>(
        &format!(
            "SELECT detail, operation, COUNT(*) as cnt
             FROM activity_logs
             WHERE user_id = ? AND module IN ('knowledge', 'knowledge_base') AND {}
               AND operation IN ('打开文件','查看条目','打开链接')
             GROUP BY detail
             ORDER BY cnt DESC LIMIT 20",
            date_filter
        )
    )
    .bind(&user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 从 detail 解析结构化数据：name:xxx,path_url:xxx,source_path:xxx
    let mut top_files: Vec<TopFile> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (detail, _operation, count) in kb_activities {
        if top_files.len() >= 10 { break; }

        // 解析 name: / path_url: / source_path:
        let file_name = detail.split(',')
            .find(|s| s.trim().starts_with("name:"))
            .and_then(|s| s.trim().strip_prefix("name:").map(|n| n.trim().to_string()))
            .unwrap_or_else(|| {
                // 兼容旧格式：直接用 detail 作为名称
                detail.rsplit(['/', '\\']).next().unwrap_or(&detail).to_string()
            });

        if seen.contains(&file_name) { continue; }
        seen.insert(file_name.clone());

        let kb_path = detail.split(',')
            .find(|s| s.trim().starts_with("path_url:"))
            .and_then(|s| s.trim().strip_prefix("path_url:").map(|p| p.trim().to_string()))
            .unwrap_or_default();

        let local_path = detail.split(',')
            .find(|s| s.trim().starts_with("source_path:"))
            .and_then(|s| s.trim().strip_prefix("source_path:").map(|p| p.trim().to_string()))
            .unwrap_or_default();

        top_files.push(TopFile { file_name, kb_path, local_path, access_count: count });
    }

    // ========== 生产力评分：纯数据驱动（不调用AI，秒级响应）==========
    let period_labels: Vec<String> = daily_trend.iter().map(|d| d.date.clone()).collect();
    let active_days = daily_trend.len() as f64;
    let module_diversity = top_modules.len() as f64;

    let (productivity_score, productivity_breakdown) = if total_operations == 0 && total_active_secs == 0 {
        (0.0, ProductivityBreakdown {
            focus_ratio: 0.0, todo_completion_ratio: 0.0, knowledge_regularity: 0.0,
            focus_weight: 0.4, todo_weight: 0.35, knowledge_weight: 0.25, raw_score: 0.0,
        })
    } else {
        // 数据驱动评分公式（无需网络请求）
        let period_days: f64 = match period.as_str() { "day" => 1.0, "week" => 7.0, _ => 30.0 };
        let consistency = (active_days / period_days).min(1.0);
        let diversity_score = (module_diversity / 6.0).min(1.0) * 35.0;
        let intensity = ((total_operations as f64 / (active_days.max(1.0) * 15.0)).min(1.0)) * 35.0;
        let engagement = consistency * 30.0;
        let base = diversity_score + intensity + engagement;
        (base.round(), ProductivityBreakdown {
            focus_ratio: intensity.round(),
            todo_completion_ratio: (consistency * 100.0).round(),
            knowledge_regularity: diversity_score.round(),
            focus_weight: 0.35, todo_weight: 0.30, knowledge_weight: 0.35,
            raw_score: base.round(),
        })
    };

    let total_active_hours = total_active_secs as f64 / 3600.0;

    Ok(ApiResponse::success(DashboardData {
        period,
        total_active_secs,
        total_active_hours: (total_active_hours * 10.0).round() / 10.0,
        total_operations,
        top_files,
        top_modules,
        hourly_heatmap,
        period_labels,
        productivity_score,
        productivity_breakdown,
        peak_hours,
        daily_trend,
        generated_at: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
    }))
}

pub async fn get_realtime_stats(
    pool: &SqlitePool,
    user_id: String,
) -> Result<ApiResponse<RealtimeStats>, AppError> {
    tracing::info!("[实时统计] user_id={}", user_id);
    let stats = activity_log_repo::get_realtime_stats_for_today(pool, &user_id).await?;
    Ok(ApiResponse::success(stats))
}

pub async fn generate_suggestions(
    pool: &SqlitePool,
    user_id: String,
) -> Result<ApiResponse<Vec<Suggestion>>, AppError> {
    let mut suggestions = Vec::new();

    detect_todo_neglect(pool, &user_id, &mut suggestions).await?;
    detect_late_night_usage(pool, &user_id, &mut suggestions).await?;
    detect_knowledge_fragmentation(pool, &user_id, &mut suggestions).await?;
    detect_feature_neglect(pool, &user_id, &mut suggestions).await?;
    detect_repetitive_operations(pool, &user_id, &mut suggestions).await?;
    detect_workflow_pattern(pool, &user_id, &mut suggestions).await?;
    // D2.4 主动决策深化：5 个新检测点（触发执行式，记录 Suggestion 而非对话回复）
    detect_session_stall(pool, &user_id, &mut suggestions).await?;
    detect_skill_dormant(pool, &user_id, &mut suggestions).await?;
    detect_mcp_failure(pool, &user_id, &mut suggestions).await?;
    detect_error_spike(pool, &user_id, &mut suggestions).await?;
    detect_git_uncommitted(pool, &user_id, &mut suggestions).await?;
    // D2.5 V4→V5 迁移：5 个问答式函数的触发执行式版本（渐进兼容，保留旧问答式命令）
    kb_summarize_proactive(pool, &user_id, &mut suggestions).await?;
    kb_classify_proactive(pool, &user_id, &mut suggestions).await?;
    todo_enhance_proactive(pool, &user_id, &mut suggestions).await?;
    resume_spell_check_proactive(pool, &user_id, &mut suggestions).await?;
    news_summarize_proactive(pool, &user_id, &mut suggestions).await?;

    let mut saved = Vec::new();
    for s in suggestions {
        let already_exists = suggestion_repo::has_similar_suggestion(
            pool,
            &s.user_id,
            &s.category,
            &s.title,
        )
        .await
        .unwrap_or(false);

        if !already_exists {
            let saved_suggestion = suggestion_repo::insert_suggestion(
                pool,
                &s.user_id,
                &s.category,
                &s.title,
                &s.description,
                &s.priority,
                &s.source,
            )
            .await?;
            saved.push(saved_suggestion);
        }
    }

    Ok(ApiResponse::success(saved))
}

async fn detect_todo_neglect(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let incomplete_days: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT date) FROM todos
         WHERE completed = 0 AND date < date('now')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if incomplete_days >= 2 {
        suggestions.push(Suggestion {
            id: 0,
            user_id: user_id.to_string(),
            category: "productivity".to_string(),
            title: "待办事项积压".to_string(),
            description: format!(
                "您已连续 {} 天有未完成的待办事项。建议将大任务拆分为更小的子任务，或重新评估优先级。",
                incomplete_days
            ),
            priority: "high".to_string(),
            source: "rule".to_string(),
            status: "unread".to_string(),
            created_at: None,
            updated_at: None,
        });
    }
    Ok(())
}

async fn detect_late_night_usage(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let late_night_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND strftime('%H', timestamp) >= '22'
         AND date(timestamp) >= date('now', '-7 days')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let late_night_days: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT date(timestamp)) FROM activity_logs
         WHERE user_id = ? AND strftime('%H', timestamp) >= '22'
         AND date(timestamp) >= date('now', '-7 days')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if late_night_days >= 3 && late_night_count > 10 {
        suggestions.push(Suggestion {
            id: 0,
            user_id: user_id.to_string(),
            category: "health".to_string(),
            title: "深夜使用提醒".to_string(),
            description: format!(
                "最近7天有{}天在深夜（22:00后）使用应用，共{}次操作。建议调整作息，保证充足睡眠。",
                late_night_days, late_night_count
            ),
            priority: "normal".to_string(),
            source: "rule".to_string(),
            status: "unread".to_string(),
            created_at: None,
            updated_at: None,
        });
    }
    Ok(())
}

async fn detect_knowledge_fragmentation(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let today_entries: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND date(timestamp) = date('now')
         AND module = 'knowledge_base' AND operation LIKE '%打开%'",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let today_kb_duration: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(duration_secs), 0) FROM activity_logs
         WHERE user_id = ? AND date(timestamp) = date('now')
         AND module = 'knowledge_base'",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if today_entries >= 5 && today_kb_duration > 0 {
        let avg_secs = today_kb_duration / today_entries;
        if avg_secs < 300 {
            suggestions.push(Suggestion {
                id: 0,
                user_id: user_id.to_string(),
                category: "focus".to_string(),
                title: "知识库学习碎片化".to_string(),
                description: format!(
                    "今日打开了 {} 个知识条目，平均每个条目仅停留 {} 秒。建议每次专注1-2个主题深入学习。",
                    today_entries, avg_secs
                ),
                priority: "high".to_string(),
                source: "rule".to_string(),
                status: "unread".to_string(),
                created_at: None,
                updated_at: None,
            });
        }
    }
    Ok(())
}

async fn detect_feature_neglect(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let features = vec![
        ("terminal", "终端"),
        ("journal", "日志"),
        ("timer", "计时器"),
        ("game", "游戏"),
        ("news", "新闻"),
        ("resume", "简历"),
        ("quote", "语录"),
    ];

    for (module, name) in features {
        let last_used: Option<String> = sqlx::query_scalar(
            "SELECT MAX(timestamp) FROM activity_logs WHERE user_id = ? AND module = ?",
        )
        .bind(user_id)
        .bind(module)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        let should_remind = match &last_used {
            Some(ts) => {
                let days_ago: i64 = sqlx::query_scalar(
                    "SELECT julianday('now') - julianday(?)",
                )
                .bind(ts)
                .fetch_one(pool)
                .await
                .unwrap_or(0.0) as i64;
                days_ago >= 14
            }
            None => true,
        };

        if should_remind {
            suggestions.push(Suggestion {
                id: 0,
                user_id: user_id.to_string(),
                category: "discovery".to_string(),
                title: format!("{} 功能探索", name),
                description: format!(
                    "您已有一段时间未使用「{}」功能。试试用它来提升您的工作效率吧！",
                    name
                ),
                priority: "low".to_string(),
                source: "rule".to_string(),
                status: "unread".to_string(),
                created_at: None,
                updated_at: None,
            });
        }
    }
    Ok(())
}

async fn detect_repetitive_operations(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let repeated: Vec<(String, i64)> = sqlx::query_as::<_, (String, i64)>(
        "SELECT operation, COUNT(*) as cnt FROM activity_logs
         WHERE user_id = ? AND date(timestamp) = date('now')
         AND remark LIKE '%错误%'
         GROUP BY operation HAVING cnt >= 3",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    for (op, count) in repeated {
        suggestions.push(Suggestion {
            id: 0,
            user_id: user_id.to_string(),
            category: "efficiency".to_string(),
            title: "重复操作提醒".to_string(),
            description: format!(
                "今日「{}」操作出现了 {} 次错误。建议检查是否有更高效的替代方案。",
                op, count
            ),
            priority: "normal".to_string(),
            source: "rule".to_string(),
            status: "unread".to_string(),
            created_at: None,
            updated_at: None,
        });
    }
    Ok(())
}

async fn detect_workflow_pattern(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let recent_logs: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
        "SELECT timestamp, module FROM activity_logs
         WHERE user_id = ? AND date(timestamp) >= date('now', '-7 days')
         ORDER BY timestamp ASC LIMIT 500",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut patterns: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for window in recent_logs.windows(2) {
        let pattern = format!("{} → {}", window[0].1, window[1].1);
        if window[0].1 != window[1].1 {
            *patterns.entry(pattern).or_insert(0) += 1;
        }
    }

    let top_patterns: Vec<_> = {
        let mut v: Vec<_> = patterns.into_iter().filter(|(_, c)| *c >= 3).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    };

    if let Some((pattern, count)) = top_patterns.first() {
        suggestions.push(Suggestion {
            id: 0,
            user_id: user_id.to_string(),
            category: "workflow".to_string(),
            title: "工作流模式发现".to_string(),
            description: format!(
                "检测到您经常进行「{}」的操作序列（近7天出现 {} 次）。可以考虑将此流程设为快捷工作流。",
                pattern, count
            ),
            priority: "normal".to_string(),
            source: "pattern".to_string(),
            status: "unread".to_string(),
            created_at: None,
            updated_at: None,
        });
    }
    Ok(())
}

// ===== D2.4 主动决策深化：5 个新检测点（触发执行式） =====
// 设计依据：.trae/rules/项目核心设计意图.md §2.3「触发执行式 / 优化非问答」
// 每个检测点的输出是「记录 Suggestion 到 DB」而非「对话回复」，
// 符合底层智能作为「智能优化层」的定位。

/// D2.4-1 会话停滞检测
/// 触发条件：近 30 分钟无 AI 对话（module='ai_chat'）但近 30 分钟有其他模块活动
/// 自动执行动作：记录"会话回顾"建议到 dashboard
async fn detect_session_stall(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let recent_ai_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND module = 'ai_chat'
         AND timestamp >= datetime('now', '-30 minutes')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let recent_other_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND module != 'ai_chat'
         AND timestamp >= datetime('now', '-30 minutes')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 阈值：30 分钟内无 AI 对话 + 其他模块活动 ≥ 8（说明用户在使用但没问 AI）
    if recent_ai_count == 0 && recent_other_count >= 8 {
        suggestions.push(Suggestion {
            id: 0,
            user_id: user_id.to_string(),
            category: "productivity".to_string(),
            title: "会话停滞提醒".to_string(),
            description: format!(
                "近 30 分钟您进行了 {} 次操作但未使用 AI 助手。如有需要，可以随时召唤小欣或使用 Yuan Code 获取智能辅助。",
                recent_other_count
            ),
            priority: "normal".to_string(),
            source: "rule".to_string(),
            status: "unread".to_string(),
            created_at: None,
            updated_at: None,
        });
    }
    Ok(())
}

/// D2.4-2 技能闲置检测
/// 触发条件：近 7 天无技能调用（module='skill'）但用户总活动 > 50
/// 自动执行动作：记录"技能探索"建议
async fn detect_skill_dormant(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let skill_invocations_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND module = 'skill'
         AND date(timestamp) >= date('now', '-7 days')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let total_activity_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ?
         AND date(timestamp) >= date('now', '-7 days')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 阈值：7 天无技能调用 + 总活动 > 50（说明用户活跃但未用技能）
    if skill_invocations_7d == 0 && total_activity_7d > 50 {
        suggestions.push(Suggestion {
            id: 0,
            user_id: user_id.to_string(),
            category: "exploration".to_string(),
            title: "技能闲置提醒".to_string(),
            description: format!(
                "近 7 天您进行了 {} 次操作但未触发任何技能。Yuan Code 技能市场提供 code-review、test-writer、security-audit 等 10 个预置技能，可显著提升编程效率。",
                total_activity_7d
            ),
            priority: "normal".to_string(),
            source: "rule".to_string(),
            status: "unread".to_string(),
            created_at: None,
            updated_at: None,
        });
    }
    Ok(())
}

/// D2.4-3 MCP 服务器失效检测
/// 触发条件：近 24 小时 MCP 调用失败 ≥ 3 次
/// 自动执行动作：记录"MCP 检查"建议
async fn detect_mcp_failure(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let mcp_failures_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND module = 'mcp'
         AND (operation LIKE '%fail%' OR operation LIKE '%error%' OR operation LIKE '%disconnect%')
         AND timestamp >= datetime('now', '-1 day')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 阈值：24 小时内 MCP 失败 ≥ 3 次
    if mcp_failures_24h >= 3 {
        suggestions.push(Suggestion {
            id: 0,
            user_id: user_id.to_string(),
            category: "system".to_string(),
            title: "MCP 服务器异常".to_string(),
            description: format!(
                "近 24 小时检测到 {} 次 MCP 调用失败。建议在 Yuan Code → MCP 面板检查服务器连接状态，必要时重启失效的服务器。",
                mcp_failures_24h
            ),
            priority: "high".to_string(),
            source: "rule".to_string(),
            status: "unread".to_string(),
            created_at: None,
            updated_at: None,
        });
    }
    Ok(())
}

/// D2.4-4 错误激增检测
/// 触发条件：近 1 小时错误类操作 ≥ 10 次
/// 自动执行动作：记录"错误模式分析"建议
async fn detect_error_spike(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let error_count_1h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ?
         AND (operation LIKE '%error%' OR operation LIKE '%fail%' OR operation LIKE '%exception%')
         AND timestamp >= datetime('now', '-1 hour')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 阈值：1 小时内错误类操作 ≥ 10 次
    if error_count_1h >= 10 {
        suggestions.push(Suggestion {
            id: 0,
            user_id: user_id.to_string(),
            category: "system".to_string(),
            title: "错误激增预警".to_string(),
            description: format!(
                "近 1 小时检测到 {} 次错误类操作。建议暂停当前操作，检查最近的日志或调用栈，定位根因后再继续，避免错误累积。",
                error_count_1h
            ),
            priority: "high".to_string(),
            source: "rule".to_string(),
            status: "unread".to_string(),
            created_at: None,
            updated_at: None,
        });
    }
    Ok(())
}

/// D2.4-5 git 未提交检测
/// 触发条件：近 24 小时有文件修改操作（module='git' AND operation LIKE '%modify%'）
///          但无 commit 操作，且修改次数 ≥ 5
/// 自动执行动作：记录"提交提醒"建议
async fn detect_git_uncommitted(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let git_modifies_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND module = 'git'
         AND (operation LIKE '%modify%' OR operation LIKE '%edit%' OR operation LIKE '%write%')
         AND timestamp >= datetime('now', '-1 day')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let git_commits_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND module = 'git'
         AND (operation LIKE '%commit%' OR operation LIKE '%push%')
         AND timestamp >= datetime('now', '-1 day')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 阈值：24 小时内文件修改 ≥ 5 次且无 commit
    if git_modifies_24h >= 5 && git_commits_24h == 0 {
        suggestions.push(Suggestion {
            id: 0,
            user_id: user_id.to_string(),
            category: "productivity".to_string(),
            title: "代码未提交提醒".to_string(),
            description: format!(
                "近 24 小时您修改了 {} 次文件但未进行任何提交。建议及时提交代码，避免工作丢失或合并冲突。",
                git_modifies_24h
            ),
            priority: "normal".to_string(),
            source: "rule".to_string(),
            status: "unread".to_string(),
            created_at: None,
            updated_at: None,
        });
    }
    Ok(())
}

// ===== D2.5 V4→V5 迁移：问答式 → 触发执行式（渐进兼容） =====
// 设计依据：.trae/rules/项目核心设计意图.md §8.1「底层智能输出必须是优化建议/触发执行」
// 策略：保留旧问答式命令（intelligence_v4_kb_summarize 等），新增 _proactive 触发执行式版本，
//        由 generate_suggestions 自动调用，输出为 Suggestion（非对话回复）。
// 迁移的 5 个 V4 函数：
//   1. kb_summarize        → kb_summarize_proactive
//   2. kb_classify         → kb_classify_proactive
//   3. todo_enhance        → todo_enhance_proactive
//   4. resume_spell_check  → resume_spell_check_proactive
//   5. news_summarize      → news_summarize_proactive

/// D2.5-1 知识库主动摘要（迁移自 kb_summarize）
/// 触发条件：近 7 天新增 KB 条目 ≥ 3 个但无摘要操作
/// 自动执行动作：记录"启用摘要"建议
async fn kb_summarize_proactive(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let new_kb_entries_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kb_entries
         WHERE created_at >= strftime('%s', 'now', '-7 days')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let kb_summarize_actions_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND module = 'kb' AND operation LIKE '%summarize%'
         AND date(timestamp) >= date('now', '-7 days')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 阈值：7 天新增 ≥ 3 个 KB 条目但无摘要操作
    if new_kb_entries_7d >= 3 && kb_summarize_actions_7d == 0 {
        suggestions.push(Suggestion {
            id: 0,
            user_id: user_id.to_string(),
            category: "knowledge".to_string(),
            title: "知识库摘要建议".to_string(),
            description: format!(
                "近 7 天新增了 {} 个知识库条目但未生成摘要。建议对重要条目启用 AI 摘要，便于快速回顾。",
                new_kb_entries_7d
            ),
            priority: "normal".to_string(),
            source: "rule".to_string(),
            status: "unread".to_string(),
            created_at: None,
            updated_at: None,
        });
    }
    Ok(())
}

/// D2.5-2 知识库主动分类（迁移自 kb_classify）
/// 触发条件：KB 条目集中在单一分类（某分类占比 > 60% 且总条目 > 10）
/// 自动执行动作：记录"分类优化"建议
async fn kb_classify_proactive(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let total_entries: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM kb_entries")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let max_category_count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(cnt), 0) FROM (
            SELECT COUNT(*) AS cnt FROM kb_entries GROUP BY category_id
        )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 阈值：总条目 > 10 且单一分类占比 > 60%
    if total_entries > 10 && max_category_count * 100 > total_entries * 60 {
        let ratio = (max_category_count * 100) / total_entries;
        suggestions.push(Suggestion {
            id: 0,
            user_id: user_id.to_string(),
            category: "knowledge".to_string(),
            title: "知识库分类优化".to_string(),
            description: format!(
                "检测到知识库 {} 个条目中，单一分类占比达 {}%。建议启用 AI 分类，将条目更均衡地分布到多个分类，便于检索。",
                total_entries, ratio
            ),
            priority: "normal".to_string(),
            source: "rule".to_string(),
            status: "unread".to_string(),
            created_at: None,
            updated_at: None,
        });
    }
    Ok(())
}

/// D2.5-3 待办主动增强（迁移自 todo_enhance）
/// 触发条件：存在未完成且标题简短（<10 字）的待办 ≥ 3 个
/// 自动执行动作：记录"增强描述"建议
async fn todo_enhance_proactive(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let short_todos: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM todos
         WHERE completed = 0 AND LENGTH(title) < 10",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 阈值：未完成且简短待办 ≥ 3 个
    if short_todos >= 3 {
        suggestions.push(Suggestion {
            id: 0,
            user_id: user_id.to_string(),
            category: "productivity".to_string(),
            title: "待办事项增强".to_string(),
            description: format!(
                "检测到 {} 个未完成待办的标题过于简短（少于10字）。建议使用 AI 增强待办描述，补充执行步骤和优先级，提升可执行性。",
                short_todos
            ),
            priority: "normal".to_string(),
            source: "rule".to_string(),
            status: "unread".to_string(),
            created_at: None,
            updated_at: None,
        });
    }
    Ok(())
}

/// D2.5-4 简历主动拼写检查（迁移自 resume_spell_check）
/// 触发条件：近 7 天有简历更新但无 spell_check 操作
/// 自动执行动作：记录"运行拼写检查"建议
async fn resume_spell_check_proactive(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let resume_updates_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND module = 'resume'
         AND (operation LIKE '%update%' OR operation LIKE '%edit%')
         AND date(timestamp) >= date('now', '-7 days')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let spell_checks_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND module = 'resume' AND operation LIKE '%spell%'
         AND date(timestamp) >= date('now', '-7 days')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 阈值：7 天有简历更新但无 spell_check
    if resume_updates_7d > 0 && spell_checks_7d == 0 {
        suggestions.push(Suggestion {
            id: 0,
            user_id: user_id.to_string(),
            category: "career".to_string(),
            title: "简历拼写检查建议".to_string(),
            description: format!(
                "近 7 天您更新了 {} 次简历但未运行拼写检查。建议提交简历前进行 AI 拼写检查，避免低级错误影响专业形象。",
                resume_updates_7d
            ),
            priority: "normal".to_string(),
            source: "rule".to_string(),
            status: "unread".to_string(),
            created_at: None,
            updated_at: None,
        });
    }
    Ok(())
}

/// D2.5-5 新闻主动摘要（迁移自 news_summarize）
/// 触发条件：近 24 小时获取新闻 ≥ 5 条但无摘要操作
/// 自动执行动作：记录"启用新闻摘要"建议
async fn news_summarize_proactive(
    pool: &SqlitePool,
    user_id: &str,
    suggestions: &mut Vec<Suggestion>,
) -> Result<(), AppError> {
    let news_fetches_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND module = 'news'
         AND (operation LIKE '%fetch%' OR operation LIKE '%read%')
         AND timestamp >= datetime('now', '-1 day')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let news_summaries_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND module = 'news' AND operation LIKE '%summarize%'
         AND timestamp >= datetime('now', '-1 day')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 阈值：24 小时获取 ≥ 5 条新闻但无摘要
    if news_fetches_24h >= 5 && news_summaries_24h == 0 {
        suggestions.push(Suggestion {
            id: 0,
            user_id: user_id.to_string(),
            category: "information".to_string(),
            title: "新闻摘要建议".to_string(),
            description: format!(
                "近 24 小时您获取了 {} 条新闻但未启用 AI 摘要。建议对重要新闻启用摘要功能，快速获取关键信息，节省阅读时间。",
                news_fetches_24h
            ),
            priority: "normal".to_string(),
            source: "rule".to_string(),
            status: "unread".to_string(),
            created_at: None,
            updated_at: None,
        });
    }
    Ok(())
}

pub async fn query_suggestions(
    pool: &SqlitePool,
    user_id: String,
    status: Option<String>,
    category: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<ApiResponse<SuggestionPage>, AppError> {
    let limit = limit.unwrap_or(20).min(100);
    let offset = offset.unwrap_or(0);

    let (suggestions, total) = suggestion_repo::query_suggestions(
        pool,
        &user_id,
        status.as_deref(),
        category.as_deref(),
        limit,
        offset,
    )
    .await?;

    let page = SuggestionPage {
        suggestions,
        total,
        page: offset / limit + 1,
        page_size: limit,
    };

    Ok(ApiResponse::success(page))
}

pub async fn mark_suggestion(
    pool: &SqlitePool,
    feedback: SuggestionFeedback,
    user_id: &str,
) -> Result<ApiResponse<()>, AppError> {
    let valid_actions = ["read", "adopt", "ignore"];
    if !valid_actions.contains(&feedback.action.as_str()) {
        return Err(AppError::Validation("无效的操作，支持: read/adopt/ignore".into()));
    }

    suggestion_repo::update_suggestion_status(pool, feedback.suggestion_id, &feedback.action, user_id).await?;
    Ok(ApiResponse::success(()))
}

pub async fn clean_suggestions(
    pool: &SqlitePool,
    retention_days: i64,
) -> Result<ApiResponse<u64>, AppError> {
    let deleted = suggestion_repo::cleanup_old_suggestions(pool, retention_days).await?;
    Ok(ApiResponse::success(deleted))
}

// ========== 行为分析引擎 ==========

pub async fn analyze_behavior(
    pool: &SqlitePool,
    user_id: String,
    date: Option<String>,
) -> Result<ApiResponse<BehaviorAnalysis>, AppError> {
    let date = date.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());

    let cached = behavior_analysis_repo::get_pattern_by_date(pool, &user_id, &date).await?;
    if let Some(pattern) = cached {
        return Ok(ApiResponse::success(build_analysis_from_pattern(&pattern, pool, &user_id).await?));
    }

    let raw = compute_daily_behavior(pool, &user_id, &date).await?;
    let _ = behavior_analysis_repo::insert_behavior_pattern(pool, &raw).await;
    Ok(ApiResponse::success(build_analysis_from_pattern(&raw, pool, &user_id).await?))
}

pub async fn get_behavior_trend(
    pool: &SqlitePool,
    user_id: String,
    days: Option<i64>,
) -> Result<ApiResponse<BehaviorTrend>, AppError> {
    let days = days.unwrap_or(7).min(30);
    let patterns = behavior_analysis_repo::get_trend_data(pool, &user_id, days).await?;

    let trend = BehaviorTrend {
        dates: patterns.iter().map(|p| p.date.clone()).collect(),
        focus_scores: patterns.iter().map(|p| p.focus_score).collect(),
        distraction_counts: patterns.iter().map(|p| p.distraction_count).collect(),
        kb_avg_durations: patterns.iter().map(|p| p.kb_avg_duration_secs).collect(),
        error_rates: patterns
            .iter()
            .map(|p| {
                if p.total_operation_count > 0 {
                    (p.error_operation_count as f64 / p.total_operation_count as f64 * 100.0)
                        .round()
                        / 100.0
                } else {
                    0.0
                }
            })
            .collect(),
        consistency_scores: patterns.iter().map(|p| p.consistency_score).collect(),
    };

    Ok(ApiResponse::success(trend))
}

pub async fn query_behavior_history(
    pool: &SqlitePool,
    user_id: String,
    start_date: Option<String>,
    end_date: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<ApiResponse<(Vec<BehaviorPattern>, i64)>, AppError> {
    let end = end_date.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
    let start = start_date.unwrap_or_else(|| {
        chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::days(30))
            .unwrap_or(chrono::Utc::now())
            .format("%Y-%m-%d")
            .to_string()
    });
    let limit = limit.unwrap_or(30);
    let offset = offset.unwrap_or(0);

    let (patterns, total) =
        behavior_analysis_repo::query_patterns(pool, &user_id, &start, &end, limit, offset).await?;
    Ok(ApiResponse::success((patterns, total)))
}

async fn compute_daily_behavior(
    pool: &SqlitePool,
    user_id: &str,
    date: &str,
) -> Result<BehaviorPattern, AppError> {
    let kb_entries: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND date(timestamp) = ? AND module = 'knowledge_base'
         AND operation LIKE '%打开%'",
    )
    .bind(user_id)
    .bind(date)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let kb_total_secs: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(duration_secs), 0) FROM activity_logs
         WHERE user_id = ? AND date(timestamp) = ? AND module = 'knowledge_base'",
    )
    .bind(user_id)
    .bind(date)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let kb_avg_duration = if kb_entries > 0 {
        kb_total_secs / kb_entries
    } else {
        0
    };

    let focus_score = if kb_avg_duration > 600 {
        90.0
    } else if kb_avg_duration > 300 {
        75.0
    } else if kb_avg_duration > 120 {
        50.0
    } else if kb_avg_duration > 0 {
        30.0
    } else {
        0.0
    };

    let quick_switches: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND date(timestamp) = ? AND module = 'knowledge_base'
         AND duration_secs > 0 AND duration_secs < 180",
    )
    .bind(user_id)
    .bind(date)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let total_ops: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs WHERE user_id = ? AND date(timestamp) = ?",
    )
    .bind(user_id)
    .bind(date)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let error_ops: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND date(timestamp) = ? AND remark LIKE '%错误%'",
    )
    .bind(user_id)
    .bind(date)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let module_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT module) FROM activity_logs
         WHERE user_id = ? AND date(timestamp) = ?",
    )
    .bind(user_id)
    .bind(date)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let active_start: Option<i32> = sqlx::query_scalar(
        "SELECT MIN(CAST(strftime('%H', timestamp) AS INTEGER)) FROM activity_logs
         WHERE user_id = ? AND date(timestamp) = ?",
    )
    .bind(user_id)
    .bind(date)
    .fetch_one(pool)
    .await
    .ok()
    .flatten();

    let active_end: Option<i32> = sqlx::query_scalar(
        "SELECT MAX(CAST(strftime('%H', timestamp) AS INTEGER)) FROM activity_logs
         WHERE user_id = ? AND date(timestamp) = ?",
    )
    .bind(user_id)
    .bind(date)
    .fetch_one(pool)
    .await
    .ok()
    .flatten();

    let peak_hour: Option<i32> = sqlx::query_scalar(
        "SELECT CAST(strftime('%H', timestamp) AS INTEGER) as h FROM activity_logs
         WHERE user_id = ? AND date(timestamp) = ?
         GROUP BY h ORDER BY COUNT(*) DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(date)
    .fetch_one(pool)
    .await
    .ok()
    .flatten();

    let consistency_score = compute_consistency(pool, user_id, date).await?;

    let summary = generate_behavior_summary(
        focus_score,
        quick_switches,
        kb_avg_duration,
        error_ops,
        total_ops,
        module_count,
    );

    Ok(BehaviorPattern {
        id: 0,
        user_id: user_id.to_string(),
        date: date.to_string(),
        focus_score,
        distraction_count: quick_switches,
        kb_avg_duration_secs: kb_avg_duration,
        kb_entry_count: kb_entries,
        error_operation_count: error_ops,
        total_operation_count: total_ops,
        active_start_hour: active_start,
        active_end_hour: active_end,
        peak_hour,
        module_diversity: module_count,
        consistency_score,
        summary: Some(summary),
        created_at: None,
    })
}

async fn compute_consistency(
    pool: &SqlitePool,
    user_id: &str,
    date: &str,
) -> Result<f64, AppError> {
    let past_hours: Vec<(i64,)> = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND date(timestamp) = date(?, '-3 days')
         AND date(timestamp) < ?
         GROUP BY CAST(strftime('%H', timestamp) AS INTEGER)",
    )
    .bind(user_id)
    .bind(date)
    .bind(date)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let today_hours: Vec<(i64,)> = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND date(timestamp) = ?
         GROUP BY CAST(strftime('%H', timestamp) AS INTEGER)",
    )
    .bind(user_id)
    .bind(date)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if past_hours.is_empty() || today_hours.is_empty() {
        return Ok(50.0);
    }

    let past_count = past_hours.len() as f64;
    let today_count = today_hours.len() as f64;
    let overlap = past_count.min(today_count) / past_count.max(today_count);
    Ok((overlap * 100.0).round())
}

fn generate_behavior_summary(
    focus_score: f64,
    quick_switches: i64,
    kb_avg_duration: i64,
    error_ops: i64,
    total_ops: i64,
    module_count: i64,
) -> String {
    let mut parts = Vec::new();

    match focus_score as i64 {
        0 => parts.push("今日无知识库学习记录".to_string()),
        s if s < 30 => parts.push("学习状态：严重碎片化，频繁切换内容".to_string()),
        s if s < 50 => parts.push("学习状态：注意力较分散，建议减少切换".to_string()),
        s if s < 75 => parts.push("学习状态：基本专注，仍有改进空间".to_string()),
        _ => parts.push("学习状态：高度专注，表现优秀".to_string()),
    }

    if quick_switches >= 3 {
        parts.push(format!("分心次数：{} 次", quick_switches));
    }

    if kb_avg_duration > 0 {
        let min = kb_avg_duration / 60;
        parts.push(format!("平均专注时长：{} 分钟", min));
    }

    if total_ops > 0 {
        let error_rate = error_ops as f64 / total_ops as f64 * 100.0;
        if error_rate > 10.0 {
            parts.push(format!("错误率偏高：{:.0}%", error_rate));
        }
    }

    if module_count <= 2 && total_ops > 0 {
        parts.push(format!("使用模块集中：{} 个模块", module_count));
    } else if module_count >= 6 {
        parts.push(format!("使用模块较多：{} 个模块", module_count));
    }

    parts.join("。")
}

fn build_focus_verdict(score: f64) -> String {
    match score as i64 {
        s if s >= 75 => "高度专注".to_string(),
        s if s >= 50 => "基本专注".to_string(),
        s if s >= 30 => "注意力分散".to_string(),
        _ => "严重分心".to_string(),
    }
}

fn build_distraction_verdict(count: i64) -> String {
    match count {
        0 => "无分心行为".to_string(),
        c if c <= 2 => "偶尔分心".to_string(),
        c if c <= 5 => "分心较多".to_string(),
        _ => "频繁分心，三心二意".to_string(),
    }
}

fn build_error_verdict(rate: f64) -> String {
    if rate == 0.0 {
        "零错误，操作精准".to_string()
    } else if rate <= 5.0 {
        "错误率低，操作熟练".to_string()
    } else if rate <= 15.0 {
        "有一定错误率，注意提升".to_string()
    } else {
        "错误率偏高，需要加强".to_string()
    }
}

fn build_diversity_verdict(count: i64) -> String {
    match count {
        0 => "今日无活动".to_string(),
        c if c <= 2 => "模块使用集中，目标明确".to_string(),
        c if c <= 4 => "模块使用适中".to_string(),
        _ => "模块使用分散，可能注意力不集中".to_string(),
    }
}

fn build_consistency_verdict(score: f64) -> String {
    match score as i64 {
        s if s >= 75 => "作息规律，习惯稳定".to_string(),
        s if s >= 50 => "作息基本规律".to_string(),
        _ => "作息不规律，波动较大".to_string(),
    }
}

async fn build_analysis_from_pattern(
    pattern: &BehaviorPattern,
    pool: &SqlitePool,
    user_id: &str,
) -> Result<BehaviorAnalysis, AppError> {
    let error_rate = if pattern.total_operation_count > 0 {
        (pattern.error_operation_count as f64 / pattern.total_operation_count as f64 * 100.0)
            .round()
            / 100.0
    } else {
        0.0
    };

    let active_period = match (pattern.active_start_hour, pattern.active_end_hour) {
        (Some(s), Some(e)) => format!("{}:00 - {}:00", s, e + 1),
        (Some(s), None) => format!("{}:00 开始", s),
        _ => "暂无数据".to_string(),
    };

    let peak_hour_label = pattern.peak_hour.map(|h| match h {
        h if h < 6 => format!("凌晨 {}:00", h),
        h if h < 12 => format!("上午 {}:00", h),
        h if h < 14 => format!("中午 {}:00", h),
        h if h < 18 => format!("下午 {}:00", h),
        _ => format!("晚上 {}:00", h),
    });

    let comparison = build_comparison(pool, user_id, pattern).await;

    Ok(BehaviorAnalysis {
        date: pattern.date.clone(),
        focus_score: pattern.focus_score,
        focus_verdict: build_focus_verdict(pattern.focus_score),
        distraction_count: pattern.distraction_count,
        distraction_verdict: build_distraction_verdict(pattern.distraction_count),
        kb_avg_duration_secs: pattern.kb_avg_duration_secs,
        kb_avg_duration_human: if pattern.kb_avg_duration_secs >= 60 {
            format!("{}分钟", pattern.kb_avg_duration_secs / 60)
        } else {
            format!("{}秒", pattern.kb_avg_duration_secs)
        },
        kb_entry_count: pattern.kb_entry_count,
        error_rate,
        error_verdict: build_error_verdict(error_rate),
        active_period,
        peak_hour: pattern.peak_hour,
        peak_hour_label,
        module_diversity: pattern.module_diversity,
        module_diversity_verdict: build_diversity_verdict(pattern.module_diversity),
        consistency_score: pattern.consistency_score,
        consistency_verdict: build_consistency_verdict(pattern.consistency_score),
        summary: pattern.summary.clone().unwrap_or_default(),
        comparison,
    })
}

async fn build_comparison(
    pool: &SqlitePool,
    user_id: &str,
    today: &BehaviorPattern,
) -> Option<BehaviorComparison> {
    let today_date = &today.date;
    let week_start = chrono::NaiveDate::parse_from_str(today_date, "%Y-%m-%d")
        .ok()?
        .checked_sub_signed(chrono::Duration::days(7))?
        .format("%Y-%m-%d")
        .to_string();
    let yesterday = chrono::NaiveDate::parse_from_str(today_date, "%Y-%m-%d")
        .ok()?
        .checked_sub_signed(chrono::Duration::days(1))?
        .format("%Y-%m-%d")
        .to_string();

    let avg = behavior_analysis_repo::get_avg_patterns(pool, user_id, &week_start, &yesterday)
        .await
        .ok()
        .flatten()?;

    let today_error_rate = if today.total_operation_count > 0 {
        today.error_operation_count as f64 / today.total_operation_count as f64 * 100.0
    } else {
        0.0
    };

    let avg_error_rate = if avg.total_operation_count > 0 {
        avg.error_operation_count as f64 / avg.total_operation_count as f64 * 100.0
    } else {
        0.0
    };

    let focus_change = today.focus_score - avg.focus_score;
    let distraction_change = avg.distraction_count as f64 - today.distraction_count as f64;
    let error_change = avg_error_rate - today_error_rate;
    let consistency_change = today.consistency_score - avg.consistency_score;

    let total_change = focus_change + distraction_change + error_change + consistency_change;
    let overall_trend = if total_change > 10.0 {
        "较历史表现明显提升".to_string()
    } else if total_change > 0.0 {
        "较历史表现略有提升".to_string()
    } else if total_change > -10.0 {
        "与历史表现基本持平".to_string()
    } else {
        "较历史表现有所下降".to_string()
    };

    Some(BehaviorComparison {
        period_label: "过去7天对比".to_string(),
        focus_change: (focus_change * 10.0).round() / 10.0,
        distraction_change: (distraction_change * 10.0).round() / 10.0,
        error_change: (error_change * 10.0).round() / 10.0,
        consistency_change: (consistency_change * 10.0).round() / 10.0,
        overall_trend,
    })
}

// ========== 设置管理 ==========

pub async fn get_settings(
    pool: &SqlitePool,
    user_id: String,
) -> Result<ApiResponse<IntelligenceSettings>, AppError> {
    let settings = intelligence_settings_repo::get_or_create_default(pool, &user_id).await?;
    Ok(ApiResponse::success(settings))
}

pub async fn save_settings(
    pool: &SqlitePool,
    settings: IntelligenceSettings,
) -> Result<ApiResponse<IntelligenceSettings>, AppError> {
    let saved = intelligence_settings_repo::upsert_settings(pool, &settings).await?;
    Ok(ApiResponse::success(saved))
}

pub async fn reset_settings(
    pool: &SqlitePool,
    user_id: String,
) -> Result<ApiResponse<IntelligenceSettings>, AppError> {
    intelligence_settings_repo::delete_settings(pool, &user_id).await?;
    let settings = intelligence_settings_repo::get_or_create_default(pool, &user_id).await?;
    Ok(ApiResponse::success(settings))
}

// ========== 连接测试 ==========

pub async fn test_connection(
    provider: &str,
    endpoint: &str,
    api_key: Option<&str>,
    _model: Option<&str>,
) -> Result<ApiResponse<TestConnectionResult>, AppError> {
    use std::time::Instant;

    let start = Instant::now();

    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::AiApi(format!("创建HTTP客户端失败: {e}")))?;

    let result: Result<reqwest::Response, reqwest::Error> = match provider {
        "ollama" => {
            let url = format!("{}/api/tags", endpoint.trim_end_matches('/'));
            client.get(&url).send().await
        }
        "openai" => {
            let url = format!("{}/models", endpoint.trim_end_matches('/'));
            let mut req = client.get(&url);
            if let Some(key) = api_key {
                req = req.header("Authorization", format!("Bearer {key}"));
            }
            req.send().await
        }
        _ => {
            client.get(endpoint).send().await
        }
    };

    let elapsed = start.elapsed().as_millis() as u64;

    match result {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() || status.as_u16() == 401 || status.as_u16() == 403 {
                // 401/403 means endpoint is reachable but auth is wrong
                let msg = if status == 200 {
                    "连接成功".to_string()
                } else {
                    format!("端点到通，但返回 HTTP {} (可能需要检查权限配置)", status.as_u16())
                };
                Ok(ApiResponse::success(TestConnectionResult {
                    success: true,
                    latency_ms: elapsed,
                    message: msg,
                }))
            } else {
                Ok(ApiResponse::success(TestConnectionResult {
                    success: false,
                    latency_ms: elapsed,
                    message: format!("端点返回 HTTP {}", status.as_u16()),
                }))
            }
        }
        Err(e) => Ok(ApiResponse::success(TestConnectionResult {
            success: false,
            latency_ms: elapsed,
            message: format!("连接失败: {e}"),
        })),
    }
}

// ========== 活动感知增强 ==========

pub async fn export_activity_logs(
    pool: &SqlitePool,
    user_id: Option<String>,
    module: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
) -> Result<ApiResponse<ActivityLogExport>, AppError> {
    let filters = ActivityLogExportFilter {
        user_id: user_id.clone(),
        module: module.clone(),
        start_time: start_time.clone(),
        end_time: end_time.clone(),
    };

    let (logs, total) = activity_log_repo::query_logs(
        pool,
        user_id.as_deref(),
        module.as_deref(),
        start_time.as_deref(),
        end_time.as_deref(),
        10000,
        0,
    )
    .await?;

    Ok(ApiResponse::success(ActivityLogExport {
        logs,
        total_count: total,
        export_time: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        filters,
    }))
}

pub fn get_operation_templates() -> Result<ApiResponse<Vec<OperationTemplate>>, AppError> {
    let templates = vec![
        OperationTemplate {
            module: "knowledge_base".into(),
            template: "打开资料库/{分类}/{文件名}".into(),
            description: "浏览知识库条目".into(),
        },
        OperationTemplate {
            module: "knowledge_base".into(),
            template: "下载资料库/{分类}/{文件名}".into(),
            description: "下载知识库文件".into(),
        },
        OperationTemplate {
            module: "terminal".into(),
            template: "在终端/{终端名}执行命令{命令内容}".into(),
            description: "终端命令执行".into(),
        },
        OperationTemplate {
            module: "terminal".into(),
            template: "在终端/{终端名}执行命令{命令内容}".into(),
            description: "终端命令执行（错误）".into(),
        },
        OperationTemplate {
            module: "resume".into(),
            template: "编辑简历/{简历名称}".into(),
            description: "简历编辑".into(),
        },
        OperationTemplate {
            module: "resume".into(),
            template: "导出简历/{简历名称}".into(),
            description: "简历导出".into(),
        },
        OperationTemplate {
            module: "quote".into(),
            template: "搜索语录/{关键词}".into(),
            description: "语录搜索".into(),
        },
        OperationTemplate {
            module: "quote".into(),
            template: "添加语录/{作者}-{出处}".into(),
            description: "语录收藏".into(),
        },
        OperationTemplate {
            module: "news".into(),
            template: "阅读新闻/{新闻标题}".into(),
            description: "新闻阅读".into(),
        },
        OperationTemplate {
            module: "news".into(),
            template: "生成新闻摘要/{新闻标题}".into(),
            description: "智能摘要生成".into(),
        },
        OperationTemplate {
            module: "journal".into(),
            template: "写日志".into(),
            description: "日志记录".into(),
        },
        OperationTemplate {
            module: "todo".into(),
            template: "创建待办/{待办标题}".into(),
            description: "待办事项创建".into(),
        },
        OperationTemplate {
            module: "todo".into(),
            template: "完成待办/{待办标题}".into(),
            description: "待办事项完成".into(),
        },
        OperationTemplate {
            module: "timer".into(),
            template: "开始计时/{计时标签}".into(),
            description: "专注计时开始".into(),
        },
        OperationTemplate {
            module: "timer".into(),
            template: "结束计时/{计时标签}".into(),
            description: "专注计时结束".into(),
        },
        OperationTemplate {
            module: "game".into(),
            template: "开始游戏/{游戏名称}".into(),
            description: "游戏启动".into(),
        },
        OperationTemplate {
            module: "search".into(),
            template: "搜索/{搜索关键词}".into(),
            description: "全局搜索".into(),
        },
        OperationTemplate {
            module: "editor".into(),
            template: "编辑文件/{文件名}".into(),
            description: "文件编辑".into(),
        },
        OperationTemplate {
            module: "settings".into(),
            template: "修改设置/{设置项}".into(),
            description: "系统设置变更".into(),
        },
    ];

    Ok(ApiResponse::success(templates))
}

pub async fn log_with_template(
    pool: &SqlitePool,
    user_id: String,
    module: String,
    template: String,
    detail: Option<String>,
    remark: Option<String>,
    duration_secs: Option<i64>,
) -> Result<ApiResponse<ActivityLog>, AppError> {
    let operation = if let Some(d) = &detail {
        template.replace("{detail}", d)
    } else {
        template
    };

    let record = ActivityLogRecord {
        user_id,
        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        module,
        operation,
        detail,
        remark,
        duration_secs,
    };

    log_activity(pool, record).await
}

// ========== 知识库智能体（LLM 驱动） ==========

use crate::models::intelligence_cross_module::{KbSummarizeRequest, KbSummarizeResult, KbTagRequest, KbTagResult};

/// 调用 LLM chat completion，返回文本响应
pub async fn call_llm_chat(
    provider: &str,
    endpoint: &str,
    api_key: Option<&str>,
    model: Option<&str>,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, AppError> {
    let provider_lower = provider.to_lowercase();
    let endpoint = endpoint.trim_end_matches('/').to_string();

    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(120))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::AiApi(format!("创建HTTP客户端失败: {e}")))?;

    let model = model.map(|m| m.to_string()).unwrap_or_else(|| match provider_lower.as_str() {
        "ollama" => "llama3.2".to_string(),
        "openai" => "gpt-4o-mini".to_string(),
        _ => "default".to_string(),
    });

    let body: serde_json::Value = match provider_lower.as_str() {
        "ollama" => {
            serde_json::json!({
                "model": model,
                "messages": [
                    { "role": "system", "content": system_prompt },
                    { "role": "user", "content": user_prompt }
                ],
                "stream": false
            })
        }
        _ => {
            serde_json::json!({
                "model": model,
                "messages": [
                    { "role": "system", "content": system_prompt },
                    { "role": "user", "content": user_prompt }
                ],
                "temperature": 0.7,
                "max_tokens": 1024
            })
        }
    };

    let url = match provider_lower.as_str() {
        "ollama" => format!("{}/api/chat", endpoint),
        _ => format!("{}/chat/completions", endpoint),
    };

    let mut req = client.post(&url).json(&body);
    if let Some(key) = api_key {
        if provider_lower.as_str() == "openai" || provider_lower.as_str() == "anthropic" {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
    }

    let resp = req.send().await.map_err(|e| {
        let err_str = e.to_string();
        if err_str.contains("sending request") || err_str.contains("connect") || err_str.contains("connection") || err_str.contains("refused") || err_str.contains("dns") {
            let hint = if provider_lower.as_str() == "ollama" {
                format!(" 请检查：① Ollama 是否已启动（终端运行 ollama serve）② 端口是否正确（当前: {}）③ Windows 防火墙是否拦截", endpoint)
            } else {
                format!(" 请检查网络连接和端点地址（当前: {}）", endpoint)
            };
            AppError::AiApi(format!("无法连接到 LLM 服务端 ({})：{}{}", provider_lower, err_str, hint))
        } else if err_str.contains("timeout") {
            AppError::AiApi(format!("LLM 请求超时（{}），模型可能正在加载，请稍后重试", endpoint))
        } else {
            AppError::AiApi(format!("LLM 请求失败: {e}"))
        }
    })?;

    if !resp.status().is_success() {
        return Err(AppError::AiApi(format!("LLM 返回 HTTP {}", resp.status().as_u16())));
    }

    let json: serde_json::Value = resp.json().await
        .map_err(|e| AppError::AiApi(format!("解析LLM响应失败: {e}")))?;

    let content = match provider_lower.as_str() {
        "ollama" => json["message"]["content"].as_str().unwrap_or(""),
        _ => json["choices"][0]["message"]["content"].as_str().unwrap_or(""),
    };

    if content.is_empty() {
        return Err(AppError::AiApi("LLM 返回空内容".into()));
    }

    Ok(content.to_string())
}

pub async fn kb_summarize(
    request: KbSummarizeRequest,
) -> Result<ApiResponse<KbSummarizeResult>, AppError> {
    let system_prompt = "你是一个专业的中文知识管理助手。请对用户提供的内容进行精炼总结。\n\
        \n\
        输出格式要求（严格遵守）：\n\
        第一行：直接写摘要正文，不要加【摘要】【核心摘要】等标题前缀\n\
        接下来：用 - 或 · 开头列出3个关键要点，每行一个\n\
        \n\
        示例：\n\
        本文介绍了Rust语言的所有权机制，包括借用规则和生命周期标注。\n\
        - 所有权确保内存安全无GC\n\
        - 借用检查器在编译期捕获错误\n\
        - 生命周期标注解决引用有效性";

    let user_prompt = format!(
        "请总结以下知识条目：\n\n【标题】{}\n\n【内容】\n{}",
        request.entry_name,
        if request.content.len() > 4000 { &request.content[..4000] } else { &request.content }
    );

    let result_text = call_llm_chat(
        &request.provider,
        &request.endpoint,
        request.api_key.as_deref(),
        request.model.as_deref(),
        system_prompt,
        &user_prompt,
    ).await?;

    // Parse LLM response: robust extraction of summary + key_points
    // 模型可能返回多种格式，需要兼容：
    // 格式A: "摘要正文\n- 要点1\n- 要点2"
    // 格式B: "【核心摘要】\n摘要正文\n• 要点1"
    // 格式C: "摘要正文"（无要点）
    let raw = result_text.trim();
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();

    // 分类：要点行 vs 内容行
    let mut content_lines: Vec<String> = Vec::new();
    let mut point_lines: Vec<String> = Vec::new();

    for line in &lines {
        let trimmed = line.trim();
        let is_point = trimmed.starts_with('-') || trimmed.starts_with('·')
            || trimmed.starts_with('*') || trimmed.starts_with("要点")
            || trimmed.starts_with("关键") || trimmed.starts_with("1.") || trimmed.starts_with("2.")
            || trimmed.starts_with("3.") || trimmed.starts_with("（")
            || (trimmed.len() > 2 && trimmed.chars().next() == Some('·'));

        if is_point {
            let cleaned = trimmed.trim_matches(&['-', '·', '*', ' ', '1', '2', '3', '.', '（', '）'] as &[_]).trim().to_string();
            if !cleaned.is_empty() {
                point_lines.push(cleaned);
            }
        } else {
            // 跳过标题行（如 【核心摘要】【摘要】等）
            if trimmed.starts_with("【") && trimmed.ends_with("】") {
                continue;
            }
            if trimmed == "核心摘要" || trimmed == "摘要" || trimmed == "总结" || trimmed == "Summary" {
                continue;
            }
            content_lines.push(trimmed.to_string());
        }
    }

    // summary = 所有内容行拼接
    let summary = if content_lines.is_empty() {
        // 兜底：取原始文本前200字
        let fallback: String = raw.chars().take(200).collect();
        fallback
    } else {
        content_lines.join("\n")
    };

    // 清理 summary：去除可能的残留标记
    let summary = summary.trim_matches(&['-', '·', '*', ' ', '\n'] as &[_]).to_string();

    let key_points: Vec<String> = if point_lines.is_empty() && !summary.is_empty() {
        // 如果没有明确的要点行，尝试从摘要中拆分句子作为要点
        let sentences: Vec<&str> = summary.split(['。', '！', '？', '\n'])
            .filter(|s| s.trim().len() > 6)
            .take(3)
            .collect();
        if sentences.len() >= 2 {
            sentences.iter().map(|s| s.trim().to_string()).collect()
        } else {
            vec!["AI 已完成智能总结".to_string()]
        }
    } else {
        point_lines.into_iter().take(3).collect()
    };

    let summary_len = summary.chars().count();
    let orig_len = request.content.chars().count();

    Ok(ApiResponse::success(KbSummarizeResult {
        summary,
        key_points: if key_points.is_empty() { vec!["AI 已总结".to_string()] } else { key_points },
        original_length: orig_len,
        summary_length: summary_len,
    }))
}

pub async fn kb_tags(
    request: KbTagRequest,
) -> Result<ApiResponse<KbTagResult>, AppError> {
    let existing_hint = if request.existing_tags.is_empty() {
        String::new()
    } else {
        format!("\n注意：已有标签：{}。请避免重复建议。", request.existing_tags.join(", "))
    };

    let system_prompt = "你是一个专业的中文知识管理助手。请根据内容智能推荐3-5个标签词。\
        仅返回标签词，每行一个，不要编号，不要解释。";

    let user_prompt = format!(
        "请根据以下内容推荐标签：\n\n{}\n{}",
        if request.content.len() > 3000 { &request.content[..3000] } else { &request.content },
        existing_hint
    );

    let result_text = call_llm_chat(
        &request.provider,
        &request.endpoint,
        request.api_key.as_deref(),
        request.model.as_deref(),
        system_prompt,
        &user_prompt,
    ).await?;

    let tags: Vec<String> = result_text
        .lines()
        .map(|l| l.trim().trim_matches(&['-', '·', '*', ' ', '#'] as &[_]).to_string())
        .filter(|t| !t.is_empty() && t.chars().count() <= 20)
        .take(5)
        .collect();

    Ok(ApiResponse::success(KbTagResult {
        suggested_tags: tags,
    }))
}

// ========== 可测试的辅助函数 ==========
// 注：这些函数当前仅在 #[cfg(test)] 模块中使用，但保留为 pub(crate) 以便未来底层智能 V5 复用

#[allow(dead_code)]
fn build_llm_url(provider: &str, endpoint: &str) -> String {
    let endpoint = endpoint.trim_end_matches('/');
    match provider.to_lowercase().as_str() {
        "ollama" => format!("{}/api/chat", endpoint),
        _ => format!("{}/chat/completions", endpoint),
    }
}

#[allow(dead_code)]
fn parse_summary_lines(text: &str) -> (String, Vec<String>) {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let summary = lines.first()
        .map(|s| s.trim_matches(&['-', '·', '*', ' '] as &[_]).to_string())
        .unwrap_or_default();
    let key_points: Vec<String> = lines.iter()
        .skip(1)
        .filter(|l| l.starts_with('-') || l.starts_with('·') || l.starts_with('*') || l.starts_with("要点") || l.starts_with("关键"))
        .map(|l| l.trim_matches(&['-', '·', '*', ' '] as &[_]).trim().to_string())
        .take(3)
        .collect();
    (summary, key_points)
}

#[allow(dead_code)]
fn parse_tag_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.trim().trim_matches(&['-', '·', '*', ' ', '#'] as &[_]).to_string())
        .filter(|t| !t.is_empty() && t.chars().count() <= 20)
        .take(5)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== build_llm_url =====

    #[test]
    fn test_ollama_url() {
        let url = build_llm_url("ollama", "http://localhost:11434");
        assert_eq!(url, "http://localhost:11434/api/chat");
    }

    #[test]
    fn test_ollama_url_trailing_slash() {
        let url = build_llm_url("ollama", "http://localhost:11434/");
        assert_eq!(url, "http://localhost:11434/api/chat");
    }

    #[test]
    fn test_openai_url() {
        let url = build_llm_url("openai", "https://api.openai.com/v1");
        assert_eq!(url, "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn test_custom_provider_url() {
        let url = build_llm_url("custom-provider", "https://my-llm.example.com");
        assert_eq!(url, "https://my-llm.example.com/chat/completions");
    }

    #[test]
    fn test_case_insensitive_provider() {
        let url = build_llm_url("OLLAMA", "http://localhost:11434");
        assert_eq!(url, "http://localhost:11434/api/chat");
    }

    // ===== parse_summary_lines =====

    #[test]
    fn test_parse_summary_basic() {
        let text = "这是AI生成的核心摘要。\n- 关键要点1\n- 关键要点2\n- 关键要点3";
        let (summary, points) = parse_summary_lines(text);
        assert_eq!(summary, "这是AI生成的核心摘要。");
        assert_eq!(points.len(), 3);
        assert_eq!(points[0], "关键要点1");
        assert_eq!(points[1], "关键要点2");
        assert_eq!(points[2], "关键要点3");
    }

    #[test]
    fn test_parse_summary_no_points() {
        let text = "只有一段摘要，没有要点。";
        let (summary, points) = parse_summary_lines(text);
        assert_eq!(summary, "只有一段摘要，没有要点。");
        assert!(points.is_empty());
    }

    #[test]
    fn test_parse_summary_dot_prefix_points() {
        let text = "核心摘要内容\n· 要点A\n· 要点B";
        let (summary, points) = parse_summary_lines(text);
        assert_eq!(summary, "核心摘要内容");
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn test_parse_summary_star_prefix() {
        let text = "总结内容\n* 第一点\n* 第二点\n* 第三点\n* 第四点";
        let (summary, points) = parse_summary_lines(text);
        assert_eq!(summary, "总结内容");
        assert_eq!(points.len(), 3); // capped at 3
    }

    #[test]
    fn test_parse_summary_keyword_prefix() {
        let text = "摘要\n要点1：内容\n关键要点2\n- 第三点";
        let (summary, points) = parse_summary_lines(text);
        assert_eq!(summary, "摘要");
        assert!(points.iter().any(|p| p.contains("要点1")));
        assert!(points.iter().any(|p| p.contains("关键要点2")));
    }

    #[test]
    fn test_parse_summary_empty() {
        let (summary, points) = parse_summary_lines("");
        assert!(summary.is_empty());
        assert!(points.is_empty());
    }

    #[test]
    fn test_parse_summary_whitespace_stripping() {
        let text = "  摘要内容  \n-   带空格的要点   ";
        let (summary, points) = parse_summary_lines(text);
        assert_eq!(summary, "摘要内容");
        assert_eq!(points[0], "带空格的要点");
    }

    // ===== parse_tag_lines =====

    #[test]
    fn test_parse_tags_basic() {
        let text = "Python\n机器学习\n深度学习\n数据科学";
        let tags = parse_tag_lines(text);
        assert_eq!(tags.len(), 4);
        assert_eq!(tags[0], "Python");
        assert_eq!(tags[1], "机器学习");
    }

    #[test]
    fn test_parse_tags_with_prefix() {
        let text = "- Python\n- 机器学习\n- AI\n- 编程";
        let tags = parse_tag_lines(text);
        assert_eq!(tags.len(), 4);
        assert_eq!(tags[0], "Python");
    }

    #[test]
    fn test_parse_tags_empty_lines() {
        let text = "标签1\n\n\n标签2\n";
        let tags = parse_tag_lines(text);
        assert_eq!(tags, vec!["标签1", "标签2"]);
    }

    #[test]
    fn test_parse_tags_truncate_long() {
        let text = "这是一个超级无敌长到超过20个字符限制的标签名应该是无法通过的\n短标签";
        let tags = parse_tag_lines(text);
        assert!(tags.iter().all(|t| t.chars().count() <= 20));
        assert_eq!(tags, vec!["短标签"]);
    }

    #[test]
    fn test_parse_tags_cap_at_5() {
        let text = "A\nB\nC\nD\nE\nF\nG";
        let tags = parse_tag_lines(text);
        assert_eq!(tags.len(), 5);
    }

    #[test]
    fn test_parse_tags_hash_stripping() {
        let text = "#Python\n#Rust\n#Go";
        let tags = parse_tag_lines(text);
        assert_eq!(tags[0], "Python");
        assert_eq!(tags[1], "Rust");
    }
}