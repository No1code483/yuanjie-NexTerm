-- Migration v79: create_game_daily_limit_counter_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + inline CREATE INDEX）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_game_daily_limit_counter_table"（T2.6.1 兼容性设计）
-- 说明: 游戏 3D 重构 - 每日积分上限计数器表（world + date + domain + source 唯一）。本 .sql 仅包含 CREATE TABLE；
--       idx_game_daily_limit_world 索引由 inline 语句创建。

CREATE TABLE IF NOT EXISTS game_daily_limit_counter (
    id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,
    counter_date TEXT NOT NULL,
    domain_id TEXT NOT NULL,
    source_type TEXT NOT NULL,
    current_count INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL,
    UNIQUE(world_id, counter_date, domain_id, source_type),
    FOREIGN KEY (world_id) REFERENCES game_worlds(id) ON DELETE CASCADE,
    FOREIGN KEY (domain_id) REFERENCES game_knowledge_domains(id)
);
