-- customs.sync 基线迁移（批次6b S1）：只登记既有表归属，不重放历史表 DDL。
-- sync_queue（migration 0097）、sync_devices（migration 0098）由主应用迁移创建，此处只补登记。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('sync_queue', 'customs.sync', 0, datetime('now')),
    ('sync_devices', 'customs.sync', 0, datetime('now'));
