//! N+1 查询检测器（Phase 3 §2.2.3）
//!
//! 规范：功能展望/平台级增强/01_性能优化_首屏200ms计划.md §Phase 3
//!
//! 设计目标：
//! - 在请求/任务作用域内统计相同 SQL（按 normalized key）的执行次数
//! - 超过阈值（默认 5 次）时通过 tracing::warn! 记录疑似 N+1 查询
//! - 非侵入式：仅在显式开启作用域时生效，关闭后零开销
//! - 不阻塞业务：所有错误路径仅 warn，不传播
//!
//! 使用方式（推荐在 Tauri command 入口处包裹）：
//! ```ignore
//! use crate::db::n_plus_one_detector::{start_scope, record_query, end_scope};
//!
//! let scope = start_scope("list_todos_with_tags");
//! // ... 业务逻辑中每次执行 SQL 调用 record_query(sql_text)
//! end_scope(scope);  // 输出疑似 N+1 警告
//! ```
//!
//! 也可配合 SlowQueryContext 使用：在 SlowQueryContext::execute 内自动调用 record_query。

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use once_cell::sync::Lazy;

/// 默认 N+1 触发阈值：同一 SQL 在一个作用域内执行超过此次数则告警
pub const DEFAULT_N_PLUS_ONE_THRESHOLD: usize = 5;

/// 环境变量名：覆盖 N+1 触发阈值
const ENV_THRESHOLD_VAR: &str = "NEXTERM_N_PLUS_ONE_THRESHOLD";

/// 全局作用域计数器：用于生成唯一 scope_id
static SCOPE_SEQ: AtomicUsize = AtomicUsize::new(0);

/// 全局 N+1 检测开关（默认开启，可通过 set_enabled(false) 关闭）
static ENABLED: AtomicUsize = AtomicUsize::new(1);

/// 作用域内的 SQL 执行计数表
///
/// Key: normalized SQL（去除绑定参数后的 SQL 模板）
/// Value: 执行次数
type ScopeCounters = HashMap<String, usize>;

/// 全局作用域注册表：scope_id → (label, ScopeCounters)
static SCOPES: Lazy<Mutex<HashMap<usize, (String, ScopeCounters)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// 当前线程激活的作用域 ID（用于 SlowQueryContext::execute 自动记录）
//
// 调用 `enter_scope(label)` 推入，`exit_scope()` 弹出；支持嵌套（栈结构）。
thread_local! {
    static CURRENT_SCOPE_STACK: RefCell<Vec<usize>> = RefCell::new(Vec::new());
}

/// 获取当前线程激活的作用域 ID（栈顶；无作用域返回 0）
pub fn current_scope() -> usize {
    CURRENT_SCOPE_STACK.with(|s| s.borrow().last().copied().unwrap_or(0))
}

/// 进入一个 N+1 检测作用域（线程内栈式嵌套）
///
/// 等价于 `start_scope` + 推入线程栈。返回 scope_id，传入 `exit_scope` 弹出。
/// 在 Tauri command 入口或 service 调用边界使用：
/// ```ignore
/// let sid = enter_scope("list_todos_with_tags");
/// // ... 业务逻辑（SlowQueryContext::execute 会自动 record_query）
/// exit_scope(sid);
/// ```
pub fn enter_scope(label: &str) -> usize {
    let sid = start_scope(label);
    CURRENT_SCOPE_STACK.with(|s| s.borrow_mut().push(sid));
    sid
}

/// 退出当前作用域（栈顶弹出并 end_scope）
///
/// 传入的 sid 仅做校验（若与栈顶不匹配，仍弹栈但记录 warn）。
pub fn exit_scope(expected_sid: usize) {
    let popped = CURRENT_SCOPE_STACK.with(|s| s.borrow_mut().pop());
    match popped {
        Some(sid) => {
            if sid != expected_sid {
                tracing::warn!(
                    "[N+1] exit_scope 期望 {} 实际 {}：作用域栈可能错配",
                    expected_sid,
                    sid
                );
            }
            end_scope(sid);
        }
        None => {
            // 重复 exit：忽略
        }
    }
}

/// 读取全局 N+1 触发阈值（环境变量优先）
pub fn global_threshold() -> usize {
    std::env::var(ENV_THRESHOLD_VAR)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(DEFAULT_N_PLUS_ONE_THRESHOLD)
}

/// 开启 / 关闭 N+1 检测（默认开启）
///
/// 关闭后 start_scope / record_query / end_scope 均为空操作。
pub fn set_enabled(enabled: bool) {
    ENABLED.store(if enabled { 1 } else { 0 }, Ordering::Relaxed);
}

fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed) == 1
}

/// 开启一个 N+1 检测作用域
///
/// 返回 scope_id，传入 end_scope 完成检测。
/// 若检测被关闭（set_enabled(false)），返回 0 表示空作用域。
pub fn start_scope(label: &str) -> usize {
    if !is_enabled() {
        return 0;
    }
    let id = SCOPE_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    if let Ok(mut scopes) = SCOPES.lock() {
        scopes.insert(id, (label.to_string(), HashMap::new()));
    }
    id
}

/// 记录一次 SQL 执行
///
/// 对 SQL 做简单归一化（把 `?` 占位符视为同一查询）后累加计数。
/// 在作用域内调用；若 scope_id = 0 或作用域不存在，则为空操作。
pub fn record_query(scope_id: usize, sql_text: &str) {
    if scope_id == 0 || !is_enabled() {
        return;
    }
    let key = normalize_sql(sql_text);
    if let Ok(mut scopes) = SCOPES.lock() {
        if let Some((_, counters)) = scopes.get_mut(&scope_id) {
            *counters.entry(key).or_insert(0) += 1;
        }
    }
}

/// 结束 N+1 检测作用域
///
/// 输出超过阈值的 SQL 列表，然后清理作用域数据。
pub fn end_scope(scope_id: usize) {
    if scope_id == 0 || !is_enabled() {
        return;
    }
    let threshold = global_threshold();
    let mut maybe_data = None;
    if let Ok(mut scopes) = SCOPES.lock() {
        maybe_data = scopes.remove(&scope_id);
    }
    if let Some((label, counters)) = maybe_data {
        let offenders: Vec<(&String, &usize)> =
            counters.iter().filter(|(_, &c)| c > threshold).collect();
        if !offenders.is_empty() {
            tracing::warn!(
                target: "nexterm::n_plus_one",
                scope = %label,
                threshold,
                "[N+1] 疑似 N+1 查询：作用域 '{}' 内以下 SQL 执行次数超过阈值 {}",
                label,
                threshold,
            );
            for (sql, count) in offenders {
                tracing::warn!(
                    target: "nexterm::n_plus_one",
                    count,
                    "[N+1] {} 次执行: {}",
                    count,
                    sql,
                );
            }
        }
    }
}

/// SQL 归一化：将绑定参数占位符统一为标准形式
///
/// 当前实现：保留 `?` 占位符不变，仅 trim + lowercase 关键字部分，
/// 把连续空白合并为单空格。这样 `SELECT * FROM todos WHERE id = ?`
/// 和 `SELECT  *  FROM todos WHERE id = ?` 视为同一查询。
fn normalize_sql(sql: &str) -> String {
    let trimmed = sql.trim();
    let mut out = String::with_capacity(trimmed.len());
    let mut prev_space = false;
    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_collapses_whitespace() {
        let n = normalize_sql("SELECT   *   FROM   todos   WHERE   id = ?");
        assert_eq!(n, "SELECT * FROM todos WHERE id = ?");
    }

    #[test]
    fn test_normalize_preserves_placeholders() {
        let n = normalize_sql("INSERT INTO t (a, b) VALUES (?, ?)");
        assert_eq!(n, "INSERT INTO t (a, b) VALUES (?, ?)");
    }

    #[test]
    fn test_scope_records_and_detects() {
        set_enabled(true);
        let sid = start_scope("test_scope");
        assert_ne!(sid, 0);
        // 同一 SQL 执行 6 次（默认阈值 5）
        for _ in 0..6 {
            record_query(sid, "SELECT * FROM todos WHERE id = ?");
        }
        // 不同 SQL 执行 2 次（不触发）
        record_query(sid, "SELECT * FROM users WHERE id = ?");
        record_query(sid, "SELECT * FROM users WHERE id = ?");

        // end_scope 应清理作用域（不 panic 即可，tracing 输出由测试框架捕获）
        end_scope(sid);

        // 再次 end 同一 scope 应为空操作
        end_scope(sid);
    }

    #[test]
    fn test_disabled_is_noop() {
        set_enabled(false);
        let sid = start_scope("disabled_scope");
        assert_eq!(sid, 0);
        record_query(sid, "SELECT 1");
        end_scope(sid);
        set_enabled(true); // 恢复全局开关
    }

    #[test]
    fn test_global_threshold_parses_or_falls_back() {
        let v = global_threshold();
        assert!(v > 0);
    }

    #[test]
    fn test_enter_exit_scope_thread_local() {
        set_enabled(true);
        // 无作用域时 current_scope = 0
        assert_eq!(current_scope(), 0);

        let sid = enter_scope("thread_local_test");
        assert_ne!(sid, 0);
        assert_eq!(current_scope(), sid);

        // 嵌套：再 enter 一个
        let sid2 = enter_scope("nested");
        assert_eq!(current_scope(), sid2);

        exit_scope(sid2);
        assert_eq!(current_scope(), sid);

        exit_scope(sid);
        assert_eq!(current_scope(), 0);
    }
}
