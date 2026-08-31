-- A5.1 sync_queue 表：变更捕获与同步队列
-- 设计文档：功能展望/平台级增强/04_离线与同步机制.md §2.2
-- 用途：记录本地数据变更（INSERT/UPDATE/DELETE），待同步到远端
-- 触发方式：SQLite 触发器自动捕获（Phase 2 实现）或应用层显式调用 sync_log()
CREATE TABLE IF NOT EXISTS sync_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,                 -- 变更的表名（如 todos / kb_entries）
    record_id INTEGER NOT NULL,               -- 变更记录的 ID
    operation TEXT NOT NULL CHECK (operation IN ('INSERT', 'UPDATE', 'DELETE')),
    payload TEXT NOT NULL,                    -- JSON: 变更后的完整数据（DELETE 时为变更前数据）
    vector_clock TEXT,                        -- 向量时钟 JSON（设备 ID + 版本号）
    device_id TEXT,                           -- 产生此变更的设备 ID
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    synced_at TEXT,                           -- 同步完成时间（NULL 表示未同步）
    sync_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (sync_status IN ('pending', 'syncing', 'synced', 'failed', 'conflict')),
    retry_count INTEGER DEFAULT 0,
    last_error TEXT                           -- 最近一次同步失败的错误信息
);

-- 按状态 + 创建时间索引：调度器查询 pending 队列
CREATE INDEX IF NOT EXISTS idx_sync_queue_status ON sync_queue(sync_status, created_at);

-- 按表名 + 记录 ID 索引：查询某条记录的同步历史
CREATE INDEX IF NOT EXISTS idx_sync_queue_record ON sync_queue(table_name, record_id);

-- 按设备 ID 索引：查询某设备的变更
CREATE INDEX IF NOT EXISTS idx_sync_queue_device ON sync_queue(device_id);
