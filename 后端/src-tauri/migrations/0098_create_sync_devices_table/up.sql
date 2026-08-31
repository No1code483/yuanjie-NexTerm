-- A5.6 sync_devices 表：已注册设备管理
-- 设计文档：功能展望/平台级增强/04_离线与同步机制.md §2.6
-- 用途：记录所有已注册的同步设备，支持设备间 E2E 加密密钥交换
CREATE TABLE IF NOT EXISTS sync_devices (
    id TEXT PRIMARY KEY,                      -- UUID（设备唯一标识）
    device_name TEXT NOT NULL,                -- 用户可读名称（如 "工作笔记本"）
    device_type TEXT NOT NULL CHECK (device_type IN ('windows', 'macos', 'linux')),
    public_key TEXT NOT NULL,                 -- ECDH 公钥（用于端到端加密）
    registered_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen_at TEXT,                       -- 最后活跃时间
    last_sync_at TEXT,                       -- 最后成功同步时间
    is_current_device INTEGER DEFAULT 0      -- 1=当前设备，0=其他设备
);

-- 查询当前设备的索引
CREATE INDEX IF NOT EXISTS idx_sync_devices_current ON sync_devices(is_current_device);
