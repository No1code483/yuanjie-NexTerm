-- D4.2 智能 NPC 系统（game_npcs / game_npc_conversations）
-- 创建日期：2026-07-21
-- 用途：存储 NPC 定义 + 对话历史

-- NPC 定义表
CREATE TABLE IF NOT EXISTS game_npcs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    role TEXT NOT NULL,                    -- 角色定位：guide/elder/merchant/scholar/artisan/rival
    realm_level TEXT NOT NULL DEFAULT 'mortal',  -- NPC 境界（决定对话风格）
    personality TEXT NOT NULL,             -- 性格描述（用于 AI prompt）
    knowledge_domains TEXT NOT NULL DEFAULT '[]',  -- 知识领域 JSON 数组
    greeting TEXT NOT NULL,                -- 初次见面的问候语
    system_prompt TEXT NOT NULL,           -- 完整 system prompt（与 AI 对话时使用）
    avatar_emoji TEXT NOT NULL DEFAULT '🧙',
    location TEXT,                         -- 所在地点（如 '青鸾峰' / '紫霄宫'）
    unlock_realm TEXT NOT NULL DEFAULT 'mortal',  -- 解锁境界（达到此境界才能与 NPC 对话）
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- NPC 对话历史表
CREATE TABLE IF NOT EXISTS game_npc_conversations (
    id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,
    npc_id TEXT NOT NULL,
    role TEXT NOT NULL,                    -- 'user' / 'assistant' / 'system'
    content TEXT NOT NULL,
    turn_index INTEGER NOT NULL,           -- 对话轮次（0 起步）
    created_at INTEGER NOT NULL,
    FOREIGN KEY (npc_id) REFERENCES game_npcs(id)
);

CREATE INDEX IF NOT EXISTS idx_game_npc_conversations_world_npc ON game_npc_conversations(world_id, npc_id, turn_index);
