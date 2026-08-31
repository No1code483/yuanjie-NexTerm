-- Migration v50: create_yuan_goals_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_yuan_goals_table"（T2.6.1 兼容性设计）
-- 说明: 元·目标表，存储 Agent 目标（标题、状态、优先级、进度、父目标、token 预算等）

CREATE TABLE IF NOT EXISTS yuan_goals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    goal_uuid TEXT NOT NULL UNIQUE,
    session_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 0,
    progress_pct INTEGER NOT NULL DEFAULT 0,
    parent_goal_id INTEGER,
    agent_id TEXT,
    token_budget INTEGER,
    tokens_used INTEGER NOT NULL DEFAULT 0,
    thread_json TEXT NOT NULL DEFAULT '{}',
    result_json TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
