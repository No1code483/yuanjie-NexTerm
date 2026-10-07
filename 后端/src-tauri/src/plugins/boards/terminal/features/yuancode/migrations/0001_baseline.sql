-- terminal.yuancode 基线迁移（批次3c + 5e）：只登记既有编辑器核心表归属，不重放历史表 DDL。
-- 6 张表由 3c 主应用迁移创建：yuan_code_snippets（v48）/ yuan_code_workspaces（v49）。
-- 8 张表由 5e 主应用迁移创建：api_keys（v109）/ model_routing_rules（v110）/
--   project_index_projects 等 6 张 project_index_*（v92）。
-- 此处只补登记，供 registry.rs::register_manifest_tables 校验（旧表须先登记后登记 manifest）。
-- 前缀口径：短码 `yc` + `_` = `yc_`，所有 14 张表均不以 `yc_` 开头 → name_prefixed=0
--   （既有表不改名，同 ai_models / terminal_* 先例：档案 §10.3 表归属）。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('yuan_code_snippets',      'terminal.yuancode', 0, datetime('now')),
    ('yuan_code_workspaces',    'terminal.yuancode', 0, datetime('now')),
    ('editor_documents',        'terminal.yuancode', 0, datetime('now')),
    ('editor_versions',         'terminal.yuancode', 0, datetime('now')),
    ('editor_sessions',         'terminal.yuancode', 0, datetime('now')),
    ('distill_dataset',         'terminal.yuancode', 0, datetime('now')),
    ('api_keys',                'terminal.yuancode', 0, datetime('now')),
    ('model_routing_rules',     'terminal.yuancode', 0, datetime('now')),
    ('project_index_projects',  'terminal.yuancode', 0, datetime('now')),
    ('project_index_files',     'terminal.yuancode', 0, datetime('now')),
    ('project_index_symbols',   'terminal.yuancode', 0, datetime('now')),
    ('project_index_imports',   'terminal.yuancode', 0, datetime('now')),
    ('project_index_dependencies', 'terminal.yuancode', 0, datetime('now')),
    ('project_index_changes',   'terminal.yuancode', 0, datetime('now'));
