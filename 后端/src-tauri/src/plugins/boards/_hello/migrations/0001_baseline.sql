-- _hello 基线迁移（阶段1 B8）
-- 演示表：前缀 hw_ 与 manifest.permissions.db / SHORT_CODES("_hello" -> "hw") 对齐
CREATE TABLE IF NOT EXISTS hw_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
