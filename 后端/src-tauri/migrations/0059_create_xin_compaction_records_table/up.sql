-- Migration v59: create_xin_compaction_records_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_xin_compaction_records_table"（T2.6.1 兼容性设计）
-- 说明: 小欣上下文压缩记录表，记录压缩前后的消息/token 计数、摘要、记忆刷写次数

CREATE TABLE IF NOT EXISTS xin_compaction_records (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    trigger_type TEXT NOT NULL DEFAULT 'auto',
    pre_message_count INTEGER NOT NULL DEFAULT 0,
    pre_token_count INTEGER NOT NULL DEFAULT 0,
    post_message_count INTEGER NOT NULL DEFAULT 0,
    post_token_count INTEGER NOT NULL DEFAULT 0,
    summary_text TEXT NOT NULL DEFAULT '',
    memory_flush_count INTEGER NOT NULL DEFAULT 0,
    guidance_text TEXT,
    created_at TEXT NOT NULL
);
