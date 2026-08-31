-- Migration v54: create_xin_moods_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_xin_moods_table"（T2.6.1 兼容性设计）
-- 说明: 小欣情绪状态表，每个 category 一行，存储 intensity / trigger / context

CREATE TABLE IF NOT EXISTS xin_moods (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category TEXT NOT NULL,
    intensity REAL NOT NULL DEFAULT 0.5,
    updated_at TEXT NOT NULL,
    trigger_text TEXT,
    context TEXT
);
