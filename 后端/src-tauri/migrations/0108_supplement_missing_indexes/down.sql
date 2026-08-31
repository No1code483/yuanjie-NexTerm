-- Migration v108 DOWN: 回滚补建索引（仅删除 v108 新增，不动原有索引）
-- 注：idx_audit_log_table_record / idx_audit_log_changed_at（v82 创建）不受影响

DROP INDEX IF EXISTS idx_messages_conversation_id;
DROP INDEX IF EXISTS idx_kb_entries_category_id;
DROP INDEX IF EXISTS idx_kb_categories_parent_id;
DROP INDEX IF EXISTS idx_audit_log_table_row;
DROP INDEX IF EXISTS idx_audit_log_created_at;
DROP INDEX IF EXISTS idx_backup_records_type_created;
