-- Migration v63: create_behavior_patterns_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_behavior_patterns_table"（T2.6.1 兼容性设计）
-- 说明: 用户行为模式表，按 user_id + date 存储专注度、分心次数、模块多样性等指标

CREATE TABLE IF NOT EXISTS behavior_patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    date TEXT NOT NULL,
    focus_score REAL NOT NULL DEFAULT 0,
    distraction_count INTEGER NOT NULL DEFAULT 0,
    kb_avg_duration_secs INTEGER NOT NULL DEFAULT 0,
    kb_entry_count INTEGER NOT NULL DEFAULT 0,
    error_operation_count INTEGER NOT NULL DEFAULT 0,
    total_operation_count INTEGER NOT NULL DEFAULT 0,
    active_start_hour INTEGER,
    active_end_hour INTEGER,
    peak_hour INTEGER,
    module_diversity INTEGER NOT NULL DEFAULT 0,
    consistency_score REAL NOT NULL DEFAULT 0,
    summary TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
