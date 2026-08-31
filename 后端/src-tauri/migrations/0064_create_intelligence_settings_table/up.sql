-- Migration v64: create_intelligence_settings_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_intelligence_settings_table"（T2.6.1 兼容性设计）
-- 说明: 智能系统配置表（每用户一行），存储日志保留、行为分析开关、LLM 增强、仪表盘默认周期等

CREATE TABLE IF NOT EXISTS intelligence_settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL UNIQUE,
    log_retention_days INTEGER NOT NULL DEFAULT 30,
    suggestion_retention_days INTEGER NOT NULL DEFAULT 30,
    behavior_analysis_enabled INTEGER NOT NULL DEFAULT 1,
    suggestion_enabled INTEGER NOT NULL DEFAULT 1,
    use_llm_enhancement INTEGER NOT NULL DEFAULT 0,
    llm_model TEXT DEFAULT '',
    behavior_analysis_period TEXT NOT NULL DEFAULT 'daily',
    dashboard_default_period TEXT NOT NULL DEFAULT 'weekly',
    activity_log_batch_size INTEGER NOT NULL DEFAULT 100,
    show_productivity_score INTEGER NOT NULL DEFAULT 1,
    show_behavior_analysis INTEGER NOT NULL DEFAULT 1,
    show_suggestions INTEGER NOT NULL DEFAULT 1,
    notification_frequency TEXT NOT NULL DEFAULT 'daily',
    extra_config TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
