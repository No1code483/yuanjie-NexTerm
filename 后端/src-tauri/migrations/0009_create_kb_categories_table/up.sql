-- Migration v9: create_kb_categories_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + 内部调用 migrate_kb_categories_library 动态补字段）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_kb_categories_table"（T2.6.1 兼容性设计）
-- 说明: 知识库分类表（自引用 FK）。本 .sql 仅包含 CREATE TABLE 静态部分；
--       动态字段补丁（library）由 inline 函数 migrate_kb_categories_library
--       通过 PRAGMA + ALTER TABLE 完成。

CREATE TABLE IF NOT EXISTS kb_categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    parent_id INTEGER,
    library TEXT NOT NULL DEFAULT 'material',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (parent_id) REFERENCES kb_categories(id) ON DELETE SET NULL
);
