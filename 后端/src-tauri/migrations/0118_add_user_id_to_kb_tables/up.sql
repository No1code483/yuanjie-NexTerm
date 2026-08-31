-- Migration v118: add_user_id_to_kb_tables
-- 类型: Static ALTER TABLE（多用户数据隔离批次 3）
-- 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
-- 说明:
--   给知识库模块所有用户私有表添加 user_id 字段，实现多用户数据隔离。
--   DEFAULT 1 保证现有 admin 数据自动归属 user_id=1。
--   - kb_categories: 分类树（含 parent_id 递归结构）
--   - kb_entries: 知识条目主表
--   - kb_tags: 标签
--   - kb_entry_tags: 条目-标签关联（冗余 user_id 简化过滤）
--   - kb_recent_access: 最近访问记录
--   - kb_tracked_paths: 追踪路径
--   - kb_references: 条目间引用关系
--   - kb_snapshots: 内容快照
--   - kb_templates: 模板（user_id=0 表示系统内置）
--   - kb_attachment_cache: 附件缓存

ALTER TABLE kb_categories ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_kb_categories_user_id ON kb_categories(user_id);
CREATE INDEX IF NOT EXISTS idx_kb_categories_user_library ON kb_categories(user_id, library);

ALTER TABLE kb_entries ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_kb_entries_user_id ON kb_entries(user_id);
CREATE INDEX IF NOT EXISTS idx_kb_entries_user_cat ON kb_entries(user_id, category_id);
CREATE INDEX IF NOT EXISTS idx_kb_entries_user_updated ON kb_entries(user_id, updated_at);
CREATE INDEX IF NOT EXISTS idx_kb_entries_user_fav ON kb_entries(user_id, is_favorited);

ALTER TABLE kb_tags ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_kb_tags_user_id ON kb_tags(user_id);

ALTER TABLE kb_entry_tags ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_kb_entry_tags_user_id ON kb_entry_tags(user_id);
CREATE INDEX IF NOT EXISTS idx_kb_entry_tags_user_entry ON kb_entry_tags(user_id, entry_id);

ALTER TABLE kb_recent_access ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_kb_recent_access_user_id ON kb_recent_access(user_id);
CREATE INDEX IF NOT EXISTS idx_kb_recent_access_user_entry ON kb_recent_access(user_id, entry_id);

ALTER TABLE kb_tracked_paths ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_kb_tracked_paths_user_id ON kb_tracked_paths(user_id);
CREATE INDEX IF NOT EXISTS idx_kb_tracked_paths_user_path ON kb_tracked_paths(user_id, path);

ALTER TABLE kb_references ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_kb_references_user_id ON kb_references(user_id);
CREATE INDEX IF NOT EXISTS idx_kb_references_user_source ON kb_references(user_id, source_entry_id);

ALTER TABLE kb_snapshots ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_kb_snapshots_user_id ON kb_snapshots(user_id);
CREATE INDEX IF NOT EXISTS idx_kb_snapshots_user_entry ON kb_snapshots(user_id, entry_id);

ALTER TABLE kb_templates ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_kb_templates_user_id ON kb_templates(user_id);

ALTER TABLE kb_attachment_cache ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_kb_attachment_cache_user_id ON kb_attachment_cache(user_id);
CREATE INDEX IF NOT EXISTS idx_kb_attachment_cache_user_entry ON kb_attachment_cache(user_id, entry_id);
