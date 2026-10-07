-- home.journal 基线迁移（批次1b-1）：只登记既有表归属，不重放历史表 DDL。
-- journals 由主应用迁移 v23 创建、v120 重建（UNIQUE(date) → UNIQUE(user_id,date)），
-- 此处只补登记，供 registry.rs::register_manifest_tables 校验（旧表须先登记后登记 manifest）。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('journals', 'home.journal', 0, datetime('now'));
