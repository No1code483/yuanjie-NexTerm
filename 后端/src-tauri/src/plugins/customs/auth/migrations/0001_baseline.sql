-- customs.auth 基线迁移（批次1a-1）：只登记既有表归属，不重放历史表 DDL。
-- mek_versions / mek_rotation_log 的归属登记在 1a-2（crypto 收编）批次追加。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('users', 'customs.auth', 0, datetime('now')),
    ('permissions', 'customs.auth', 0, datetime('now')),
    ('auth_sessions', 'customs.auth', 0, datetime('now'));
