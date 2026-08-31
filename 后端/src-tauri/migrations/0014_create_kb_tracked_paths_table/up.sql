-- Migration v14: create_kb_tracked_paths_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_kb_tracked_paths_table"（T2.6.1 兼容性设计）
-- 说明: 知识库追踪路径表，path 唯一约束 + FK 引用 kb_categories(id) ON DELETE CASCADE

CREATE TABLE IF NOT EXISTS kb_tracked_paths (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    category_id INTEGER NOT NULL,
    library TEXT NOT NULL DEFAULT 'material',
    last_imported_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (category_id) REFERENCES kb_categories(id) ON DELETE CASCADE
);
