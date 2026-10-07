-- ai.sessions v2 迁移（BUG-033 修复）：会话表归属迁移 boards.ai → ai.sessions。
-- v1 基线用 INSERT OR IGNORE：真机库中 4 张会话表已由 L1 boards.ai 基线登记
--（owner=boards.ai），INSERT 因主键冲突静默跳过 → register_table 判
-- 「表 conversations 已归属插件 boards.ai，不能重新登记给 ai.sessions」→ 激活失败。
-- v1 已在受影响库记账（activate_all 先 apply_all 后 register_manifest_tables），
-- 不可重放 → 归属改写必须走新版本 v2。
-- name_prefixed 不改：L1 登记为 0，本插件短码 ss 不命中表名前缀 → 推导仍为 0，口径一致。
UPDATE kernel_table_ownership
SET plugin_id = 'ai.sessions'
WHERE table_name IN ('conversations', 'conversation_participants', 'messages', 'prompt_templates')
  AND plugin_id = 'boards.ai';
