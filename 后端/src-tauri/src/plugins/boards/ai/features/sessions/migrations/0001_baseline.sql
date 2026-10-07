-- ai.sessions 基线迁移（batchC1）：登记会话表归属到 ai.sessions。
-- 4 张会话表由 boards.ai L1 创建：conversations / conversation_participants /
-- messages / prompt_templates；归属登记从 L1 收编至 L2 ai.sessions（裁定 12）。
-- 前缀口径：短码 `ss` + `_` = `ss_`，四表均不以 `ss_` 开头 → name_prefixed=0
--   （同 ai_models/ai_agents 先例：既有表不改名，档案 §10.3 表归属）。
-- ⚠️ conversations.id 被回收站 / 全局搜索 / 数据蒸馏按 ID 逻辑引用（无物理 FK），不得改值。
-- 裁定 17（2b-2）：prompt_templates 自 v121 起已有 user_id（NOT NULL DEFAULT 1），
--   但 repo 全表 SQL 不过滤、INSERT 不写 → 列存在但隔离未启用，语义仍为全局共享模板。
-- 此处只补登记，供 registry.rs::register_manifest_tables 校验。
-- 表结构本身由 boards.ai migrations 创建，ai.sessions 不重复 DDL。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('conversations',             'ai.sessions', 0, datetime('now')),
    ('conversation_participants', 'ai.sessions', 0, datetime('now')),
    ('messages',                  'ai.sessions', 0, datetime('now')),
    ('prompt_templates',          'ai.sessions', 0, datetime('now'));
