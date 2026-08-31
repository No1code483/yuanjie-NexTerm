-- Migration v53: create_xin_summaries_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_xin_summaries_table"（T2.6.1 兼容性设计）
-- 说明: 小欣对话摘要表，存储每段对话的 summary / 关键要点 / 主题 / 情感

CREATE TABLE IF NOT EXISTS xin_summaries (
    id TEXT PRIMARY KEY,
    conversation_id INTEGER,
    summary TEXT NOT NULL,
    key_takeaways TEXT NOT NULL DEFAULT '[]',
    topics TEXT NOT NULL DEFAULT '[]',
    sentiment TEXT,
    created_at TEXT NOT NULL
);
