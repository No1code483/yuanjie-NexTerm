-- Migration v48: create_yuan_code_snippets_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_yuan_code_snippets_table"（T2.6.1 兼容性设计）
-- 说明: 元·代码片段表，存储用户保存的代码片段（名称、语言、代码、标签等）

CREATE TABLE IF NOT EXISTS yuan_code_snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    language TEXT NOT NULL,
    code TEXT NOT NULL,
    description TEXT,
    tags TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
