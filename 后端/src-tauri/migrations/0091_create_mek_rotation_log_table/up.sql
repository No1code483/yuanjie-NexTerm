-- Migration v91: create_mek_rotation_log_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/05_安全加固_A4.md §2.3.2
-- hash 源: 函数名 "create_mek_rotation_log_table"（T2.6.1 兼容性设计）
-- 说明: MEK 轮换历史日志表，记录每次轮换的元数据
--       用于审计 + 故障排查（如密文迁移失败时定位）

CREATE TABLE IF NOT EXISTS mek_rotation_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,                           -- 用户标识
    from_version INTEGER,                            -- 原版本号（NULL 表示首次创建）
    to_version INTEGER NOT NULL,                     -- 新版本号
    rotated_at INTEGER NOT NULL,                     -- 轮换时间（Unix timestamp 秒）
    trigger_reason TEXT NOT NULL,                    -- 'scheduled' | 'manual' | 'emergency'
    migrated_records INTEGER NOT NULL DEFAULT 0,    -- 迁移的密文记录数
    duration_ms INTEGER NOT NULL DEFAULT 0,          -- 轮换耗时（毫秒）
    status TEXT NOT NULL DEFAULT 'completed',       -- 'in_progress' | 'completed' | 'failed'
    error_message TEXT                               -- 失败原因（NULL 表示成功）
);

CREATE INDEX IF NOT EXISTS idx_mek_rotation_log_user ON mek_rotation_log(user_id, rotated_at DESC);
