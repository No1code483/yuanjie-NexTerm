-- Migration v103: create_game_story_nodes_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/模块深化/04_游戏_真实AI接入_深度.md §D4.4（动态剧情 DB 持久化）
-- hash 源: 函数名 "create_game_story_nodes_table"
-- 说明: D4.4b 剧情节点表 — 存储每次剧情会话的所有节点
--       每个节点对应剧情推进的一个分支点，包含：
--       - 场景描述 + 旁白叙述
--       - 选择项列表（JSON 数组，含 id/text/hint）
--       - 是否结局节点（is_ending=true 时 choices 为空）
--       sequence 字段记录节点在剧情中的顺序（0=start, 1,2,3...）

CREATE TABLE IF NOT EXISTS game_story_nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    story_id TEXT NOT NULL,                    -- 所属剧情会话 ID（关联 game_stories.id）
    node_id TEXT NOT NULL,                    -- 节点逻辑 ID（start / node_1 / node_2 ...）
    title TEXT NOT NULL DEFAULT '未知剧情',    -- 节点标题
    description TEXT NOT NULL DEFAULT '',      -- 场景描述
    narration TEXT NOT NULL DEFAULT '',        -- 旁白叙述
    choices TEXT NOT NULL DEFAULT '[]',        -- 选择项 JSON 数组 [{id,text,hint}]
    is_ending INTEGER NOT NULL DEFAULT 0,      -- 是否结局节点
    sequence INTEGER NOT NULL DEFAULT 0,       -- 节点顺序（0=start）
    created_at TEXT NOT NULL,                  -- 创建时间
    FOREIGN KEY (story_id) REFERENCES game_stories(id) ON DELETE CASCADE
);

-- 索引 1：按剧情会话查询所有节点（按 sequence 排序还原剧情线）
CREATE INDEX IF NOT EXISTS idx_game_story_nodes_story ON game_story_nodes(story_id, sequence);
-- 索引 2：按节点 ID 查询（推进剧情时定位当前节点）
CREATE INDEX IF NOT EXISTS idx_game_story_nodes_node ON game_story_nodes(story_id, node_id);
