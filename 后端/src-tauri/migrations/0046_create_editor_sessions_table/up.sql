-- Migration v46: create_editor_sessions_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_editor_sessions_table"（T2.6.1 兼容性设计）
-- 说明: 编辑器会话表，每个 doc_uuid 一行，跟踪脏标记、光标位置、最后活动时间

CREATE TABLE IF NOT EXISTS editor_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_uuid TEXT NOT NULL UNIQUE,
    is_dirty INTEGER NOT NULL DEFAULT 0,
    cursor_line INTEGER,
    cursor_column INTEGER,
    last_activity INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
