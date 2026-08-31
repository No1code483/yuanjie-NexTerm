-- Migration v43: create_editor_documents_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_editor_documents_table"（T2.6.1 兼容性设计）
-- 说明: 编辑器文档主表，存储文档元信息（UUID、源、类型、语言、版本计数等）
--       version 字段由 v44 动态迁移补丁为旧库补加（v43 不含 version）

CREATE TABLE IF NOT EXISTS editor_documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_uuid TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    source_type TEXT NOT NULL,
    source_id INTEGER,
    content_type TEXT NOT NULL,
    language TEXT,
    file_size INTEGER NOT NULL DEFAULT 0,
    version_count INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
