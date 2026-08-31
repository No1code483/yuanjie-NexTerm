-- Migration v75: create_game_breakthrough_records_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + inline CREATE INDEX）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_game_breakthrough_records_table"（T2.6.1 兼容性设计）
-- 说明: 游戏 3D 重构 - 突破记录表（含问答 JSON、AI 评审、弱点分析）。本 .sql 仅包含 CREATE TABLE；
--       idx_game_breakthrough_records_world 索引由 inline 语句创建。

CREATE TABLE IF NOT EXISTS game_breakthrough_records (
    id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,
    from_realm TEXT NOT NULL,
    to_realm TEXT,
    score INTEGER,
    dao_foundation_awarded TEXT,
    result TEXT NOT NULL,
    questions_json TEXT,
    answers_json TEXT,
    ai_review TEXT,
    weakness_json TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (world_id) REFERENCES game_worlds(id) ON DELETE CASCADE
);
