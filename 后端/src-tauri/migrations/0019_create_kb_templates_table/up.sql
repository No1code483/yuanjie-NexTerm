-- Migration v19: create_kb_templates_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + 内部调用 seed_default_templates 种子数据）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_kb_templates_table"（T2.6.1 兼容性设计）
-- 说明: 知识库模板表。本 .sql 仅包含 CREATE TABLE 静态部分；
--       种子数据（6 个默认模板：会议纪要/研究笔记/周报总结/读书笔记/项目计划/链接收藏）
--       由 inline 函数 seed_default_templates 在首次创建时插入。

CREATE TABLE IF NOT EXISTS kb_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    icon TEXT NOT NULL DEFAULT '📄',
    description TEXT NOT NULL DEFAULT '',
    entry_type TEXT NOT NULL DEFAULT 'text',
    content TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
