-- Migration v11: create_kb_tags_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_kb_tags_table"（T2.6.1 兼容性设计）
-- 说明: 知识库标签表，name 唯一约束

CREATE TABLE IF NOT EXISTS kb_tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL
);
