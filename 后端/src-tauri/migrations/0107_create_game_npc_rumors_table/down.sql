-- D4.7 跨 NPC 关系联动：传闻表回滚
-- 回滚 0107_create_game_npc_rumors_table/up.sql

DROP INDEX IF EXISTS idx_game_npc_rumors_target;
DROP TABLE IF EXISTS game_npc_rumors;
