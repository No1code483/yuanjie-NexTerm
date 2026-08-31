-- D4.7 跨 NPC 关系联动：传闻表（game_npc_rumors）
-- 创建日期：2026-07-23
-- 用途：当一个 NPC 与玩家互动产生重要记忆时，该记忆会"传播"为传闻到同世界其他 NPC，
--       让其他 NPC 在对话中能"听说"关于玩家的事，形成跨 NPC 关系联动（传闻机制）。
-- 设计参考：04_游戏_真实AI接入_深度.md §2.1 + 09_游戏/游戏.md §10.3 D4.6 剩余 5%
--
-- 与 game_npc_memories 的关系：
--   - game_npc_memories 存"亲历记忆"（NPC 自己与玩家互动产生的，source='rule'/'ai'）
--   - game_npc_rumors 存"传闻记忆"（从其他 NPC 传播来的，非亲历）
--   - 独立表避免修改 D4.6 表结构（非侵入式扩展）

CREATE TABLE IF NOT EXISTS game_npc_rumors (
    id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,

    -- 传闻来源：产生该记忆的 NPC（亲历者）
    source_npc_id TEXT NOT NULL,
    -- 传闻目标：听到该传闻的 NPC
    target_npc_id TEXT NOT NULL,

    -- 继承自源记忆的字段
    memory_type TEXT NOT NULL DEFAULT 'fact',    -- fact/preference/commitment/event
    content TEXT NOT NULL,                        -- 传闻内容（自然语言）
    importance REAL NOT NULL DEFAULT 0.5,         -- 继承源记忆重要度（影响召回优先级）

    -- 源记忆 ID（用于去重：同一记忆不重复传播给同一 NPC）
    origin_memory_id TEXT NOT NULL,

    created_at INTEGER NOT NULL,

    -- 同一源记忆不重复传播给同一目标 NPC
    UNIQUE(world_id, target_npc_id, origin_memory_id),
    FOREIGN KEY (source_npc_id) REFERENCES game_npcs(id),
    FOREIGN KEY (target_npc_id) REFERENCES game_npcs(id)
);

-- 按世界+目标 NPC 查询传闻，按重要度倒序（高重要度优先召回）
CREATE INDEX IF NOT EXISTS idx_game_npc_rumors_target
    ON game_npc_rumors(world_id, target_npc_id, importance DESC);
