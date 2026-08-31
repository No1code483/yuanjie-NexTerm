-- Migration v36: create_terminal_tab_layout_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_terminal_tab_layout_table"（T2.6.1 兼容性设计）
-- 说明: 终端 Tab 布局持久化表，存储每个 Tab 的位置、激活状态等
--       pane_data 字段由 v37 动态迁移补丁添加（旧库无此字段）

CREATE TABLE IF NOT EXISTS terminal_tab_layout (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tab_id TEXT NOT NULL UNIQUE,
    tab_type TEXT NOT NULL DEFAULT 'builtin',
    title TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
