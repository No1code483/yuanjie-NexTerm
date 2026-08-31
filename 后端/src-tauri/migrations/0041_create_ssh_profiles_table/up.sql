-- Migration v41: create_ssh_profiles_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_ssh_profiles_table"（T2.6.1 兼容性设计）
-- 说明: SSH 连接配置表，存储远程主机连接信息（认证方式、密钥路径等）

CREATE TABLE IF NOT EXISTS ssh_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 22,
    username TEXT NOT NULL,
    auth_type TEXT NOT NULL DEFAULT 'password',
    private_key_path TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
