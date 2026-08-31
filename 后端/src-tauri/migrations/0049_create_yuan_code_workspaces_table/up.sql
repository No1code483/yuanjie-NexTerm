-- Migration v49: create_yuan_code_workspaces_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_yuan_code_workspaces_table"（T2.6.1 兼容性设计）
-- 说明: 元·代码工作区表，存储工作区路径、Tabs JSON、活动 Tab 索引

CREATE TABLE IF NOT EXISTS yuan_code_workspaces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    workspace_path TEXT NOT NULL,
    tabs_json TEXT NOT NULL DEFAULT '[]',
    active_tab_index INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
