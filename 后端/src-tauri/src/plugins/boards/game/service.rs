//! boards.game（L1）服务边界说明。
//!
//! 批次4b：L1 直接承载游戏板块全部 44 条命令 + 17 张 `game_*` 表归属登记。
//! 实现层（game_service / game_repo / game_story_service / game_opponent_service /
//! game_natural_language_service / game_behavior_analyzer_service / game_intelligence_hook /
//! timeline_service 等）仍留主应用，dispatcher 复用（同 3b-2 裁定 15 口径，
//! 实现层迁移登记阶段4）。

pub const OWNED_TABLES: &[&str] = &[
    "game_worlds",
    "game_knowledge_domains",
    "game_buildings",
    "game_knowledge_progress",
    "game_breakthrough_records",
    "game_build_history",
    "game_kb_category_mapping",
    "game_points_log",
    "game_daily_limit_counter",
    "game_points_source_config",
    "game_npcs",
    "game_npc_conversations",
    "game_stories",
    "game_story_nodes",
    "game_npc_memories",
    "game_npc_relationships",
    "game_npc_rumors",
    "game_player_skill",
];
