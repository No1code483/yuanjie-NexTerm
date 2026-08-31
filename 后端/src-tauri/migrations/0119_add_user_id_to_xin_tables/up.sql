-- Migration v119: add_user_id_to_xin_tables
-- 类型: Static ALTER TABLE（多用户数据隔离批次 4）
-- 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
-- 说明:
--   给小欣（Xin）模块的 12 张表添加 user_id 字段，实现多用户数据隔离。
--   DEFAULT 1 保证现有 admin 数据自动归属 user_id=1。
--   涉及表：
--   - xin_config: 单行配置表（id=1），需改为按用户隔离
--   - xin_memories: 用户长期记忆（最高敏感度）
--   - xin_summaries: 对话摘要
--   - xin_moods: 情绪状态
--   - xin_reminders: 提醒
--   - xin_habits: 习惯跟踪
--   - xin_conversations: 对话主表
--   - xin_checkpoints: 对话检查点
--   - xin_compaction_records: 压缩记录
--   - xin_compaction_config: 压缩配置单行表
--   - xin_persona_memories: 人格记忆（最高敏感度）
--   - xin_persona_switch_log: 人格切换日志

-- xin_config: 单行配置表，添加 user_id + UNIQUE(user_id) 约束
-- 注意：SQLite ALTER TABLE 不支持添加 UNIQUE 约束，仅添加列 + 索引
ALTER TABLE xin_config ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_xin_config_user_id ON xin_config(user_id);

-- xin_memories: 用户长期记忆
ALTER TABLE xin_memories ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_xin_memories_user_id ON xin_memories(user_id);
CREATE INDEX IF NOT EXISTS idx_xin_memories_user_cat ON xin_memories(user_id, category);

-- xin_summaries: 对话摘要
ALTER TABLE xin_summaries ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_xin_summaries_user_id ON xin_summaries(user_id);
CREATE INDEX IF NOT EXISTS idx_xin_summaries_user_conv ON xin_summaries(user_id, conversation_id);

-- xin_moods: 情绪状态
ALTER TABLE xin_moods ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_xin_moods_user_id ON xin_moods(user_id);
CREATE INDEX IF NOT EXISTS idx_xin_moods_user_cat ON xin_moods(user_id, category);

-- xin_reminders: 提醒
ALTER TABLE xin_reminders ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_xin_reminders_user_id ON xin_reminders(user_id);
CREATE INDEX IF NOT EXISTS idx_xin_reminders_user_active ON xin_reminders(user_id, is_active);

-- xin_habits: 习惯跟踪
ALTER TABLE xin_habits ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_xin_habits_user_id ON xin_habits(user_id);

-- xin_conversations: 对话主表
ALTER TABLE xin_conversations ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_xin_conversations_user_id ON xin_conversations(user_id);
CREATE INDEX IF NOT EXISTS idx_xin_conversations_user_updated ON xin_conversations(user_id, updated_at);

-- xin_checkpoints: 对话检查点
ALTER TABLE xin_checkpoints ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_xin_checkpoints_user_id ON xin_checkpoints(user_id);
CREATE INDEX IF NOT EXISTS idx_xin_checkpoints_user_conv ON xin_checkpoints(user_id, conversation_id);

-- xin_compaction_records: 压缩记录
ALTER TABLE xin_compaction_records ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_xin_compaction_records_user_id ON xin_compaction_records(user_id);
CREATE INDEX IF NOT EXISTS idx_xin_compaction_records_user_conv ON xin_compaction_records(user_id, conversation_id);

-- xin_compaction_config: 压缩配置单行表
ALTER TABLE xin_compaction_config ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_xin_compaction_config_user_id ON xin_compaction_config(user_id);

-- xin_persona_memories: 人格记忆（最高敏感度）
ALTER TABLE xin_persona_memories ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_xin_persona_memories_user_id ON xin_persona_memories(user_id);
CREATE INDEX IF NOT EXISTS idx_xin_persona_memories_user_persona ON xin_persona_memories(user_id, persona_id);

-- xin_persona_switch_log: 人格切换日志
ALTER TABLE xin_persona_switch_log ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_xin_persona_switch_log_user_id ON xin_persona_switch_log(user_id);
CREATE INDEX IF NOT EXISTS idx_xin_persona_switch_log_user_switched ON xin_persona_switch_log(user_id, switched_at DESC);
