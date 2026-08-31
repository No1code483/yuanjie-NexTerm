-- Migration v26: create_user_profiles_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_user_profiles_table"（T2.6.1 兼容性设计）
-- 说明: 用户扩展信息 KV 表，field_key 唯一约束

CREATE TABLE IF NOT EXISTS user_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    field_key TEXT NOT NULL UNIQUE,
    field_value TEXT,
    updated_at INTEGER NOT NULL
);
