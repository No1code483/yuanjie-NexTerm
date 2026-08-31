-- Migration v30: create_news_cache_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + 内部调用 migrate_news_cache_table 动态补字段）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_news_cache_table"（T2.6.1 兼容性设计）
-- 说明: 新闻缓存表。本 .sql 仅包含 CREATE TABLE 静态部分；
--       动态字段补丁（content/category/published_at/is_favorite/ai_summary）
--       由 inline 函数 migrate_news_cache_table 通过 PRAGMA + ALTER TABLE 完成。

CREATE TABLE IF NOT EXISTS news_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    url TEXT,
    source TEXT,
    summary TEXT,
    content TEXT,
    category TEXT,
    published_at TEXT,
    fetched_at INTEGER NOT NULL,
    is_read INTEGER NOT NULL DEFAULT 0
);
