-- home.news 补列迁移（批次1b-2b-2 · 越权面收口）：为两张历史表补隔离列 user_id。
-- news_pending_delete  由主应用 v31 创建、news_offline_cache 由 v112 创建，二者均未被 v121 覆盖。
-- 裁定 19-A / 20-A：沿用 v121 既有 14 表同法（ALTER ADD COLUMN + 索引），主键形态保持不变。
-- 存量行默认归属首个用户（DEFAULT 1，与 v121 一致）；此后新写入由服务层显式绑定会话主体。
ALTER TABLE news_pending_delete ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_news_pending_delete_user_id ON news_pending_delete(user_id);
ALTER TABLE news_offline_cache ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_news_offline_cache_user_id ON news_offline_cache(user_id);
