-- D4.6 智能 NPC 深化：NPC↔玩家关系表（game_npc_relationships）回滚
DROP INDEX IF EXISTS idx_game_npc_relationships_world;
DROP TABLE IF EXISTS game_npc_relationships;
