-- Migration v78: create_game_points_log_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + inline 2 个 CREATE INDEX）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_game_points_log_table"（T2.6.1 兼容性设计）
-- 说明: 游戏 3D 重构 - 积分流水日志表（event_id 唯一约束防重）。本 .sql 仅包含 CREATE TABLE；
--       idx_game_points_log_world + idx_game_points_log_domain 索引由 inline 语句创建。

CREATE TABLE IF NOT EXISTS game_points_log (
    id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,
    source_type TEXT NOT NULL,
    domain_id TEXT NOT NULL,
    points_delta INTEGER NOT NULL,
    points_after INTEGER NOT NULL,
    result TEXT NOT NULL,
    event_id TEXT NOT NULL UNIQUE,
    metadata_json TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (world_id) REFERENCES game_worlds(id) ON DELETE CASCADE,
    FOREIGN KEY (domain_id) REFERENCES game_knowledge_domains(id)
);
