-- Migration v29: create_quotes_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_quotes_table"（T2.6.1 兼容性设计）
-- 说明: 名言/语录表，默认 daily 类型

CREATE TABLE IF NOT EXISTS quotes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL,
    source TEXT,
    type TEXT NOT NULL DEFAULT 'daily'
);
