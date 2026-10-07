-- boards.ai 基线迁移（批次2b-2）：只登记既有会话表归属，不重放历史表 DDL。
-- 4 张会话表由主应用迁移创建：conversations（v6）/ conversation_participants（v7）/
-- messages（v8）/ prompt_templates（v69）；各表 v115-v121 陆续增列（starred / unread_count /
-- sort_order / user_id 等）。此处只补登记，供 registry.rs::register_manifest_tables 校验
-- （旧表须先登记后登记 manifest）。
-- 前缀口径：短码 `ai` + `_` = `ai_`，四表均不以 `ai_` 开头 → name_prefixed=0
--   （同 ai_models/ai_agents 先例：既有表不改名，档案 §10.3 表归属）。
-- ⚠️ conversations.id 被回收站 / 全局搜索 / 数据蒸馏按 ID 逻辑引用（无物理 FK），不得改值。
-- 裁定 17（2b-2）：prompt_templates 自 v121 起已有 user_id（NOT NULL DEFAULT 1），
--   但 repo 全表 SQL 不过滤、INSERT 不写 → 列存在但隔离未启用，语义仍为全局共享模板。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('conversations',             'boards.ai', 0, datetime('now')),
    ('conversation_participants', 'boards.ai', 0, datetime('now')),
    ('messages',                  'boards.ai', 0, datetime('now')),
    ('prompt_templates',          'boards.ai', 0, datetime('now'));
