-- Migration v21: create_kb_snapshots_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_kb_snapshots_table"（T2.6.1 兼容性设计）
-- 说明: 知识库条目内容快照表，用于版本历史

CREATE TABLE IF NOT EXISTS kb_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entry_id INTEGER NOT NULL,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
