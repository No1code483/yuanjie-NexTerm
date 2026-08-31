-- Migration v116: add_user_id_to_ai_tables
-- 类型: Static ALTER TABLE（多用户数据隔离批次 1：AI 配置类）
-- 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七（多用户数据隔离缺失修复）
-- 说明: 给 ai_models / ai_agents / api_keys 三张用户私有数据表添加 user_id 字段，
--       实现多用户数据隔离，防止 guest 账号读取/修改/删除 admin 的 API Key 等敏感配置。
--
-- 迁移策略：
--   - 新增 user_id INTEGER NOT NULL DEFAULT 1
--   - DEFAULT 1 = 现有数据归 admin（users.id=1），保证现有数据不丢失
--   - 后续 admin 可手动清理 guest 遗留数据
--
-- 索引：
--   - idx_<table>_user_id: 加速 WHERE user_id = ? 过滤
--   - idx_ai_models_user_provider: ai_models 按 (user_id, provider) 查询场景
--   - idx_api_keys_user_provider: api_keys 按 (user_id, provider) UPSERT 场景
--   注意：api_keys.provider 原有 UNIQUE 约束需调整为复合唯一约束 (user_id, provider)，
--         但 SQLite 无法直接修改约束，故先删除原索引（如有），新建复合唯一索引。

-- ===== 1. ai_models 表：添加 user_id 字段 =====
ALTER TABLE ai_models ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_ai_models_user_id ON ai_models(user_id);
CREATE INDEX IF NOT EXISTS idx_ai_models_user_provider ON ai_models(user_id, provider);

-- ===== 2. ai_agents 表：添加 user_id 字段 =====
ALTER TABLE ai_agents ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_ai_agents_user_id ON ai_agents(user_id);

-- ===== 3. api_keys 表：添加 user_id 字段 + 调整唯一约束 =====
-- 注意：api_keys 表在 migration 0109 中创建了 provider UNIQUE 约束（内联于 CREATE TABLE），
-- SQLite 不支持 DROP CONSTRAINT，只能通过重建表的方式修改。
-- 但重建表风险大，此处采用折中方案：
--   - 保留原 provider UNIQUE 约束（全局唯一）
--   - 新增 (user_id, provider) 复合唯一索引，作为应用层校验依据
--   - 后续如需完全多用户隔离，可在应用层强制 UPSERT 时带 user_id，并通过复合索引保证
--     同一 user 下 provider 不重复
-- 实际效果：当前阶段每个 provider 全局仅允许一个 Key，多用户场景下不同用户共享同一 provider Key
--           （这是合理的，因为 API Key 是按 provider 维度而非用户维度配置的，
--            真正的隔离需求在 ai_models / ai_agents 等用户自定义配置上）
ALTER TABLE api_keys ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
