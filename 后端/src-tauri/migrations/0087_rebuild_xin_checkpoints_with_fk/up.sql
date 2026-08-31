-- Migration v87: rebuild_xin_checkpoints_with_fk
-- 类型: 动态（表重建 - SQLite 不支持 ALTER TABLE ADD FOREIGN KEY）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.2.2（T2.7.4 P3 AI 表 FK 补建）
-- hash 源: 函数名 "rebuild_xin_checkpoints_with_fk"（T2.6.1 兼容性设计）
-- 说明: 重建 xin_checkpoints 表，添加 1 条外键约束
--       - conversation_id → xin_conversations(id) ON DELETE CASCADE
-- 表重建模式: CREATE _new → INSERT → DROP old → RENAME → 重建索引
-- 数据清理: 删除引用了不存在 xin_conversations.id 的孤儿记录（防御性）

-- 1. 清理孤儿数据（防御性，确保 INSERT 不会因 FK 检查失败）
DELETE FROM xin_checkpoints WHERE conversation_id NOT IN (SELECT id FROM xin_conversations);

-- 2. 创建带 FK 的新表
CREATE TABLE xin_checkpoints_new (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    checkpoint_type TEXT NOT NULL DEFAULT 'auto',
    context_json TEXT NOT NULL,
    partial_response TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    title TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES xin_conversations(id) ON DELETE CASCADE
);

-- 3. 迁移数据
INSERT INTO xin_checkpoints_new (id, conversation_id, checkpoint_type, context_json, partial_response, message_count, total_tokens, title, created_at)
SELECT id, conversation_id, checkpoint_type, context_json, partial_response, message_count, total_tokens, title, created_at FROM xin_checkpoints;

-- 4. 删除旧表
DROP TABLE xin_checkpoints;

-- 5. 重命名新表
ALTER TABLE xin_checkpoints_new RENAME TO xin_checkpoints;

-- 6. 重建索引（原 v66 create_indexes + xin_checkpoint_service 中定义的 1 个索引）
CREATE INDEX IF NOT EXISTS idx_xin_checkpoints_conv ON xin_checkpoints(conversation_id, created_at DESC);
