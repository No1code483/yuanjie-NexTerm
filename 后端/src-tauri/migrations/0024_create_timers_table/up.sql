-- Migration v24: create_timers_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_timers_table"（T2.6.1 兼容性设计）
-- 说明: 计时器表，支持 countdown 类型

CREATE TABLE IF NOT EXISTS timers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT,
    type TEXT NOT NULL DEFAULT 'countdown',
    target_time INTEGER,
    is_running INTEGER NOT NULL DEFAULT 0,
    elapsed INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
