-- Migration v10: create_kb_entries_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_kb_entries_table"（T2.6.1 兼容性设计）
-- 说明: 知识库条目主表，FK 引用 kb_categories(id) ON DELETE CASCADE

CREATE TABLE IF NOT EXISTS kb_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    path_url TEXT NOT NULL,
    entry_type TEXT NOT NULL DEFAULT 'file',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (category_id) REFERENCES kb_categories(id) ON DELETE CASCADE
);
