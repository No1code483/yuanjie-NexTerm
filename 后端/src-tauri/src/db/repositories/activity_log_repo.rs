use sqlx::{QueryBuilder, SqlitePool};

use crate::error::app_error::AppError;
use crate::models::intelligence_cross_module::{
    ActivityLog, ActivityStats, DailyTrend, HourlyActivity, RealtimeStats, TopFile,
};

pub async fn insert_log(
    pool: &SqlitePool,
    user_id: &str,
    timestamp: &str,
    module: &str,
    operation: &str,
    detail: Option<&str>,
    remark: Option<&str>,
    duration_secs: i64,
) -> Result<ActivityLog, AppError> {
    sqlx::query_as::<_, ActivityLog>(
        "INSERT INTO activity_logs (user_id, timestamp, module, operation, detail, remark, duration_secs)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(user_id)
    .bind(timestamp)
    .bind(module)
    .bind(operation)
    .bind(detail)
    .bind(remark)
    .bind(duration_secs)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn batch_insert_logs(
    pool: &SqlitePool,
    records: &[(String, String, String, String, Option<String>, Option<String>, i64)],
) -> Result<(), AppError> {
    for (user_id, timestamp, module, operation, detail, remark, duration_secs) in records {
        sqlx::query(
            "INSERT INTO activity_logs (user_id, timestamp, module, operation, detail, remark, duration_secs)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(user_id)
        .bind(timestamp)
        .bind(module)
        .bind(operation)
        .bind(detail)
        .bind(remark)
        .bind(duration_secs)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    }
    Ok(())
}

/// 查询活动日志（参数化查询，防 SQL 注入）
///
/// T2.11 重构：原实现使用 `format!` 拼接用户输入 + 单引号转义，存在 SQL 注入风险。
/// 新实现使用 `sqlx::QueryBuilder` 的 `push_bind` 自动参数化绑定。
///
/// 测试覆盖：`test_query_logs_sql_injection_safe` 验证常见 SQL 注入 payload 被正确处理。
pub async fn query_logs(
    pool: &SqlitePool,
    user_id: Option<&str>,
    module: Option<&str>,
    start_time: Option<&str>,
    end_time: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<(Vec<ActivityLog>, i64), AppError> {
    // 使用 QueryBuilder 构建动态 WHERE 子句，所有用户输入通过 push_bind 参数化绑定
    let mut count_qb = QueryBuilder::new("SELECT COUNT(*) FROM activity_logs WHERE 1=1");
    if let Some(uid) = user_id {
        count_qb.push(" AND user_id = ").push_bind(uid);
    }
    if let Some(m) = module {
        count_qb.push(" AND module = ").push_bind(m);
    }
    if let Some(start) = start_time {
        count_qb.push(" AND timestamp >= ").push_bind(start);
    }
    if let Some(end) = end_time {
        count_qb.push(" AND timestamp <= ").push_bind(end);
    }

    let total: i64 = count_qb
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    // 数据查询：复用相同的 WHERE 条件 + ORDER BY + LIMIT/OFFSET
    let mut data_qb = QueryBuilder::new("SELECT * FROM activity_logs WHERE 1=1");
    if let Some(uid) = user_id {
        data_qb.push(" AND user_id = ").push_bind(uid);
    }
    if let Some(m) = module {
        data_qb.push(" AND module = ").push_bind(m);
    }
    if let Some(start) = start_time {
        data_qb.push(" AND timestamp >= ").push_bind(start);
    }
    if let Some(end) = end_time {
        data_qb.push(" AND timestamp <= ").push_bind(end);
    }
    data_qb.push(" ORDER BY timestamp DESC LIMIT ").push_bind(limit);
    data_qb.push(" OFFSET ").push_bind(offset);

    let logs = data_qb
        .build_query_as::<ActivityLog>()
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

    Ok((logs, total))
}

pub async fn get_stats(
    pool: &SqlitePool,
    user_id: &str,
    period: &str,
) -> Result<ActivityStats, AppError> {
    // 统一用 localtime 转换，与 resolve_date_filter 保持一致
    let date_filter = match period {
        "today" => "datetime(timestamp, 'localtime') >= datetime('now', 'localtime', 'start of day')",
        "yesterday" => "datetime(timestamp, 'localtime') >= datetime('now', 'localtime', '-1 day', 'start of day') \
                       AND datetime(timestamp, 'localtime') < datetime('now', 'localtime', 'start of day')",
        "week" => "datetime(timestamp, 'localtime') >= datetime('now', 'localtime', '-7 days', 'start of day')",
        "month" => "datetime(timestamp, 'localtime') >= datetime('now', 'localtime', '-30 days', 'start of day')",
        _ => "1=1",
    };

    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM activity_logs WHERE user_id = ? AND {}",
        date_filter
    ))
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let by_module_raw: Vec<(String, i64)> =
        sqlx::query_as::<_, (String, i64)>(&format!(
            "SELECT module, COUNT(*) as cnt FROM activity_logs WHERE user_id = ? AND {}
             GROUP BY module ORDER BY cnt DESC",
            date_filter
        ))
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    let by_hour_raw: Vec<(String, i64)> =
        sqlx::query_as::<_, (String, i64)>(&format!(
            "SELECT strftime('%H', timestamp) as hour, COUNT(*) as cnt FROM activity_logs
             WHERE user_id = ? AND {} GROUP BY hour ORDER BY hour",
            date_filter
        ))
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    let by_day_raw: Vec<(String, i64)> =
        sqlx::query_as::<_, (String, i64)>(&format!(
            "SELECT date(timestamp) as day, COUNT(*) as cnt FROM activity_logs
             WHERE user_id = ? AND {} GROUP BY day ORDER BY day",
            date_filter
        ))
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    let by_operation_raw: Vec<(String, i64)> =
        sqlx::query_as::<_, (String, i64)>(&format!(
            "SELECT operation, COUNT(*) as cnt FROM activity_logs WHERE user_id = ? AND {}
             GROUP BY operation ORDER BY cnt DESC",
            date_filter
        ))
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    Ok(ActivityStats {
        total_operations: total,
        by_module: by_module_raw,
        by_operation: by_operation_raw,
        by_hour: by_hour_raw,
        by_day: by_day_raw,
        period: period.to_string(),
    })
}

pub async fn cleanup_old_logs(pool: &SqlitePool, retention_days: i64) -> Result<u64, AppError> {
    let result = sqlx::query(
        "DELETE FROM activity_logs WHERE created_at < datetime('now', ? || ' days')",
    )
    .bind(format!("-{}", retention_days))
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(result.rows_affected())
}

/// 安全审计修复（IDOR）：原 `DELETE FROM activity_logs` 无 user_id 过滤，
/// 任意登录用户可清除全部用户的活动日志。现改为按当前 user_id 过滤。
pub async fn clear_all_logs(pool: &SqlitePool, user_id: &str) -> Result<u64, AppError> {
    let result = sqlx::query("DELETE FROM activity_logs WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(result.rows_affected())
}

pub async fn get_dashboard_active_duration(
    pool: &SqlitePool,
    user_id: &str,
    date_filter: &str,
) -> Result<i64, AppError> {
    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COALESCE(SUM(duration_secs), 0) FROM activity_logs WHERE user_id = ? AND {}",
        date_filter
    ))
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    Ok(total)
}

pub async fn get_dashboard_top_files(
    pool: &SqlitePool,
    user_id: &str,
    date_filter: &str,
    limit: i64,
) -> Result<Vec<TopFile>, AppError> {
    let rows = sqlx::query_as::<_, (String, i64, i64)>(&format!(
        "SELECT COALESCE(detail, '未知') as file_name, COUNT(*) as access_count,
                COALESCE(SUM(duration_secs), 0) as total_duration
         FROM activity_logs
         WHERE user_id = ? AND {} AND detail IS NOT NULL AND detail != ''
         GROUP BY detail
         ORDER BY access_count DESC
         LIMIT ?",
        date_filter
    ))
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Ok(rows
        .into_iter()
        .map(|(name, count, _dur)| TopFile {
            file_name: name,
            kb_path: String::new(),
            local_path: String::new(),
            access_count: count,
        })
        .collect())
}

pub async fn get_dashboard_hourly_heatmap(
    pool: &SqlitePool,
    user_id: &str,
    date_filter: &str,
) -> Result<Vec<HourlyActivity>, AppError> {
    // 统一用 localtime 转换：前端存的是 UTC (toISOString)，
    // strftime + localtime 自动转为系统本地时间的小时
    let rows = sqlx::query_as::<_, (String, i64)>(&format!(
        "SELECT strftime('%H:00', timestamp, 'localtime') as hour, COUNT(*) as cnt
         FROM activity_logs WHERE user_id = ? AND {}
         GROUP BY hour ORDER BY hour",
        date_filter
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let labels = [
        "凌晨", "凌晨", "凌晨", "凌晨", "凌晨", "凌晨",
        "上午", "上午", "上午", "上午", "上午", "上午",
        "下午", "下午", "下午", "下午", "下午", "下午",
        "晚上", "晚上", "晚上", "晚上", "晚上", "晚上",
    ];

    Ok(rows
        .into_iter()
        .map(|(hour, count)| {
            let h_idx = hour[..2].parse::<usize>().unwrap_or(0);
            HourlyActivity {
                hour,
                count,
                label: labels.get(h_idx).unwrap_or(&"未知").to_string(),
            }
        })
        .collect())
}

pub async fn get_dashboard_daily_trend(
    pool: &SqlitePool,
    user_id: &str,
    date_filter: &str,
) -> Result<Vec<DailyTrend>, AppError> {
    let rows = sqlx::query_as::<_, (String, i64, i64)>(&format!(
        "SELECT date(timestamp) as day, COALESCE(SUM(duration_secs), 0) as active_secs,
                COUNT(*) as ops
         FROM activity_logs WHERE user_id = ? AND {}
         GROUP BY day ORDER BY day",
        date_filter
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Ok(rows
        .into_iter()
        .map(|(date, secs, ops)| DailyTrend {
            date,
            active_secs: secs,
            operation_count: ops,
        })
        .collect())
}

pub async fn get_realtime_stats_for_today(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<RealtimeStats, AppError> {
    // 统一用 localtime 过滤，与 get_dashboard 的 resolve_date_filter('day') 保持一致
    let today_filter = "datetime(timestamp, 'localtime') >= datetime('now', 'localtime', 'start of day')";

    let active_duration: i64 = sqlx::query_scalar(&format!(
        "SELECT COALESCE(SUM(duration_secs), 0) FROM activity_logs
         WHERE user_id = ? AND {}", today_filter
    ))
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let operations: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND {}", today_filter
    ))
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let modules: Vec<String> = sqlx::query_scalar(&format!(
        "SELECT DISTINCT module FROM activity_logs
         WHERE user_id = ? AND {}", today_filter
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let last_activity: Option<String> = sqlx::query_scalar(
        "SELECT timestamp FROM activity_logs
         WHERE user_id = ? ORDER BY timestamp DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let current_focus_module: Option<String> = sqlx::query_scalar(&format!(
        "SELECT module FROM activity_logs WHERE user_id = ? AND {}
         GROUP BY module ORDER BY SUM(duration_secs) DESC LIMIT 1", today_filter
    ))
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let todo_total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM todos WHERE date = date('now')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let todo_completed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM todos WHERE date = date('now') AND completed = 1",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let todo_ratio = if todo_total > 0 {
        todo_completed as f64 / todo_total as f64
    } else {
        1.0
    };

    let kb_switches: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM activity_logs
         WHERE user_id = ? AND {} AND module = 'knowledge_base'
         AND operation LIKE '%打开%'", today_filter
    ))
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let kb_regularity = if kb_switches > 10 {
        0.3
    } else if kb_switches > 5 {
        0.6
    } else {
        1.0
    };

    let productive_secs = active_duration.saturating_sub(900);
    let focus_ratio = if active_duration > 0 {
        productive_secs as f64 / active_duration as f64
    } else {
        0.0
    };

    let score = focus_ratio * 0.4 + todo_ratio * 0.35 + kb_regularity * 0.25;

    Ok(RealtimeStats {
        active_duration_today_secs: active_duration,
        operations_today: operations,
        current_productivity_score: (score * 100.0).round(),
        modules_used_today: modules.len(),
        current_focus_module,
        last_activity_at: last_activity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use sqlx::SqlitePool;

    /// 创建内存数据库 + activity_logs 表的测试连接池
    ///
    /// 注意：SqlitePool 在 :memory: 模式下每个连接独立，因此使用 shared_cache
    /// 或限制为单连接，确保 schema 在所有查询中可用。
    async fn setup_test_pool() -> SqlitePool {
        let options = SqliteConnectOptions::new()
            .filename(":memory:")
            .create_if_missing(true)
            .shared_cache(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();
        sqlx::query(
            r#"CREATE TABLE activity_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                module TEXT NOT NULL,
                operation TEXT NOT NULL,
                detail TEXT,
                remark TEXT,
                duration_secs INTEGER DEFAULT 0,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    /// T2.11 SQL 注入测试：验证经典注入 payload 被参数化查询正确处理
    #[tokio::test]
    async fn test_query_logs_sql_injection_safe() {
        let pool = setup_test_pool().await;

        // 插入正常数据
        insert_log(&pool, "user1", "2026-07-20T10:00:00Z", "kb", "open", None, None, 10)
            .await
            .unwrap();

        // 测试经典 SQL 注入 payload
        let injection_payloads = [
            "' OR '1'='1",                       // 经典 OR 注入
            "'; DROP TABLE activity_logs; --",   // 堆叠注入
            "' UNION SELECT * FROM users; --",   // UNION 注入
            "admin'--",                           // 注释绕过
            "' OR 1=1; --",                       // 数值型 OR
            "user1' AND '1'='1",                  // 布尔盲注
        ];

        for payload in &injection_payloads {
            // 使用注入 payload 作为 user_id 查询
            let (logs, total) = query_logs(&pool, Some(payload), None, None, None, 100, 0)
                .await
                .unwrap();

            // 预期：参数化查询将整个 payload 作为字符串字面值处理，
            // 不会匹配任何记录，且不会执行恶意 SQL
            assert_eq!(
                total, 0,
                "注入 payload '{}' 不应匹配任何记录",
                payload
            );
            assert_eq!(
                logs.len(),
                0,
                "注入 payload '{}' 不应返回任何日志",
                payload
            );

            // 关键断言：activity_logs 表仍存在（未被 DROP）
            let table_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='activity_logs';")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(table_count, 1, "注入 payload '{}' 后表仍应存在", payload);
        }
    }

    /// T2.11 SQL 注入测试：验证正常查询仍能返回结果
    #[tokio::test]
    async fn test_query_logs_normal_query() {
        let pool = setup_test_pool().await;

        // 插入多条数据
        insert_log(&pool, "user1", "2026-07-20T10:00:00Z", "kb", "open", None, None, 10)
            .await
            .unwrap();
        insert_log(&pool, "user1", "2026-07-20T11:00:00Z", "chat", "send", None, None, 20)
            .await
            .unwrap();
        insert_log(&pool, "user2", "2026-07-20T12:00:00Z", "kb", "open", None, None, 15)
            .await
            .unwrap();

        // 查询 user1 的日志
        let (logs, total) = query_logs(&pool, Some("user1"), None, None, None, 100, 0)
            .await
            .unwrap();

        assert_eq!(total, 2, "user1 应有 2 条日志");
        assert_eq!(logs.len(), 2, "应返回 2 条日志");

        // 按 module 过滤
        let (logs, total) = query_logs(&pool, Some("user1"), Some("kb"), None, None, 100, 0)
            .await
            .unwrap();
        assert_eq!(total, 1, "user1 + kb 应有 1 条日志");
        assert_eq!(logs.len(), 1);

        // 按时间范围过滤（>= 11:00:00 AND <= 12:00:00，匹配 11:00 和 12:00 两条）
        let (logs, total) = query_logs(
            &pool,
            None,
            None,
            Some("2026-07-20T11:00:00Z"),
            Some("2026-07-20T12:00:00Z"),
            100,
            0,
        )
        .await
        .unwrap();
        assert_eq!(total, 2, "时间范围 11:00-12:00 应有 2 条日志");
        assert_eq!(logs.len(), 2);

        // 分页
        let (logs, total) = query_logs(&pool, None, None, None, None, 1, 0)
            .await
            .unwrap();
        assert_eq!(total, 3, "总记录数应为 3");
        assert_eq!(logs.len(), 1, "分页应返回 1 条");
    }

    /// T2.11 SQL 注入测试：验证特殊字符的 user_id 仍可正常查询
    #[tokio::test]
    async fn test_query_logs_special_chars_in_user_id() {
        let pool = setup_test_pool().await;

        // 插入含特殊字符的 user_id
        // 注意：实际应用中 user_id 应为纯字母数字，但参数化查询应能正确处理任意字符串
        insert_log(&pool, "user'with'quotes", "2026-07-20T10:00:00Z", "kb", "open", None, None, 10)
            .await
            .unwrap();

        // 查询该 user_id（应能正确匹配，不会因引号被转义而失败）
        let (logs, total) = query_logs(&pool, Some("user'with'quotes"), None, None, None, 100, 0)
            .await
            .unwrap();

        assert_eq!(total, 1, "含引号的 user_id 应能匹配");
        assert_eq!(logs.len(), 1);
    }
}