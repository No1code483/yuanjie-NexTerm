-- Migration v117: add_user_id_to_conversations_tables
-- 类型: Static ALTER TABLE（多用户数据隔离批次 2）
-- 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
-- 说明:
--   给 conversations / messages / conversation_participants 三张表添加 user_id 字段，
--   实现多用户数据隔离。DEFAULT 1 保证现有 admin 数据自动归属 user_id=1。
--   - conversations: 主表，user_id 直接过滤
--   - messages: 通过 conversation_id 关联，但加冗余 user_id 简化查询 + 防御深度
--   - conversation_participants: 参与者关联也是用户私有数据

ALTER TABLE conversations ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_conversations_user_id ON conversations(user_id);
CREATE INDEX IF NOT EXISTS idx_conversations_user_updated ON conversations(user_id, updated_at);

ALTER TABLE messages ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_messages_user_id ON messages(user_id);
CREATE INDEX IF NOT EXISTS idx_messages_user_conv ON messages(user_id, conversation_id);

ALTER TABLE conversation_participants ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_conversation_participants_user_id ON conversation_participants(user_id);
