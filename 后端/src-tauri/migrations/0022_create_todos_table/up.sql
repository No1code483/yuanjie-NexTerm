-- Migration v22: create_todos_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + 内部调用 migrate_todos_table 动态补字段）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_todos_table"（T2.6.1 兼容性设计）
-- 说明: 待办事项表。本 .sql 仅包含 CREATE TABLE 静态部分；
--       动态字段补丁（description/priority/due_date/user_id/version）
--       由 inline 函数 migrate_todos_table 通过 PRAGMA + ALTER TABLE 完成。

CREATE TABLE IF NOT EXISTS todos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    description TEXT,
    priority TEXT NOT NULL DEFAULT 'medium',
    due_date TEXT,
    date TEXT NOT NULL,
    completed INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
