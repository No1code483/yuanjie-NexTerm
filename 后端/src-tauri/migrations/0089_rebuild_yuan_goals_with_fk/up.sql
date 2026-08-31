-- Migration v89: rebuild_yuan_goals_with_fk
-- 类型: 动态（表重建 - SQLite 不支持 ALTER TABLE ADD FOREIGN KEY）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.2.2（T2.7.4 P3 元界表 FK 补建）
-- hash 源: 函数名 "rebuild_yuan_goals_with_fk"（T2.6.1 兼容性设计）
-- 说明: 重建 yuan_goals 表，添加 1 条自引用外键约束
--       - parent_goal_id → yuan_goals(id) ON DELETE SET NULL
-- 表重建模式: CREATE _new → INSERT → DROP old → RENAME
-- 自引用 FK 处理: INSERT ORDER BY id ASC（确保父节点先插入）
-- 数据清理: 删除 parent_goal_id 引用了不存在 id 的孤儿记录（防御性）
-- 注意: 此迁移在 Rust 函数中用事务 + PRAGMA defer_foreign_keys=ON 执行，
--       以处理自引用 FK 在 INSERT 时的顺序依赖问题

-- 1. 清理孤儿数据（自引用：parent_goal_id 引用了不存在的 yuan_goals.id）
DELETE FROM yuan_goals WHERE parent_goal_id IS NOT NULL AND parent_goal_id NOT IN (SELECT id FROM yuan_goals);

-- 2. 创建带自引用 FK 的新表
CREATE TABLE yuan_goals_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    goal_uuid TEXT NOT NULL UNIQUE,
    session_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 0,
    progress_pct INTEGER NOT NULL DEFAULT 0,
    parent_goal_id INTEGER,
    agent_id TEXT,
    token_budget INTEGER,
    tokens_used INTEGER NOT NULL DEFAULT 0,
    thread_json TEXT NOT NULL DEFAULT '{}',
    result_json TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (parent_goal_id) REFERENCES yuan_goals_new(id) ON DELETE SET NULL
);

-- 3. 迁移数据（按 id ASC 顺序，确保父节点先插入；defer_foreign_keys 已在 tx 中启用）
INSERT INTO yuan_goals_new (id, goal_uuid, session_id, title, description, status, priority, progress_pct, parent_goal_id, agent_id, token_budget, tokens_used, thread_json, result_json, created_at, updated_at)
SELECT id, goal_uuid, session_id, title, description, status, priority, progress_pct, parent_goal_id, agent_id, token_budget, tokens_used, thread_json, result_json, created_at, updated_at
FROM yuan_goals ORDER BY id ASC;

-- 4. 删除旧表
DROP TABLE yuan_goals;

-- 5. 重命名新表（SQLite 会自动更新自引用 FK 指向新表名 yuan_goals）
ALTER TABLE yuan_goals_new RENAME TO yuan_goals;

-- 6. 重建索引（yuan_goals 无额外索引，PK 和 UNIQUE 由 SQLite 自动管理）
