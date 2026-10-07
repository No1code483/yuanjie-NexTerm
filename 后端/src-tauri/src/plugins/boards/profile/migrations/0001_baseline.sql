-- boards.profile 基线迁移：只登记既有表归属，不重放历史表 DDL。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('user_profiles', 'boards.profile', 0, datetime('now')),
    ('resumes', 'boards.profile', 0, datetime('now')),
    ('quotes', 'boards.profile', 0, datetime('now'));
