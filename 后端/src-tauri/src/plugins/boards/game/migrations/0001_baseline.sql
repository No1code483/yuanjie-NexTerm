-- boards.game 基线迁移（批次4b）：只登记既有表归属，不重放历史表 DDL。
-- 17 张 game_* 表由主应用迁移创建（0071-0107）；此处仅补登记 kernel_table_ownership，
-- 供 registry.rs::register_manifest_tables 校验（旧表须先登记后登记 manifest）。
-- 前缀口径：短码 `gm` + `_` = `gm_`，17 张表均以 `game_` 前缀 → name_prefixed=0。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('game_worlds',                'boards.game', 0, datetime('now')),
    ('game_knowledge_domains',     'boards.game', 0, datetime('now')),
    ('game_buildings',             'boards.game', 0, datetime('now')),
    ('game_knowledge_progress',    'boards.game', 0, datetime('now')),
    ('game_breakthrough_records',  'boards.game', 0, datetime('now')),
    ('game_build_history',         'boards.game', 0, datetime('now')),
    ('game_kb_category_mapping',   'boards.game', 0, datetime('now')),
    ('game_points_log',            'boards.game', 0, datetime('now')),
    ('game_daily_limit_counter',   'boards.game', 0, datetime('now')),
    ('game_points_source_config',  'boards.game', 0, datetime('now')),
    ('game_npcs',                  'boards.game', 0, datetime('now')),
    ('game_npc_conversations',     'boards.game', 0, datetime('now')),
    ('game_stories',               'boards.game', 0, datetime('now')),
    ('game_story_nodes',           'boards.game', 0, datetime('now')),
    ('game_npc_memories',          'boards.game', 0, datetime('now')),
    ('game_npc_relationships',     'boards.game', 0, datetime('now')),
    ('game_npc_rumors',            'boards.game', 0, datetime('now')),
    ('game_player_skill',          'boards.game', 0, datetime('now'));
