-- Migration v108: supplement_missing_indexes (Phase 3 §2.2.2)
-- 类型: 静态（pure CREATE INDEX IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/01_性能优化_首屏200ms计划.md §Phase 3
-- hash 源: 函数名 "supplement_missing_indexes"
-- 说明: 健康检查（health_check_service::check_indexes）发现 8 个期望索引缺失，本迁移补建。
--
-- 补建清单（按健康检查 CRITICAL_INDEXES 期望名）：
--   ✅ idx_messages_conversation_id   — messages(conversation_id) 高频 FK 查询
--   ✅ idx_kb_entries_category_id      — kb_entries(category_id) 分类筛选
--   ✅ idx_kb_categories_parent_id     — kb_categories(parent_id) 自引用 FK
--   ✅ idx_audit_log_table_row         — audit_log(table_name, record_id) 按表+记录查询
--   ✅ idx_audit_log_created_at        — audit_log(changed_at) 时间范围查询
--   ✅ idx_backup_records_type_created — backup_records(backup_type, created_at DESC)
--   ⚠️  idx_users_email                — 跳过：users 表无 email 列（username 是唯一标识）
--   ⚠️  idx_conversations_user_id      — 跳过：conversations 表无 user_id 列（多 AI 群聊设计）
--
-- 关于 audit_log 索引：v82 已创建 idx_audit_log_table_record / idx_audit_log_changed_at
-- （列相同但名称不同），此处按健康检查期望名补建，等效索引重复存在轻微写入开销，
-- 但保证 check_indexes() 通过。后续可在 CRITICAL_INDEXES 中统一名称后移除重复。
--
-- 关于 backup_records 索引：backup_records 表位于独立 backup_records.db（由 backup_service
-- 初始化），不在主库。此处 CREATE INDEX IF NOT EXISTS 在主库执行时，若表不存在会失败。
-- 为保证迁移幂等，使用防御性创建：先确保表存在（与 backup_service DDL 一致），再建索引。
-- 若 backup_service 已初始化该表，CREATE TABLE IF NOT EXISTS 为空操作；索引同理。

-- 防御性创建 backup_records 表（仅当主库中不存在时；正常情况表在 backup_records.db）
CREATE TABLE IF NOT EXISTS backup_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    backup_path TEXT NOT NULL UNIQUE,
    backup_size_bytes INTEGER NOT NULL,
    backup_type TEXT NOT NULL CHECK (backup_type IN ('hourly', 'daily', 'pre_op', 'emergency')),
    label TEXT,
    schema_version INTEGER NOT NULL,
    integrity_check TEXT NOT NULL,
    checksum TEXT NOT NULL,
    table_counts_json TEXT,
    created_at INTEGER NOT NULL,
    verified INTEGER NOT NULL DEFAULT 1
);

-- 1. messages.conversation_id — 加载会话消息（最高频 FK 查询）
CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages(conversation_id);

-- 2. kb_entries.category_id — 按分类筛选知识库条目
CREATE INDEX IF NOT EXISTS idx_kb_entries_category_id ON kb_entries(category_id);

-- 3. kb_categories.parent_id — 自引用 FK，递归查询子分类
CREATE INDEX IF NOT EXISTS idx_kb_categories_parent_id ON kb_categories(parent_id);

-- 4. audit_log(table_name, record_id) — 按表+记录查询审计历史
--    （v82 已有 idx_audit_log_table_record 同列索引；此处按健康检查期望名补建）
CREATE INDEX IF NOT EXISTS idx_audit_log_table_row ON audit_log(table_name, record_id);

-- 5. audit_log(changed_at) — 按时间范围查询审计日志
--    （v82 已有 idx_audit_log_changed_at 同列索引；此处按健康检查期望名补建）
CREATE INDEX IF NOT EXISTS idx_audit_log_created_at ON audit_log(changed_at);

-- 6. backup_records(backup_type, created_at DESC) — 按类型+时间查询备份记录
CREATE INDEX IF NOT EXISTS idx_backup_records_type_created
    ON backup_records(backup_type, created_at DESC);
