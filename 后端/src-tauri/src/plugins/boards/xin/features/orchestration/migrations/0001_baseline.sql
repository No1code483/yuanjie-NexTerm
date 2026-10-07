-- boards.xin 编排面 4a-2 基线迁移：只登记既有表归属，不重放历史表 DDL。
-- xin_conversations / xin_checkpoints / xin_compaction_config / xin_compaction_records
-- 由主应用历史迁移创建（xin_conversations 经 0119_add_user_id_up.sql 补 user_id；
-- xin_checkpoints / xin_compaction_* 经 0087/0088 rebuild_fk 添加外键约束）；
-- 此处只补登记，供 registry.rs::register_manifest_tables 校验（旧表须先登记后登记 manifest）。
-- 前缀口径：L2 短码 `xo` + `_` = `xo_`，四表均以 `xin_` 开头 → name_prefixed=0（裁定 2）。

INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('xin_conversations',      'xin.orchestration', 0, datetime('now')),
    ('xin_checkpoints',        'xin.orchestration', 0, datetime('now')),
    ('xin_compaction_config',  'xin.orchestration', 0, datetime('now')),
    ('xin_compaction_records', 'xin.orchestration', 0, datetime('now'));
