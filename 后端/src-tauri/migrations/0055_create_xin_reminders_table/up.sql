-- Migration v55: create_xin_reminders_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_xin_reminders_table"（T2.6.1 兼容性设计）
-- 说明: 小欣提醒表，支持 once / cron 两种类型，记录触发时间与重复计数

CREATE TABLE IF NOT EXISTS xin_reminders (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    reminder_type TEXT NOT NULL DEFAULT 'once',
    trigger_at TEXT,
    cron_expression TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    last_triggered_at TEXT,
    repeat_count INTEGER NOT NULL DEFAULT 0
);
