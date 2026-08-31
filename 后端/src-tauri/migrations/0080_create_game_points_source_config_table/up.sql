-- Migration v80: create_game_points_source_config_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + inline seed 函数）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_game_points_source_config_table"（T2.6.1 兼容性设计）
-- 说明: 游戏 3D 重构 - 积分来源配置表。本 .sql 仅包含 CREATE TABLE 静态部分；
--       11 条种子数据（INSERT OR IGNORE）由 inline 函数 seed_game_points_source_config 完成。

CREATE TABLE IF NOT EXISTS game_points_source_config (
    source_type TEXT PRIMARY KEY,
    default_domain_id TEXT,
    points_per_event INTEGER NOT NULL,
    daily_limit INTEGER,
    description TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (default_domain_id) REFERENCES game_knowledge_domains(id)
);
