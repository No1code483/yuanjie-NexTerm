-- boards.terminal 基线迁移（批次3a）：只登记既有表归属，不重放历史表 DDL。
-- terminal_history（v34）/ terminal_tab_layout（v36）/ terminal_sessions（v38 + v39 补列
-- + v86 重建含 FK）/ ssh_profiles（v41）/ terminal_config（v42）由主应用迁移创建；
-- terminal_history_fts（v40）为 FTS5 虚表（+2 触发器同步 terminal_history），一并登记归属。
-- 此处只补登记，供 registry.rs::register_manifest_tables 校验（旧表须先登记后登记 manifest）。
-- 前缀口径：短码 `tm` + `_` = `tm_`，六表均不以 `tm_` 开头 → name_prefixed=0。
-- ⚠️ FTS5 虚表仅登记归属行（不重放 DDL）；其数据同步由触发器绑定 terminal_history，
--    不得对该虚表单独写入/重建。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('terminal_sessions',     'boards.terminal', 0, datetime('now')),
    ('terminal_history',      'boards.terminal', 0, datetime('now')),
    ('terminal_history_fts',  'boards.terminal', 0, datetime('now')),
    ('terminal_tab_layout',   'boards.terminal', 0, datetime('now')),
    ('terminal_config',       'boards.terminal', 0, datetime('now')),
    ('ssh_profiles',          'boards.terminal', 0, datetime('now'));
