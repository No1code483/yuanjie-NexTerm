-- Migration v74: create_game_knowledge_progress_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + inline CREATE INDEX）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_game_knowledge_progress_table"（T2.6.1 兼容性设计）
-- 说明: 游戏 3D 重构 - 知识领域进度表（world_id + domain_id 唯一）。本 .sql 仅包含 CREATE TABLE；
--       idx_game_knowledge_progress_world 索引由 inline 语句创建。

CREATE TABLE IF NOT EXISTS game_knowledge_progress (
    id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,
    domain_id TEXT NOT NULL,
    points INTEGER NOT NULL DEFAULT 0,
    level INTEGER NOT NULL DEFAULT 1,
    total_earned INTEGER NOT NULL DEFAULT 0,
    total_consumed INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(world_id, domain_id),
    FOREIGN KEY (world_id) REFERENCES game_worlds(id) ON DELETE CASCADE,
    FOREIGN KEY (domain_id) REFERENCES game_knowledge_domains(id)
);
