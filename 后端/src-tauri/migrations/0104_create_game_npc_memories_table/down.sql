-- D4.6 智能 NPC 深化：长期记忆表（game_npc_memories）回滚
DROP INDEX IF EXISTS idx_game_npc_memories_type;
DROP INDEX IF EXISTS idx_game_npc_memories_world_npc;
DROP TABLE IF EXISTS game_npc_memories;
