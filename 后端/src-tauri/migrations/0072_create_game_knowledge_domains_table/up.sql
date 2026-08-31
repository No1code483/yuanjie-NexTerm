-- Migration v72: create_game_knowledge_domains_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + inline CREATE INDEX + inline seed 函数）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_game_knowledge_domains_table"（T2.6.1 兼容性设计）
-- 说明: 游戏 3D 重构 - 12 个学术知识领域定义表。本 .sql 仅包含 CREATE TABLE 静态部分；
--       idx_game_knowledge_domains_sort 索引 + 12 条种子数据（INSERT OR IGNORE）
--       由 inline 语句 / seed_game_knowledge_domains 函数完成。

CREATE TABLE IF NOT EXISTS game_knowledge_domains (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    name_en TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    building_category TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
);
