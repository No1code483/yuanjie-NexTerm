-- Migration v25: create_recycle_bin_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + 内部调用 migrate_recycle_bin_table 动态补字段）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_recycle_bin_table"（T2.6.1 兼容性设计）
-- 说明: 回收站表。本 .sql 仅包含 CREATE TABLE 静态部分；
--       动态字段补丁（title/deleted_by）
--       由 inline 函数 migrate_recycle_bin_table 通过 PRAGMA + ALTER TABLE 完成。

CREATE TABLE IF NOT EXISTS recycle_bin (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    original_path TEXT NOT NULL,
    item_type TEXT NOT NULL,
    item_id INTEGER,
    title TEXT,
    metadata_json TEXT,
    file_size INTEGER,
    deleted_by INTEGER,
    deleted_at INTEGER NOT NULL,
    auto_delete_at INTEGER NOT NULL
);
