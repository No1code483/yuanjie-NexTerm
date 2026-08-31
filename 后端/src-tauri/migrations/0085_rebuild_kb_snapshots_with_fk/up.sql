-- Migration v85: rebuild_kb_snapshots_with_fk
-- 类型: 动态（表重建 - SQLite 不支持 ALTER TABLE ADD FOREIGN KEY）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.2.2（T2.7.4 P1 知识库表 FK 补建）
-- hash 源: 函数名 "rebuild_kb_snapshots_with_fk"（T2.6.1 兼容性设计）
-- 说明: 重建 kb_snapshots 表，添加 1 条外键约束
--       - entry_id → kb_entries(id) ON DELETE CASCADE
-- 表重建模式: CREATE _new → INSERT → DROP old → RENAME
-- 数据清理: 删除引用了不存在 kb_entries.id 的孤儿记录（防御性）

-- 1. 清理孤儿数据（防御性，确保 INSERT 不会因 FK 检查失败）
DELETE FROM kb_snapshots WHERE entry_id NOT IN (SELECT id FROM kb_entries);

-- 2. 创建带 FK 的新表
CREATE TABLE kb_snapshots_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entry_id INTEGER NOT NULL,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (entry_id) REFERENCES kb_entries(id) ON DELETE CASCADE
);

-- 3. 迁移数据
INSERT INTO kb_snapshots_new (id, entry_id, content, created_at)
SELECT id, entry_id, content, created_at FROM kb_snapshots;

-- 4. 删除旧表
DROP TABLE kb_snapshots;

-- 5. 重命名新表
ALTER TABLE kb_snapshots_new RENAME TO kb_snapshots;

-- 6. 重建索引（kb_snapshots 无额外索引，PK 由 SQLite 自动管理）
