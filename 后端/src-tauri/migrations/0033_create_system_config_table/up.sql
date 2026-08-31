-- Migration v33: create_system_config_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_system_config_table"（T2.6.1 兼容性设计）
-- 说明: 系统配置 KV 表，config_key 唯一约束

CREATE TABLE IF NOT EXISTS system_config (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    config_key TEXT NOT NULL UNIQUE,
    config_value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
