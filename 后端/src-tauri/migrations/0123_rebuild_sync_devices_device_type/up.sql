-- Migration v123: rebuild_sync_devices_device_type
-- 类型: 动态（表重建 - SQLite 不支持 ALTER TABLE 修改 CHECK 约束）
-- 来源: BUG-018（阶段0 集成测试 sync_integration 15/5 暴露）
-- 说明: device_type 语义冲突修复（用户裁决方案 C）
--       - device_type: 设备形态（desktop/laptop/phone/tablet/server），CHECK 改为形态枚举
--       - device_os:   操作系统（windows/macos/linux/other），新增列（宽松，无 CHECK）
-- 表重建模式: CREATE _new → INSERT → DROP old → RENAME → 重建索引

-- 0. 清理可能的残留表（上次迁移失败时可能残留 _new 表）
DROP TABLE IF EXISTS sync_devices_new;

-- 1. 创建新表（形态枚举 + 操作系统列）
CREATE TABLE sync_devices_new (
    id TEXT PRIMARY KEY,                      -- UUID（设备唯一标识）
    device_name TEXT NOT NULL,                -- 用户可读名称（如 "工作笔记本"）
    device_type TEXT NOT NULL DEFAULT 'desktop' CHECK (device_type IN ('desktop', 'laptop', 'phone', 'tablet', 'server')),
    device_os TEXT NOT NULL DEFAULT 'other',  -- 操作系统（windows/macos/linux/other）
    public_key TEXT NOT NULL,                 -- ECDH 公钥（用于端到端加密）
    registered_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen_at TEXT,                        -- 最后活跃时间
    last_sync_at TEXT,                        -- 最后成功同步时间
    is_current_device INTEGER DEFAULT 0       -- 1=当前设备，0=其他设备
);

-- 2. 迁移数据：旧 device_type 为 OS 枚举 → 归入新 device_os；device_type 无形态信息 → 默认 'desktop'
INSERT INTO sync_devices_new (id, device_name, device_type, device_os, public_key, registered_at, last_seen_at, last_sync_at, is_current_device)
SELECT id, device_name, 'desktop', device_type, public_key, registered_at, last_seen_at, last_sync_at, is_current_device
FROM sync_devices;

-- 3. 删除旧表
DROP TABLE sync_devices;

-- 4. 重命名新表
ALTER TABLE sync_devices_new RENAME TO sync_devices;

-- 5. 重建索引（原 0098 定义的索引）
CREATE INDEX IF NOT EXISTS idx_sync_devices_current ON sync_devices(is_current_device);