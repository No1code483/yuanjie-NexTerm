-- Migration v122: add_user_id_to_terminal_history_tab_layout
-- 类型: Static ALTER TABLE（多用户数据隔离补充修复 — 安全审计再次检查发现）
-- 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
-- 说明:
--   terminal_history 和 terminal_tab_layout 两张表在批次 6 中被遗漏，
--   完全没有 user_id 隔离。terminal_history 存储用户执行的命令和输出，
--   属于高敏感数据（可能含密码/密钥），必须按用户隔离。
--
--   两张表均无 UNIQUE 约束，直接 ALTER ADD COLUMN 即可。
--   DEFAULT 1 保证现有数据自动归属 user_id=1（admin）。

-- ===== terminal_history: 终端命令历史（高敏感） =====
ALTER TABLE terminal_history ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_terminal_history_user_id ON terminal_history(user_id);
CREATE INDEX IF NOT EXISTS idx_terminal_history_user_created ON terminal_history(user_id, created_at);

-- ===== terminal_tab_layout: 终端标签布局 =====
ALTER TABLE terminal_tab_layout ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_terminal_tab_layout_user_id ON terminal_tab_layout(user_id);
