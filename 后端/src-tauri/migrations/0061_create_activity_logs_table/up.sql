-- Migration v61: create_activity_logs_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + 内部调用 create_activity_logs_view 创建视图）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_activity_logs_table"（T2.6.1 兼容性设计）
-- 说明: 用户行为日志主表。本 .sql 仅包含 CREATE TABLE 静态部分；
--       日汇总视图 v_daily_activity_summary 由 inline 函数 create_activity_logs_view 创建。

CREATE TABLE IF NOT EXISTS activity_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    module TEXT NOT NULL,
    operation TEXT NOT NULL,
    detail TEXT,
    remark TEXT,
    duration_secs INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
