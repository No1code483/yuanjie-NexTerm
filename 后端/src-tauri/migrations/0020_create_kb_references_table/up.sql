-- Migration v20: create_kb_references_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_kb_references_table"（T2.6.1 兼容性设计）
-- 说明: 知识库条目间引用关系表，source→target 二元唯一约束

CREATE TABLE IF NOT EXISTS kb_references (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_entry_id INTEGER NOT NULL,
    target_entry_id INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(source_entry_id, target_entry_id)
);
