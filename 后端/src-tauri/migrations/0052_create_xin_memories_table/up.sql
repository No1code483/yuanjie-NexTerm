-- Migration v52: create_xin_memories_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_xin_memories_table"（T2.6.1 兼容性设计）
-- 说明: 小欣长期记忆表，存储分类记忆（category + key + value + 重要性 + 来源 + 信心度）

CREATE TABLE IF NOT EXISTS xin_memories (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    importance REAL NOT NULL DEFAULT 0.5,
    source TEXT NOT NULL DEFAULT 'user',
    confidence REAL NOT NULL DEFAULT 0.5,
    created_at TEXT NOT NULL,
    last_recalled_at TEXT
);
