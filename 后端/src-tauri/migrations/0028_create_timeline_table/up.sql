-- Migration v28: create_timeline_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_timeline_table"（T2.6.1 兼容性设计）
-- 说明: 时间线事件表，按日期组织个人里程碑

CREATE TABLE IF NOT EXISTS timeline (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
