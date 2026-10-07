-- home.todo 基线迁移（批次1b-1）：只登记既有表归属，不重放历史表 DDL。
-- todos 由主应用迁移 v22（create_todos_table 静态 DDL + inline migrate_todos_table 补 5 列）创建，
-- 此处只补登记，供 registry.rs::register_manifest_tables 校验（旧表须先登记后登记 manifest）。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('todos', 'home.todo', 0, datetime('now'));
