-- Migration v100: create_xin_persona_memories_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/模块深化/03_小欣_多模态融合_深度.md §2.7（D3.8 人格系统补全）
-- hash 源: 函数名 "create_xin_persona_memories_table"
-- 说明: D3.8 人格记忆表 — 每个 persona 积累独立的交互摘要 + 用户偏好
--       切换回某人格时，可从该表恢复该人格的上下文记忆
--       自生长人格（self_growing）也用此表积累"用户画像"原料

CREATE TABLE IF NOT EXISTS xin_persona_memories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    persona_id TEXT NOT NULL UNIQUE,
    interaction_summary TEXT NOT NULL DEFAULT '',
    user_preferences TEXT NOT NULL DEFAULT '{}',
    topic_tags TEXT NOT NULL DEFAULT '[]',
    interaction_count INTEGER NOT NULL DEFAULT 0,
    last_interaction_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_xin_persona_memories_persona ON xin_persona_memories(persona_id);
CREATE INDEX IF NOT EXISTS idx_xin_persona_memories_last_interaction ON xin_persona_memories(last_interaction_at DESC);
