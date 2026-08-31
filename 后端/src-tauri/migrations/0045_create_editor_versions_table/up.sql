-- Migration v45: create_editor_versions_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_editor_versions_table"（T2.6.1 兼容性设计）
-- 说明: 编辑器文档版本快照表，记录每个版本的文件路径、大小、变更摘要

CREATE TABLE IF NOT EXISTS editor_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_uuid TEXT NOT NULL,
    version_num INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL DEFAULT 0,
    change_summary TEXT,
    created_at INTEGER NOT NULL
);
