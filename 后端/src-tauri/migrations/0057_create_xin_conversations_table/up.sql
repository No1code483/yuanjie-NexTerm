-- Migration v57: create_xin_conversations_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_xin_conversations_table"（T2.6.1 兼容性设计）
-- 说明: 小欣对话主表，存储 persona / model / context_json / token 计数 / 摘要

CREATE TABLE IF NOT EXISTS xin_conversations (
    id TEXT PRIMARY KEY,
    persona_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '新对话',
    context_json TEXT NOT NULL DEFAULT '',
    message_count INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    summary TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
