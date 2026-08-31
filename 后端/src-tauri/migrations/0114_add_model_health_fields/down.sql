-- Rollback migration v114
ALTER TABLE ai_models DROP COLUMN latency_ms;
ALTER TABLE ai_models DROP COLUMN last_health_check;
ALTER TABLE ai_models DROP COLUMN status;
