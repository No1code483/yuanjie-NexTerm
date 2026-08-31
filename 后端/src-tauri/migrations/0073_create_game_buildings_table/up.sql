-- Migration v73: create_game_buildings_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + inline 2 个 CREATE INDEX）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_game_buildings_table"（T2.6.1 兼容性设计）
-- 说明: 游戏 3D 重构 - 建筑实例表。本 .sql 仅包含 CREATE TABLE 静态部分；
--       idx_game_buildings_world + idx_game_buildings_status 索引由 inline 语句创建。

CREATE TABLE IF NOT EXISTS game_buildings (
    id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,
    building_category TEXT NOT NULL,
    building_subtype TEXT NOT NULL,
    name TEXT NOT NULL,
    level INTEGER NOT NULL DEFAULT 1,
    pos_x REAL NOT NULL,
    pos_y REAL NOT NULL,
    pos_z REAL NOT NULL,
    rotation_y REAL NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'completed',
    build_progress REAL NOT NULL DEFAULT 0,
    knowledge_domain TEXT NOT NULL,
    built_at INTEGER,
    completed_at INTEGER,
    FOREIGN KEY (world_id) REFERENCES game_worlds(id) ON DELETE CASCADE
);
