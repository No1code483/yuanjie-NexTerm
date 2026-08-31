-- Migration v102: create_game_stories_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/模块深化/04_游戏_真实AI接入_深度.md §D4.4（动态剧情 DB 持久化）
-- hash 源: 函数名 "create_game_stories_table"
-- 说明: D4.4b 动态剧情主表 — 持久化每次生成的剧情会话
--       替代 MVP 阶段的 OnceCell 内存存储，支持：
--       1) 跨会话恢复剧情（玩家行为影响剧情走向的持久性验收）
--       2) 历史剧情列表查询（前端时间线展示）
--       3) 节点表 game_story_nodes 通过 story_id 关联（0103）

CREATE TABLE IF NOT EXISTS game_stories (
    id TEXT PRIMARY KEY,                       -- 剧情会话 ID（story_xxxx）
    world_id TEXT NOT NULL,                    -- 所属世界 ID
    user_id INTEGER NOT NULL,                 -- 玩家用户 ID（鉴权隔离）
    theme TEXT NOT NULL DEFAULT '修仙历练',     -- 剧情主题
    current_node_id TEXT NOT NULL DEFAULT 'start',  -- 当前所处节点 ID
    used_ai INTEGER NOT NULL DEFAULT 0,        -- 是否使用 AI 生成（0=降级模板，1=AI）
    is_finished INTEGER NOT NULL DEFAULT 0,    -- 剧情是否已达成结局
    node_count INTEGER NOT NULL DEFAULT 0,     -- 节点总数（冗余字段，便于列表展示）
    created_at TEXT NOT NULL,                 -- 创建时间（ISO8601）
    updated_at TEXT NOT NULL                   -- 最后推进时间（ISO8601）
);

-- 索引 1：按玩家查询其所有剧情（历史列表，按时间倒序）
CREATE INDEX IF NOT EXISTS idx_game_stories_user ON game_stories(user_id, updated_at DESC);
-- 索引 2：按世界查询剧情（世界维度统计）
CREATE INDEX IF NOT EXISTS idx_game_stories_world ON game_stories(world_id, updated_at DESC);
