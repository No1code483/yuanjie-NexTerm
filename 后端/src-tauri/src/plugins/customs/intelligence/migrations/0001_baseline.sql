-- customs.intelligence 基线迁移（批次6c S1）：补登记既有表归属。
-- 表 DDL 由主应用 migrations v61-v64 创建，此处仅补 kernel_table_ownership。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('activity_logs', 'customs.intelligence', 0, datetime('now')),
    ('suggestions', 'customs.intelligence', 0, datetime('now')),
    ('behavior_patterns', 'customs.intelligence', 0, datetime('now')),
    ('intelligence_settings', 'customs.intelligence', 0, datetime('now'));
