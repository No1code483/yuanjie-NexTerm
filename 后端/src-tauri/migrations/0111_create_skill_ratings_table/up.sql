-- Migration v111: create_skill_ratings_table
-- 类型: Static CREATE TABLE
-- 规范: 功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §Phase 5（v1.54 Skill 评分/评论）
--       + 项目核心设计意图 §三（Yuan Code Skill 系统）
-- hash 源: 函数名 "create_skill_ratings_table"（T2.6.1 兼容性设计）
-- 说明: Yuan Code v3.2 Skill 市场评分/评论系统专用表。
--       每个 user 对每个 skill_name 只能有一条评分记录（UNIQUE 约束），
--       评分更新时走 UPSERT（INSERT ON CONFLICT UPDATE）。
--       rating 范围 1-5（前端星级），review 为可选评论文本。

CREATE TABLE IF NOT EXISTS skill_ratings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    skill_name TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    rating INTEGER NOT NULL CHECK(rating BETWEEN 1 AND 5),
    review TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(skill_name, user_id)
);

CREATE INDEX IF NOT EXISTS idx_skill_ratings_skill ON skill_ratings(skill_name);
CREATE INDEX IF NOT EXISTS idx_skill_ratings_user ON skill_ratings(user_id);
