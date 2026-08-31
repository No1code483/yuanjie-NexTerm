-- Migration v90: create_mek_versions_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/05_安全加固_A4.md §2.3.2
-- hash 源: 函数名 "create_mek_versions_table"（T2.6.1 兼容性设计）
-- 说明: MEK 版本管理表，支持密钥轮换 + 多版本共存 + 渐进迁移
--       T2.13 密钥轮换机制的核心数据结构

CREATE TABLE IF NOT EXISTS mek_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,                           -- 用户标识（兼容 users.id TEXT 类型）
    version INTEGER NOT NULL,                        -- MEK 版本号（从 1 开始递增）
    encrypted_mek BLOB NOT NULL,                     -- 用 KEK 加密后的 MEK 密文
    nonce BLOB NOT NULL,                             -- AES-GCM nonce（12 字节）
    created_at INTEGER NOT NULL,                     -- 创建时间（Unix timestamp 秒）
    rotated_at INTEGER,                              -- 轮换时间（NULL 表示当前活跃版本）
    is_active INTEGER NOT NULL DEFAULT 0,            -- 1=当前使用，0=历史版本（SQLite 无 BOOLEAN）
    rotated_from INTEGER,                            -- 上一版本号（NULL 表示初始版本）
    UNIQUE(user_id, version)
);

CREATE INDEX IF NOT EXISTS idx_mek_versions_user_active ON mek_versions(user_id, is_active);
CREATE INDEX IF NOT EXISTS idx_mek_versions_user_created ON mek_versions(user_id, created_at DESC);
