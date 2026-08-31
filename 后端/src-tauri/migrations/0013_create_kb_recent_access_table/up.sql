-- Migration v13: create_kb_recent_access_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_kb_recent_access_table"（T2.6.1 兼容性设计）
-- 说明: 知识库最近访问记录表，FK 引用 kb_entries(id) ON DELETE CASCADE

CREATE TABLE IF NOT EXISTS kb_recent_access (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entry_id INTEGER NOT NULL,
    accessed_at INTEGER NOT NULL,
    FOREIGN KEY (entry_id) REFERENCES kb_entries(id) ON DELETE CASCADE
);
