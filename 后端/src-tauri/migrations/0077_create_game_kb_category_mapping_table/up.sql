-- Migration v77: create_game_kb_category_mapping_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + inline CREATE INDEX）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_game_kb_category_mapping_table"（T2.6.1 兼容性设计）
-- 说明: 游戏 3D 重构 - 知识库分类到知识领域的映射表。本 .sql 仅包含 CREATE TABLE；
--       idx_game_kb_category_mapping_domain 索引由 inline 语句创建。

CREATE TABLE IF NOT EXISTS game_kb_category_mapping (
    category_id INTEGER PRIMARY KEY,
    domain_id TEXT NOT NULL,
    mapped_at INTEGER NOT NULL,
    mapped_by TEXT NOT NULL DEFAULT 'manual',
    confidence REAL,
    FOREIGN KEY (domain_id) REFERENCES game_knowledge_domains(id)
);
