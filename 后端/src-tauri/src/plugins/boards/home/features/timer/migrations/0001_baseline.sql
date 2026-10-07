-- home.timer 基线迁移（批次1b-2a）：只登记既有表归属，不重放历史表 DDL。
-- timers 由主应用迁移 v24 创建、v120 补 user_id 列（INTEGER NOT NULL DEFAULT 1），
-- 此处只补登记，供 registry.rs::register_manifest_tables 校验（旧表须先登记后登记 manifest）。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('timers', 'home.timer', 0, datetime('now'));
