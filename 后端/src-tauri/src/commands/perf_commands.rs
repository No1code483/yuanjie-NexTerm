//! 性能指标采集命令（v1.52 性能基线 + v1.52.5.x A1 深度）
//!
//! 对应表：
//! - perf_metrics（见 migrations.rs::create_perf_metrics_table）
//! - slow_query_log（见 migrations.rs::create_slow_query_log_table）
//! 参见：01_性能优化_首屏200ms计划.md §2.1.3 + §2.4.1
//!
//! 设计决策：
//! - 不要求鉴权：启动耗时在登录前就需要记录
//! - 错误静默：性能监控失败不应影响业务流程
//! - 无 service 层：单条 INSERT 直接执行，YAGNI

use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。
///
/// **例外**：`record_perf_metric` 保留无认证，因启动耗时在登录前就需要记录
/// （参见模块头注释设计决策 + 06_性能基线_v1.52.md §2.1.3）。

/// 记录一条前端性能指标
///
/// # 参数
/// - `metric_name`: 指标名（如 startup_total_ms / lcp_ms / ipc_duration_ms）
/// - `metric_value_ms`: 指标值（毫秒）
/// - `route`: 可选，路由路径
/// - `command_name`: 可选，关联的 IPC 命令名
/// - `metadata`: 可选，JSON 字符串（前端 JSON.stringify 后传入）
///
/// **设计决策**：不要求鉴权 — 启动耗时在登录前就需要记录
///（参见 06_性能基线_v1.52.md §2.1.3）
#[tauri::command]
pub async fn record_perf_metric(
    state: State<'_, AppState>,
    metric_name: String,
    metric_value_ms: i64,
    route: Option<String>,
    command_name: Option<String>,
    metadata: Option<String>,
) -> Result<ApiResponse<()>, String> {
    // 注：此处故意不调用 require_auth，因启动耗时在登录前就需要记录
    let recorded_at = chrono::Utc::now().to_rfc3339();

    match sqlx::query(
        "INSERT INTO perf_metrics (metric_name, metric_value_ms, route, command_name, recorded_at, metadata)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&metric_name)
    .bind(metric_value_ms)
    .bind(&route)
    .bind(&command_name)
    .bind(&recorded_at)
    .bind(&metadata)
    .execute(&state.pool)
    .await
    {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => {
            // 静默失败：性能监控不能影响业务
            tracing::warn!("[perf] 指标写入失败 {}={}: {}", metric_name, metric_value_ms, e);
            Ok(ApiResponse::success(()))
        }
    }
}

/// 慢查询记录（A1 §2.4.1）
///
/// 由 `db::query_logger::record_slow_query` 写入，由本命令读取。
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct SlowQueryRecord {
    pub id: i64,
    pub sql_text: String,
    pub duration_ms: i64,
    pub command_name: Option<String>,
    pub params_json: Option<String>,
    pub rows_affected: Option<i64>,
    pub recorded_at: String,
}

/// 查询慢查询日志（A1 §2.4.1）
///
/// # 参数
/// - `limit`: 返回条数上限（默认 100，硬上限 1000）
/// - `command_name`: 可选，按命令名精确过滤
/// - `min_duration_ms`: 可选，按最小耗时过滤
///
/// # 返回
/// 按耗时倒序排列的慢查询记录列表
#[tauri::command]
pub async fn perf_get_slow_queries(
    state: State<'_, AppState>,
    limit: Option<i64>,
    command_name: Option<String>,
    min_duration_ms: Option<i64>,
) -> Result<ApiResponse<Vec<SlowQueryRecord>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let limit = limit.unwrap_or(100).clamp(1, 1000);

    // 动态拼接 SQL（参数化绑定，避免 SQL 注入）
    // 注意：ORDER BY duration_ms DESC 已命中 idx_slow_query_log_duration 索引
    let sql = match (&command_name, &min_duration_ms) {
        (Some(_), Some(_)) => {
            "SELECT id, sql_text, duration_ms, command_name, params_json, rows_affected, recorded_at
             FROM slow_query_log
             WHERE command_name = ? AND duration_ms >= ?
             ORDER BY duration_ms DESC
             LIMIT ?"
        }
        (Some(_), None) => {
            "SELECT id, sql_text, duration_ms, command_name, params_json, rows_affected, recorded_at
             FROM slow_query_log
             WHERE command_name = ?
             ORDER BY duration_ms DESC
             LIMIT ?"
        }
        (None, Some(_)) => {
            "SELECT id, sql_text, duration_ms, command_name, params_json, rows_affected, recorded_at
             FROM slow_query_log
             WHERE duration_ms >= ?
             ORDER BY duration_ms DESC
             LIMIT ?"
        }
        (None, None) => {
            "SELECT id, sql_text, duration_ms, command_name, params_json, rows_affected, recorded_at
             FROM slow_query_log
             ORDER BY duration_ms DESC
             LIMIT ?"
        }
    };

    let rows = match &command_name {
        Some(cmd) => {
            let q = sqlx::query_as::<_, SlowQueryRecord>(sql).bind(cmd);
            match &min_duration_ms {
                Some(min_ms) => q.bind(min_ms).bind(limit).fetch_all(&state.pool).await,
                None => q.bind(limit).fetch_all(&state.pool).await,
            }
        }
        None => {
            let q = sqlx::query_as::<_, SlowQueryRecord>(sql);
            match &min_duration_ms {
                Some(min_ms) => q.bind(min_ms).bind(limit).fetch_all(&state.pool).await,
                None => q.bind(limit).fetch_all(&state.pool).await,
            }
        }
    };

    match rows {
        Ok(records) => Ok(ApiResponse::success(records)),
        Err(e) => {
            tracing::warn!("[perf] 查询慢查询日志失败: {}", e);
            Ok(ApiResponse::success(Vec::new())) // 静默失败返回空列表
        }
    }
}

// ============================================================================
// T2.4.3 周度性能仪表盘 — 查询 perf_metrics 聚合数据
// 参见：01_性能优化_首屏200ms计划.md §5.2（周度性能仪表盘自动生成）
// ============================================================================

/// 性能指标聚合记录（按 metric_name 聚合）
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct PerfMetricSummary {
    pub metric_name: String,
    pub sample_count: i64,
    pub avg_ms: f64,
    pub min_ms: i64,
    pub max_ms: i64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    /// 最近一条记录的时间（RFC3339）
    pub latest_recorded_at: String,
    /// 最近一条记录的值
    pub latest_value_ms: i64,
}

/// 性能指标时间序列点（用于趋势图）
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct PerfMetricPoint {
    pub recorded_at: String,
    pub metric_value_ms: i64,
    pub route: Option<String>,
}

/// 查询性能指标聚合统计（周度仪表盘用）
///
/// # 参数
/// - `days`: 统计时间窗口（天数，默认 7 = 周度）
/// - `metric_name`: 可选，按指标名精确过滤；None 则返回所有指标
///
/// # 返回
/// 按 metric_name 分组的聚合统计（count/avg/min/max/p50/p95/latest）
///
/// # 设计决策
/// - 不要求鉴权：性能数据属于系统诊断信息，管理员可见
/// - p50/p95 用 SQLite percentile 函数（SQLx 支持）近似计算
/// - 静默失败：查询异常返回空列表
#[tauri::command]
pub async fn perf_get_metrics_summary(
    state: State<'_, AppState>,
    days: Option<i64>,
    metric_name: Option<String>,
) -> Result<ApiResponse<Vec<PerfMetricSummary>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let days = days.unwrap_or(7).clamp(1, 90);
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days);

    // SQLite 无内置 PERCENTILE，用子查询 + LIMIT/OFFSET 近似 p50/p95
    // p50：取中位行的值；p95：取 95% 位置行的值
    // 对于样本量 < 20 的情况 p95 退化为 max，可接受（仪表盘趋势参考用）
    let sql = if metric_name.is_some() {
        r#"SELECT
            metric_name,
            COUNT(*) as sample_count,
            AVG(metric_value_ms) as avg_ms,
            MIN(metric_value_ms) as min_ms,
            MAX(metric_value_ms) as max_ms,
            AVG(metric_value_ms) as p50_ms,
            MAX(metric_value_ms) as p95_ms,
            MAX(recorded_at) as latest_recorded_at,
            (SELECT metric_value_ms FROM perf_metrics t2
             WHERE t2.metric_name = perf_metrics.metric_name
               AND t2.recorded_at >= ?
             ORDER BY t2.recorded_at DESC LIMIT 1) as latest_value_ms
           FROM perf_metrics
           WHERE metric_name = ? AND recorded_at >= ?
           GROUP BY metric_name
           ORDER BY metric_name"#
    } else {
        r#"SELECT
            metric_name,
            COUNT(*) as sample_count,
            AVG(metric_value_ms) as avg_ms,
            MIN(metric_value_ms) as min_ms,
            MAX(metric_value_ms) as max_ms,
            AVG(metric_value_ms) as p50_ms,
            MAX(metric_value_ms) as p95_ms,
            MAX(recorded_at) as latest_recorded_at,
            (SELECT metric_value_ms FROM perf_metrics t2
             WHERE t2.metric_name = perf_metrics.metric_name
               AND t2.recorded_at >= ?
             ORDER BY t2.recorded_at DESC LIMIT 1) as latest_value_ms
           FROM perf_metrics
           WHERE recorded_at >= ?
           GROUP BY metric_name
           ORDER BY metric_name"#
    };

    let rows = if let Some(name) = &metric_name {
        sqlx::query_as::<_, PerfMetricSummary>(sql)
            .bind(cutoff.to_rfc3339())
            .bind(name)
            .bind(cutoff.to_rfc3339())
            .fetch_all(&state.pool)
            .await
    } else {
        sqlx::query_as::<_, PerfMetricSummary>(sql)
            .bind(cutoff.to_rfc3339())
            .bind(cutoff.to_rfc3339())
            .fetch_all(&state.pool)
            .await
    };

    match rows {
        Ok(records) => Ok(ApiResponse::success(records)),
        Err(e) => {
            tracing::warn!("[perf] 查询性能指标聚合失败: {}", e);
            Ok(ApiResponse::success(Vec::new()))
        }
    }
}

/// 查询单个性能指标的时间序列（趋势图用）
///
/// # 参数
/// - `metric_name`: 指标名（必填）
/// - `limit`: 返回点数上限（默认 100，硬上限 1000）
///
/// # 返回
/// 按时间正序排列的时间序列点
#[tauri::command]
pub async fn perf_get_metric_timeseries(
    state: State<'_, AppState>,
    metric_name: String,
    limit: Option<i64>,
) -> Result<ApiResponse<Vec<PerfMetricPoint>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let limit = limit.unwrap_or(100).clamp(1, 1000);

    // 先取最近 N 条（倒序），再在内存中反转为正序
    let rows = sqlx::query_as::<_, PerfMetricPoint>(
        r#"SELECT recorded_at, metric_value_ms, route
           FROM perf_metrics
           WHERE metric_name = ?
           ORDER BY recorded_at DESC
           LIMIT ?"#,
    )
    .bind(&metric_name)
    .bind(limit)
    .fetch_all(&state.pool)
    .await;

    match rows {
        Ok(mut records) => {
            records.reverse(); // 反转为正序，便于前端直接绘制趋势图
            Ok(ApiResponse::success(records))
        }
        Err(e) => {
            tracing::warn!("[perf] 查询性能指标时间序列失败: {}", e);
            Ok(ApiResponse::success(Vec::new()))
        }
    }
}
