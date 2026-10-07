-- customs.auth 基线迁移（批次1a-2a：crypto/MEK 收编）：登记既有表归属，不重放历史表 DDL。
-- mek_versions / mek_rotation_log 由主应用迁移 v90 / v91 创建，此处只补登记，
-- 供 registry.rs::register_manifest_tables 校验（旧表须先登记后登记 manifest）。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('mek_versions', 'customs.auth', 0, datetime('now')),
    ('mek_rotation_log', 'customs.auth', 0, datetime('now'));
