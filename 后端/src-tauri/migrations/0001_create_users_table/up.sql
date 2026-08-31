-- Migration v1: create_users_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_users_table"（T2.6.1 兼容性设计）
-- 说明: 用户主表，存储账号凭证与扩展资料字段

CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    salt BLOB NOT NULL,
    encrypted_mek BLOB NOT NULL,
    mek_nonce BLOB NOT NULL,
    recovery_phrase_hash TEXT,
    is_permanent INTEGER NOT NULL DEFAULT 1,
    expires_at INTEGER,
    role TEXT NOT NULL DEFAULT 'admin',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    avatar_url TEXT,
    bio TEXT,
    display_name TEXT
);
