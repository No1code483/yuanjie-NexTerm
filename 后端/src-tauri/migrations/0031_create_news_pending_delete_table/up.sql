-- Migration v31: create_news_pending_delete_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_news_pending_delete_table"（T2.6.1 兼容性设计）
-- 说明: 新闻待删除回收表，记录 original_fetched_at 与 moved_at 用于自动清理

CREATE TABLE IF NOT EXISTS news_pending_delete (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    url TEXT,
    source TEXT,
    summary TEXT,
    content TEXT,
    category TEXT,
    published_at TEXT,
    original_fetched_at INTEGER NOT NULL,
    moved_at INTEGER NOT NULL,
    is_read INTEGER NOT NULL DEFAULT 0,
    is_favorite INTEGER NOT NULL DEFAULT 0
);
