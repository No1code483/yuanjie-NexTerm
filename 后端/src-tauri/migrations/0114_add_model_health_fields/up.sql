-- Migration v114: add model health fields to ai_models table
-- 类型: Static ALTER TABLE（参考已有 migrations/0004_create_ai_models_table 的 PRAGMA + ALTER TABLE 模式）
-- 规范: spec ai-chat-enhancement Phase 1 §1.1
-- 说明: 给 ai_models 表增加 status / last_health_check / latency_ms 三个字段，
--       支持模型状态真实检测与持久化（取代 to_response() 中的硬编码 "active"）。

-- status: online/offline/checking/error/unknown（默认 unknown，首次后台检测后填充真实值）
ALTER TABLE ai_models ADD COLUMN status TEXT NOT NULL DEFAULT 'unknown';
-- last_health_check: unix timestamp ms，NULL 表示从未检测
ALTER TABLE ai_models ADD COLUMN last_health_check INTEGER;
-- latency_ms: 最近一次成功检测的延迟，NULL 表示未检测或失败
ALTER TABLE ai_models ADD COLUMN latency_ms INTEGER;
