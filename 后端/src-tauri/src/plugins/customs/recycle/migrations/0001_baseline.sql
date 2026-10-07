-- customs.recycle 基线迁移（批次6a S1）：只登记既有表归属，不重放历史表 DDL。
-- recycle_bin 表由主应用迁移 v25 创建，此处只补登记。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('recycle_bin', 'customs.recycle', 0, datetime('now'));
