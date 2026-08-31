-- Migration v82: create_audit_log_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS + CREATE INDEX IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.2.4（T2.7.1 审计触发器基础设施）
-- hash 源: 函数名 "create_audit_log_table"（T2.6.1 兼容性设计）
-- 说明: T2.7.1 审计日志主表。记录 10 张关键表（users/permissions/ai_models/ai_agents/
--       kb_categories/kb_entries/system_config/conversations/messages/todos）的
--       INSERT/UPDATE/DELETE 操作。old_data/new_data 以 JSON 格式存储关键字段快照。
--       changed_by 默认 'system'，后续 A4 安全加固阶段可通过 session 上下文细化。
-- 索引: 2 个 — (table_name, record_id) 用于按表+记录查询；(changed_at) 用于时间范围查询。

CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,
    record_id INTEGER NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('INSERT', 'UPDATE', 'DELETE')),
    old_data TEXT,
    new_data TEXT,
    changed_by TEXT NOT NULL DEFAULT 'system',
    changed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_audit_log_table_record ON audit_log(table_name, record_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_changed_at ON audit_log(changed_at);
