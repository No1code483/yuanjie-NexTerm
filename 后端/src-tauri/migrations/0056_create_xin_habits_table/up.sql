-- Migration v56: create_xin_habits_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_xin_habits_table"（T2.6.1 兼容性设计）
-- 说明: 小欣习惯跟踪表，记录连续天数、总打卡次数、今日是否完成

CREATE TABLE IF NOT EXISTS xin_habits (
    id TEXT PRIMARY KEY,
    habit_name TEXT NOT NULL,
    streak_days INTEGER NOT NULL DEFAULT 0,
    total_checkins INTEGER NOT NULL DEFAULT 0,
    last_checkin TEXT,
    completed_today INTEGER NOT NULL DEFAULT 0,
    category TEXT NOT NULL DEFAULT 'general'
);
