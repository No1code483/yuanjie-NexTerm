-- Migration v58: create_xin_checkpoints_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_xin_checkpoints_table"（T2.6.1 兼容性设计）
-- 说明: 小欣对话检查点表，支持 auto/manual 类型，存储 context 快照与部分响应

CREATE TABLE IF NOT EXISTS xin_checkpoints (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    checkpoint_type TEXT NOT NULL DEFAULT 'auto',
    context_json TEXT NOT NULL,
    partial_response TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    title TEXT,
    created_at TEXT NOT NULL
);
