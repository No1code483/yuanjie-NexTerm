-- D4.3 自适应难度：玩家技能表回滚
-- 回滚 0106_create_game_player_skill_table/up.sql

DROP INDEX IF EXISTS idx_game_player_skill_score;
DROP TABLE IF EXISTS game_player_skill;
