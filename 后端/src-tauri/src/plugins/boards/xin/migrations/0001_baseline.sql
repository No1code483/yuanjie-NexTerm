-- boards.xin 基线迁移（批次4a-1）：只登记既有表归属，不重放历史表 DDL。
-- xin_config / xin_memories / xin_summaries / xin_moods / xin_persona_memories /
-- xin_persona_switch_log 由主应用历史迁移创建（含多用户隔离批次补 user_id 列）；
-- 此处只补登记，供 registry.rs::register_manifest_tables 校验（旧表须先登记后登记 manifest）。
-- 前缀口径：L1 短码 `xn` + `_` = `xn_`，六表均以 `xin_` 开头 → name_prefixed=0（裁定 2）。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('xin_config',             'boards.xin', 0, datetime('now')),
    ('xin_memories',           'boards.xin', 0, datetime('now')),
    ('xin_summaries',          'boards.xin', 0, datetime('now')),
    ('xin_moods',              'boards.xin', 0, datetime('now')),
    ('xin_persona_memories',   'boards.xin', 0, datetime('now')),
    ('xin_persona_switch_log', 'boards.xin', 0, datetime('now'));