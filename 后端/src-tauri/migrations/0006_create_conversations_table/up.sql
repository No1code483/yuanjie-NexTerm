-- Migration v6: create_conversations_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_conversations_table"（T2.6.1 兼容性设计）
-- 说明: 多轮对话主表，支持 single/group 类型与 token 预算

CREATE TABLE IF NOT EXISTS conversations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT,
    type TEXT NOT NULL DEFAULT 'single',
    is_temp INTEGER NOT NULL DEFAULT 0,
    dissolve_at INTEGER,
    token_budget INTEGER NOT NULL DEFAULT 50000,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
