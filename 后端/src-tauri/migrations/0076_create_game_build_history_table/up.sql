-- Migration v76: create_game_build_history_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + inline 2 个 CREATE INDEX）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_game_build_history_table"（T2.6.1 兼容性设计）
-- 说明: 游戏 3D 重构 - 建造历史事件流表。本 .sql 仅包含 CREATE TABLE；
--       idx_game_build_history_world + idx_game_build_history_building 索引由 inline 语句创建。

CREATE TABLE IF NOT EXISTS game_build_history (
    id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    building_id TEXT NOT NULL,
    building_name TEXT NOT NULL,
    pos_x REAL,
    pos_y REAL,
    pos_z REAL,
    level INTEGER,
    progress REAL,
    snapshot_json TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (world_id) REFERENCES game_worlds(id) ON DELETE CASCADE
);
