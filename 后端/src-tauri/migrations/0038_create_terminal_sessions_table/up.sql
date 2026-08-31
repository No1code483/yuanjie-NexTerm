-- Migration v38: create_terminal_sessions_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_terminal_sessions_table"（T2.6.1 兼容性设计）
-- 说明: 终端会话表，存储每个 PTY 会话的状态、尺寸、生命周期
--       tab_id / pane_id 字段由 v39 动态迁移补丁为旧库补加（v38 已包含）

CREATE TABLE IF NOT EXISTS terminal_sessions (
    id TEXT PRIMARY KEY,
    session_type TEXT NOT NULL DEFAULT 'cmd',
    tab_id TEXT,
    pane_id TEXT,
    cols INTEGER NOT NULL DEFAULT 80,
    rows INTEGER NOT NULL DEFAULT 24,
    status TEXT NOT NULL DEFAULT 'active',
    created_at INTEGER NOT NULL,
    killed_at INTEGER
);
