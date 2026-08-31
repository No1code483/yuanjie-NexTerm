-- D4.3 自适应难度：玩家能力评估表
--
-- 记录玩家在突破考验中的历史表现，用于动态调整难度系数。
-- 设计依据：04_游戏_真实AI接入_深度.md §2.3 自适应难度（心流理论：挑战略高于能力）
--
-- 单玩家世界假设：world_id 为主键（一个世界一行技能记录）

CREATE TABLE IF NOT EXISTS game_player_skill (
    world_id          TEXT    PRIMARY KEY,

    -- 综合能力评分 0.0-100.0（基于成功率 + 平均分 + 连胜综合计算）
    skill_score       REAL    NOT NULL DEFAULT 50.0,

    -- 突破历史统计
    attempt_count     INTEGER NOT NULL DEFAULT 0,  -- 总尝试次数
    success_count     INTEGER NOT NULL DEFAULT 0,  -- 成功突破次数
    fail_count        INTEGER NOT NULL DEFAULT 0,  -- 失败次数（failed + dropped）
    total_score       INTEGER NOT NULL DEFAULT 0,  -- 累计得分（满分 100/次）
    avg_score         REAL    NOT NULL DEFAULT 0.0, -- 平均得分
    last_score        INTEGER NOT NULL DEFAULT 0,  -- 最近一次得分
    last_result       TEXT    NOT NULL DEFAULT '', -- 最近一次结果：success/failed/dropped

    -- 连胜/连败（正数=连胜，负数=连败，0=无）
    streak            INTEGER NOT NULL DEFAULT 0,

    -- 难度乘数（应用到 difficulty_coefficient 上，默认 1.0）
    -- 范围 0.6-1.5：连败降难度，连胜升难度
    difficulty_multiplier REAL NOT NULL DEFAULT 1.0,

    created_at        INTEGER NOT NULL,
    updated_at        INTEGER NOT NULL
);

-- 索引：按 skill_score 倒序（用于全局排行榜，未来扩展）
CREATE INDEX IF NOT EXISTS idx_game_player_skill_score ON game_player_skill(skill_score DESC);
