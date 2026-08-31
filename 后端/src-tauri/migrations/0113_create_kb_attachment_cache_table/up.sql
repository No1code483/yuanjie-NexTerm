-- Migration v113: create_kb_attachment_cache_table
-- 类型: Static CREATE TABLE
-- 规范: 功能展望/平台级增强/04_离线与同步机制.md §Phase 3 Task 4（知识库附件预加载）
-- hash 源: 函数名 "create_kb_attachment_cache_table"（T2.6.1 兼容性设计）
-- 说明: A5 离线同步 Phase 3 Task 4 知识库附件预加载专用表。
--       记录被标记「常用」的 KB 附件，并将文件副本预加载到 app_data_dir/attachment_cache/。
--       离线打开附件时从 local_cache_path 读取，无需访问原始文件路径。
--       LRU 淘汰：默认 500MB + 30 天（由 service 层 evict_lru 实现）。
--       FK 引用 kb_entries(id) ON DELETE CASCADE：条目删除时自动级联清理缓存记录。

CREATE TABLE IF NOT EXISTS kb_attachment_cache (
    id TEXT PRIMARY KEY,
    entry_id INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    pinned_at TEXT NOT NULL,
    last_accessed_at TEXT NOT NULL,
    local_cache_path TEXT NOT NULL,
    FOREIGN KEY (entry_id) REFERENCES kb_entries(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_kb_attachment_cache_entry ON kb_attachment_cache(entry_id);
CREATE INDEX IF NOT EXISTS idx_kb_attachment_cache_last_accessed ON kb_attachment_cache(last_accessed_at);
