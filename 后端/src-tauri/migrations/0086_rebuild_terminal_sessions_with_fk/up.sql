-- Migration v86: rebuild_terminal_sessions_with_fk
-- 类型: 动态（表重建 - SQLite 不支持 ALTER TABLE ADD FOREIGN KEY）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.2.2（T2.7.4 P2 终端表 FK 补建）
-- hash 源: 函数名 "rebuild_terminal_sessions_with_fk"（T2.6.1 兼容性设计）
-- 说明: 重建 terminal_sessions 表，添加 1 条外键约束
--       - tab_id → terminal_tab_layout(tab_id) ON DELETE SET NULL
-- 表重建模式: CREATE _new → INSERT → DROP old → RENAME → 重建索引
-- 数据清理: 不需要（tab_id 是 nullable，ON DELETE SET NULL 兼容孤儿数据）

-- 0. 清理可能的残留表（上次迁移失败时可能残留 _new 表）
DROP TABLE IF EXISTS terminal_sessions_new;

-- 1. 创建带 FK 的新表
CREATE TABLE terminal_sessions_new (
    id TEXT PRIMARY KEY,
    session_type TEXT NOT NULL DEFAULT 'cmd',
    tab_id TEXT,
    pane_id TEXT,
    cols INTEGER NOT NULL DEFAULT 80,
    rows INTEGER NOT NULL DEFAULT 24,
    status TEXT NOT NULL DEFAULT 'active',
    created_at INTEGER NOT NULL,
    killed_at INTEGER,
    FOREIGN KEY (tab_id) REFERENCES terminal_tab_layout(tab_id) ON DELETE SET NULL
);

-- 2. 迁移数据
INSERT INTO terminal_sessions_new (id, session_type, tab_id, pane_id, cols, rows, status, created_at, killed_at)
SELECT id, session_type, tab_id, pane_id, cols, rows, status, created_at, killed_at FROM terminal_sessions;

-- 3. 删除旧表
DROP TABLE terminal_sessions;

-- 4. 重命名新表
ALTER TABLE terminal_sessions_new RENAME TO terminal_sessions;

-- 5. 重建索引（原 v66 create_indexes 中定义的 2 个索引）
CREATE INDEX IF NOT EXISTS idx_terminal_sessions_tab ON terminal_sessions(tab_id);
CREATE INDEX IF NOT EXISTS idx_terminal_sessions_pane ON terminal_sessions(pane_id);
