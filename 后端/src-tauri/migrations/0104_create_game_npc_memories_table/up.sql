-- D4.6 智能 NPC 深化：长期记忆表（game_npc_memories）
-- 创建日期：2026-07-23
-- 用途：存储 NPC 对玩家的长期记忆（混合方案：规则匹配即时抽取 + AI 每 5 轮批量提取）
-- 设计参考：04_游戏_真实AI接入_深度.md §2.1.1 memory.longTerm

CREATE TABLE IF NOT EXISTS game_npc_memories (
    id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,
    npc_id TEXT NOT NULL,
    -- 记忆类型：fact（事实，如玩家姓名）/ preference（喜好）/ commitment（承诺）/ event（重要事件）
    memory_type TEXT NOT NULL DEFAULT 'fact',
    content TEXT NOT NULL,                    -- 记忆内容（自然语言，如"玩家叫张三"）
    importance REAL NOT NULL DEFAULT 0.5,     -- 重要度 0.0-1.0（影响召回优先级 + 衰减速率）
    source TEXT NOT NULL DEFAULT 'rule',     -- 来源：'rule'（规则匹配）/ 'ai'（AI 提取）
    -- 召回元数据（用于 LRU + 衰减策略）
    recall_count INTEGER NOT NULL DEFAULT 0,
    last_recalled_at INTEGER,                 -- 最近一次被注入 prompt 的时间戳（NULL 表示从未召回）
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (npc_id) REFERENCES game_npcs(id)
);

-- 按世界+NPC 查询记忆，按重要度倒序（高重要度优先召回）
CREATE INDEX IF NOT EXISTS idx_game_npc_memories_world_npc ON game_npc_memories(world_id, npc_id, importance DESC);
-- 按类型筛选（fact/preference/commitment/event）
CREATE INDEX IF NOT EXISTS idx_game_npc_memories_type ON game_npc_memories(world_id, npc_id, memory_type);
