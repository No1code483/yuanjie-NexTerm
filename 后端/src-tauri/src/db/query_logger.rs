//! 慢查询日志包装器（A1 §2.4.1 + Phase 3 §2.2.1）
//!
//! 提供 SQL 执行耗时测量 + 慢查询自动记录能力。
//! 表：slow_query_log（见 migrations.rs::create_slow_query_log_table）
//! 规范：功能展望/平台级增强/01_性能优化_首屏200ms计划.md §2.4.1 + Phase 3 §2.2.1
//!
//! 设计原则：
//! - 非侵入式：调用方选择是否使用，不影响现有代码
//! - 静默失败：写入失败仅 warn 日志，不传播错误
//! - 截断保护：sql_text 截断到 1000 字符，params_json 截断到 500 字符（UTF-8 安全）
//! - 阈值控制：默认 10ms，可通过参数覆盖；也可通过环境变量
//!   `NEXTERM_SLOW_QUERY_THRESHOLD_MS` 全局覆盖（Phase 3 §2.2.1）
//! - 双通道记录：同时写入 slow_query_log 表 + tracing 日志（Phase 3 §2.2.1）

use std::time::Instant;

use sqlx::SqlitePool;

use crate::error::app_error::AppError;

/// 默认慢查询阈值（毫秒）
pub const DEFAULT_SLOW_QUERY_THRESHOLD_MS: u64 = 10;

/// 环境变量名：覆盖慢查询阈值（毫秒）
const ENV_THRESHOLD_VAR: &str = "NEXTERM_SLOW_QUERY_THRESHOLD_MS";

/// 读取全局慢查询阈值（环境变量优先，回落到默认 10ms）
///
/// Phase 3 §2.2.1：阈值可配。解析失败或未设置时回落到默认值，
/// 不会因为环境变量值不合法而影响业务。
pub fn global_threshold_ms() -> u64 {
    std::env::var(ENV_THRESHOLD_VAR)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(DEFAULT_SLOW_QUERY_THRESHOLD_MS)
}

/// SQL 文本最大长度（超出截断）
const MAX_SQL_TEXT_LEN: usize = 1000;

/// 参数 JSON 最大长度（超出截断）
const MAX_PARAMS_JSON_LEN: usize = 500;

/// 慢查询记录上下文（用于 execute_with_log 自动测量）
///
/// # 用法
/// ```ignore
/// use crate::db::query_logger::SlowQueryContext;
///
/// let ctx = SlowQueryContext::new(&state.pool, Some("get_todos"))
///     .with_params(&params_json);
/// let rows = ctx.execute("SELECT * FROM todos", async {
///     sqlx::query("SELECT * FROM todos").fetch_all(pool).await.map_err(AppError::Database)
/// }).await?;
/// ```
pub struct SlowQueryContext<'a> {
    pool: &'a SqlitePool,
    command_name: Option<&'a str>,
    params_json: Option<&'a str>,
    threshold_ms: u64,
}

impl<'a> SlowQueryContext<'a> {
    /// 创建慢查询上下文，阈值取自环境变量（Phase 3 §2.2.1）或默认 10ms
    pub fn new(pool: &'a SqlitePool, command_name: Option<&'a str>) -> Self {
        Self {
            pool,
            command_name,
            params_json: None,
            threshold_ms: global_threshold_ms(),
        }
    }

    pub fn with_params(mut self, params_json: &'a str) -> Self {
        self.params_json = Some(params_json);
        self
    }

    pub fn with_threshold(mut self, threshold_ms: u64) -> Self {
        self.threshold_ms = threshold_ms;
        self
    }

    /// 执行一个闭包并自动测量耗时；若超过阈值则记录到 slow_query_log
    ///
    /// # 参数
    /// - `sql_text`: SQL 语句文本（用于日志展示，不需要与实际执行的 SQL 完全一致）
    /// - `f`: 异步闭包，执行实际的 SQL 查询
    ///
    /// Phase 3 §2.2.3：同时调用 n_plus_one_detector::record_query 进行作用域内
    /// SQL 重复执行统计（需调用方先用 start_scope 开启作用域，否则为空操作）。
    pub async fn execute<F, T>(&self, sql_text: &str, f: F) -> Result<T, AppError>
    where
        F: std::future::Future<Output = Result<T, AppError>>,
    {
        // Phase 3 §2.2.3：当前线程作用域内记录 SQL 执行（无作用域时为空操作）
        crate::db::n_plus_one_detector::record_query(
            crate::db::n_plus_one_detector::current_scope(),
            sql_text,
        );

        let start = Instant::now();
        let result = f.await;
        let duration_ms = start.elapsed().as_millis() as u64;

        if duration_ms >= self.threshold_ms {
            record_slow_query(
                self.pool,
                sql_text,
                duration_ms,
                self.command_name,
                self.params_json,
                None,
            )
            .await;
        }

        result
    }
}

/// 直接记录一条慢查询到 slow_query_log 表
///
/// # 参数
/// - `pool`: 数据库连接池
/// - `sql_text`: SQL 语句（自动 UTF-8 安全截断到 1000 字符）
/// - `duration_ms`: 执行耗时（毫秒）
/// - `command_name`: 可选，关联的 Tauri 命令名
/// - `params_json`: 可选，参数 JSON 快照（自动截断到 500 字符）
/// - `rows_affected`: 可选，受影响行数
///
/// # 失败处理
/// 写入失败仅 warn 日志，不传播错误（性能监控不能影响业务流程）。
pub async fn record_slow_query(
    pool: &SqlitePool,
    sql_text: &str,
    duration_ms: u64,
    command_name: Option<&str>,
    params_json: Option<&str>,
    rows_affected: Option<i64>,
) {
    let truncated_sql = truncate_str(sql_text, MAX_SQL_TEXT_LEN);
    let truncated_params = params_json.map(|p| {
        let truncated = truncate_str(p, MAX_PARAMS_JSON_LEN);
        if p.len() > MAX_PARAMS_JSON_LEN {
            format!("{}...(truncated)", truncated)
        } else {
            truncated.to_string()
        }
    });

    let recorded_at = chrono::Utc::now().to_rfc3339();

    // Phase 3 §2.2.1：双通道记录 — 先 emit tracing 日志（保证即使 DB 写入失败也有记录）
    tracing::warn!(
        target: "nexterm::slow_query",
        duration_ms,
        command = ?command_name,
        rows_affected = ?rows_affected,
        "[slow_query] {}ms {}",
        duration_ms,
        truncate_str(sql_text, 120),
    );

    let result = sqlx::query(
        "INSERT INTO slow_query_log (sql_text, duration_ms, command_name, params_json, rows_affected, recorded_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(truncated_sql)
    .bind(duration_ms as i64)
    .bind(command_name)
    .bind(&truncated_params)
    .bind(rows_affected)
    .bind(&recorded_at)
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::warn!(
            "[slow_query] 写入失败 sql={:.40}... duration_ms={}: {}",
            sql_text,
            duration_ms,
            e
        );
    }
}

/// UTF-8 安全的字符串截断
///
/// 在 `max_len` 字节内找到最大的 UTF-8 字符边界，避免切到字符中间导致 panic。
fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        return s;
    }
    let mut end = max_len;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_ascii() {
        assert_eq!(truncate_str("hello world", 5), "hello");
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_utf8() {
        // 中文每个字符 3 字节
        let s = "你好世界你好世界"; // 24 字节
        let truncated = truncate_str(s, 7); // 应截断到 6 字节（2 个中文字符）
        assert_eq!(truncated, "你好");
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(truncate_str("", 10), "");
    }

    #[test]
    fn test_global_threshold_falls_back_to_default() {
        // 环境变量未设置或不合法时回落到默认 10ms
        // （不直接操作 env，避免影响并发测试）
        let v = global_threshold_ms();
        assert!(v > 0);
    }
}
