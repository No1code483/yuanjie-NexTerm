-- Migration v47: create_editor_fts_table
-- 类型: 静态（单条 CREATE VIRTUAL TABLE，可提取）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_editor_fts_table"（T2.6.1 兼容性设计）
-- 说明: 编辑器文档全文搜索索引（FTS5），按 doc_uuid / title / content_type / content 索引
--       注意：与 v40 create_terminal_history_fts 不同，本表无触发器，可单语句提取

CREATE VIRTUAL TABLE IF NOT EXISTS editor_fts USING fts5(
    doc_uuid UNINDEXED,
    title,
    content_type UNINDEXED,
    content,
    tokenize='unicode61'
);
