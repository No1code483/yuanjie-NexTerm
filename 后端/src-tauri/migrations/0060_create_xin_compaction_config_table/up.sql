-- Migration v60: create_xin_compaction_config_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_xin_compaction_config_table"（T2.6.1 兼容性设计）
-- 说明: 小欣上下文压缩配置单行表（id=1），存储触发阈值、保留 token 数、压缩模式

CREATE TABLE IF NOT EXISTS xin_compaction_config (
    id INTEGER PRIMARY KEY DEFAULT 1,
    enabled INTEGER NOT NULL DEFAULT 1,
    trigger_threshold_ratio REAL NOT NULL DEFAULT 0.8,
    keep_recent_tokens INTEGER NOT NULL DEFAULT 4000,
    model_override TEXT,
    memory_flush_enabled INTEGER NOT NULL DEFAULT 1,
    notify_user INTEGER NOT NULL DEFAULT 0,
    compaction_mode TEXT NOT NULL DEFAULT 'sliding_window'
);
