-- D4.6 智能 NPC 深化：NPC↔玩家关系表（game_npc_relationships）
-- 创建日期：2026-07-23
-- 用途：存储 NPC 对玩家的关系值（-100~100，AI 每轮评估增量）
-- 表结构选择：world+npc 单行（符合单玩家世界假设，每对 world_id+npc_id UNIQUE）

CREATE TABLE IF NOT EXISTS game_npc_relationships (
    id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,
    npc_id TEXT NOT NULL,
    -- 关系值 -100（仇恨）~ +100（挚友），0 为中立，初次见面默认 0
    relationship_value INTEGER NOT NULL DEFAULT 0 CHECK (relationship_value BETWEEN -100 AND 100),
    -- 互动统计
    interaction_count INTEGER NOT NULL DEFAULT 0,
    first_interaction_at INTEGER NOT NULL,
    last_interaction_at INTEGER NOT NULL,
    -- 关系变化历史（JSON 数组，每项 {turn, delta, reason, ts}，最多保留 50 条）
    relationship_history TEXT NOT NULL DEFAULT '[]',
    -- 当前关系等级标签（基于 relationship_value 派生，定期同步：hostile/cold/neutral/warm/close/sworn）
    relationship_label TEXT NOT NULL DEFAULT 'neutral',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    -- 单玩家世界约束：每对 world_id+npc_id 只允许一行
    UNIQUE (world_id, npc_id),
    FOREIGN KEY (npc_id) REFERENCES game_npcs(id)
);

-- 按 world_id 查询所有 NPC 关系（NPC 列表场景）
CREATE INDEX IF NOT EXISTS idx_game_npc_relationships_world ON game_npc_relationships(world_id);
