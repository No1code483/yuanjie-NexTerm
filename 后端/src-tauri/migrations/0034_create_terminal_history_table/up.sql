-- Migration v34: create_terminal_history_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_terminal_history_table"（T2.6.1 兼容性设计）
-- 说明: 终端命令历史记录表，存储每条命令及其输出、退出码、耗时
--       注意：duration_ms 字段在 v34 中已包含，但 v35 仍为旧库（无 duration_ms）补字段

CREATE TABLE IF NOT EXISTS terminal_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    command TEXT NOT NULL,
    output TEXT,
    exit_code INTEGER NOT NULL DEFAULT 0,
    session_type TEXT NOT NULL DEFAULT 'builtin',
    duration_ms INTEGER,
    created_at INTEGER NOT NULL
);
