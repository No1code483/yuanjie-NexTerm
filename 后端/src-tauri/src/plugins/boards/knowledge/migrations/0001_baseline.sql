-- boards.knowledge 基线迁移（批次2a-1）：只登记既有表归属，不重放历史表 DDL。
-- 10 张 kb_* 表由主应用迁移创建（0009—0113 + 0084/0085 表重建 + v118 补 user_id 列），
-- 此处只补登记，供 registry.rs::register_manifest_tables 校验（旧表须先登记后登记 manifest）。
-- ⚠️ name_prefixed 必须为 1：内核按「表名是否以本插件短码 `kb_` 开头」推导该标志
--    （registry.rs:99 `table.starts_with(&prefix)`），kb_* 天然命中；登记为 0 会被
--    db.rs::register_table 判为「已登记为 0，不能改写为 1」并导致插件整体激活失败。
-- ⚠️ kb_categories.id 被 game_kb_category_mapping 逻辑引用（无物理 FK），不得改值。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('kb_categories',       'boards.knowledge', 1, datetime('now')),
    ('kb_entries',          'boards.knowledge', 1, datetime('now')),
    ('kb_tags',             'boards.knowledge', 1, datetime('now')),
    ('kb_entry_tags',       'boards.knowledge', 1, datetime('now')),
    ('kb_recent_access',    'boards.knowledge', 1, datetime('now')),
    ('kb_tracked_paths',    'boards.knowledge', 1, datetime('now')),
    ('kb_templates',        'boards.knowledge', 1, datetime('now')),
    ('kb_references',       'boards.knowledge', 1, datetime('now')),
    ('kb_snapshots',        'boards.knowledge', 1, datetime('now')),
    ('kb_attachment_cache', 'boards.knowledge', 1, datetime('now'));
