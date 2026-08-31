-- Migration v71: create_game_worlds_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + inline CREATE INDEX）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_game_worlds_table"（T2.6.1 兼容性设计）
-- 说明: 游戏 3D 重构 - 世界主表。本 .sql 仅包含 CREATE TABLE 静态部分；
--       idx_game_worlds_updated 索引由 inline 语句创建（sqlx::query 单语句限制）。

CREATE TABLE IF NOT EXISTS game_worlds (
    id TEXT PRIMARY KEY,
    player_name TEXT NOT NULL DEFAULT '无名修士',
    civilization_level INTEGER NOT NULL DEFAULT 1,
    realm_major TEXT NOT NULL DEFAULT 'mortal',
    realm_minor TEXT NOT NULL DEFAULT 'early',
    dao_foundation TEXT NOT NULL DEFAULT 'white',
    total_xp INTEGER NOT NULL DEFAULT 0,
    realm_xp INTEGER NOT NULL DEFAULT 0,
    map_width INTEGER NOT NULL DEFAULT 32,
    map_height INTEGER NOT NULL DEFAULT 32,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
