-- Migration v32: create_news_sources_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + 内部调用 seed_default_news_sources 种子数据）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_news_sources_table"（T2.6.1 兼容性设计）
-- 说明: 新闻源配置表。本 .sql 仅包含 CREATE TABLE 静态部分；
--       种子数据（26 个默认新闻源：网安/漏洞/攻防/工具/技术创新/云原生/开源/HN 频道）
--       由 inline 函数 seed_default_news_sources 在首次创建时通过 INSERT OR IGNORE 插入。

CREATE TABLE IF NOT EXISTS news_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    url TEXT NOT NULL UNIQUE,
    category TEXT NOT NULL DEFAULT 'security',
    feed_type TEXT NOT NULL DEFAULT 'rss'
);
