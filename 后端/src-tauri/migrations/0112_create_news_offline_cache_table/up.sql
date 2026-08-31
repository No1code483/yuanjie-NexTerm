-- Migration v112: create_news_offline_cache_table
-- 类型: Static CREATE TABLE
-- 规范: 功能展望/平台级增强/04_离线与同步机制.md §Phase 3 Task 3（新闻源离线缓存）
-- hash 源: 函数名 "create_news_offline_cache_table"（T2.6.1 兼容性设计）
-- 说明: A5 离线同步 Phase 3 Task 3 新闻源离线缓存专用表。
--       与现有 news_cache（v30，主存储）解耦：news_cache 是 RSS 拉取后的主存储，
--       news_offline_cache 是离线场景的快照副本（包含 cached_at 写入时间戳）。
--       在线时 fetch_and_cache_news 成功后同步写入本表；离线时从本表读取展示。
--       id 使用 TEXT（RSS item guid/link 哈希），避免与 news_cache 的 INTEGER 主键冲突。

CREATE TABLE IF NOT EXISTS news_offline_cache (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT,
    url TEXT,
    published_at TEXT,
    fetched_at TEXT NOT NULL,
    cached_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_news_offline_cache_source ON news_offline_cache(source);
CREATE INDEX IF NOT EXISTS idx_news_offline_cache_cached_at ON news_offline_cache(cached_at);
