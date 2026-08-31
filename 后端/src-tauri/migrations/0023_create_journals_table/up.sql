-- Migration v23: create_journals_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_journals_table"（T2.6.1 兼容性设计）
-- 说明: 日记表，date 唯一约束

CREATE TABLE IF NOT EXISTS journals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL UNIQUE,
    content TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
