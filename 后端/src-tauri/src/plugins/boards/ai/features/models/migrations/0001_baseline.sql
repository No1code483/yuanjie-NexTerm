-- ai.models 基线迁移（批次2b-1）：只登记既有表归属，不重放历史表 DDL。
-- ai_models 由主应用迁移 v4 创建、v114 增健康字段（status/last_health_check/latency_ms）、
-- v116 增 user_id；此处只补登记，供 registry.rs::register_manifest_tables 校验
-- （旧表须先登记后登记 manifest）。
-- 前缀口径：短码 `am` ≠ 旧名前缀 `ai_` → name_prefixed=0（既有表不改名，档案 §八 表归属）。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('ai_models', 'ai.models', 0, datetime('now'));