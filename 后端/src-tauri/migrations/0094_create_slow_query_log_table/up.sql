-- A1 §2.4.1 慢查询日志表（v1.52.5.x 性能优化深度）
-- 创建日期：2026-07-22
-- 用途：记录执行耗时超过阈值（默认 10ms）的 SQL 语句，用于性能分析
-- 规范：功能展望/平台级增强/01_性能优化_首屏200ms计划.md §2.4.1

CREATE TABLE IF NOT EXISTS slow_query_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sql_text TEXT NOT NULL,                    -- SQL 语句（调用方负责截断到 1000 字符）
    duration_ms INTEGER NOT NULL,              -- 执行耗时（毫秒）
    command_name TEXT,                         -- 关联的 Tauri 命令名（可选）
    params_json TEXT,                          -- 参数 JSON 快照（可选，截断到 500 字符）
    rows_affected INTEGER,                     -- 受影响行数（可选）
    recorded_at TEXT NOT NULL                  -- ISO8601 时间戳
);

-- 按时间倒序索引（查询最近慢查询）
CREATE INDEX IF NOT EXISTS idx_slow_query_log_time ON slow_query_log(recorded_at DESC);

-- 按耗时倒序索引（查询最慢的 N 条）
CREATE INDEX IF NOT EXISTS idx_slow_query_log_duration ON slow_query_log(duration_ms DESC);
