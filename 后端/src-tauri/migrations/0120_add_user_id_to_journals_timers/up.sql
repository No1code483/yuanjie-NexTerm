-- Migration v120: add_user_id_to_journals_timers
-- 类型: 表重建（journals）+ Static ALTER TABLE（timers）（多用户数据隔离批次 5）
-- 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
-- 说明:
--   给日程类 2 张表添加 user_id 字段，实现多用户数据隔离。
--   DEFAULT 1 保证现有 admin 数据自动归属 user_id=1。
--
--   journals 表特殊处理：
--     原表有 UNIQUE(date) 约束，多用户场景下两个用户无法在同一天写日记。
--     SQLite ALTER TABLE 不支持修改 UNIQUE 约束，必须重建表。
--     重建后约束改为 UNIQUE(user_id, date)，支持多用户同一天各写各的。
--     现有数据 user_id=1，date 已唯一 → 重建后无冲突。
--
--   timers 表无 UNIQUE 约束，直接 ALTER ADD COLUMN 即可。

-- ===== journals 表：重建以改 UNIQUE(date) → UNIQUE(user_id, date) =====
DROP TABLE IF EXISTS journals_new;

CREATE TABLE journals_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL DEFAULT 1,
    date TEXT NOT NULL,
    content TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(user_id, date)
);

INSERT INTO journals_new (user_id, date, content, created_at, updated_at)
SELECT 1, date, content, created_at, updated_at FROM journals;

DROP TABLE journals;
ALTER TABLE journals_new RENAME TO journals;

CREATE INDEX IF NOT EXISTS idx_journals_user_id ON journals(user_id);
CREATE INDEX IF NOT EXISTS idx_journals_user_date ON journals(user_id, date);

-- ===== timers 表：无 UNIQUE 约束，直接 ALTER =====
ALTER TABLE timers ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_timers_user_id ON timers(user_id);
