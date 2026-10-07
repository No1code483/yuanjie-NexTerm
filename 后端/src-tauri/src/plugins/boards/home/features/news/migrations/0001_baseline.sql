-- home.news 基线迁移（批次1b-2b-1）：只登记既有表归属，不重放历史表 DDL。
-- news_cache           由主应用迁移 v30 创建、v121 补 user_id 列（INTEGER NOT NULL DEFAULT 1）；
-- news_pending_delete  由 v31 创建（v121 未覆盖 → 无 user_id，补列属批次 1b-2b-2）；
-- news_sources         由 v32 创建 + 种子数据、v121 重建并补 user_id；
-- news_offline_cache   由 v112 创建（v121 未覆盖 → 无 user_id，补列属批次 1b-2b-2）。
-- 此处只补登记，供 registry.rs::register_manifest_tables 校验（旧表须先登记后登记 manifest）。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('news_cache', 'home.news', 0, datetime('now')),
    ('news_pending_delete', 'home.news', 0, datetime('now')),
    ('news_sources', 'home.news', 0, datetime('now')),
    ('news_offline_cache', 'home.news', 0, datetime('now'));
