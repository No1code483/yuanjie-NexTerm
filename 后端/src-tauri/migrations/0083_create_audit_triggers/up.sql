-- Migration v83: create_audit_triggers
-- 类型: 静态（pure CREATE TRIGGER IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.2.4（T2.7.1 审计触发器）
-- hash 源: 函数名 "create_audit_triggers"（T2.6.1 兼容性设计）
-- 说明: 为 10 张关键表创建审计触发器（每表 3 个 = INSERT/UPDATE/DELETE，共 30 个触发器）。
--       触发器将操作记录写入 audit_log 表，old_data/new_data 以 JSON 格式存储关键字段。
--       设计原则：只记录业务关键字段（非全部字段），避免日志过大；
--                 messages.content 截断到 500 字符；changed_by 默认 'system'。
-- 涉及表: users / permissions / ai_models / ai_agents / kb_categories / kb_entries /
--         system_config / conversations / messages / todos

-- =============================================================================
-- 1. users 表审计触发器（关键字段: username, role, is_permanent）
-- =============================================================================

CREATE TRIGGER IF NOT EXISTS audit_users_insert
AFTER INSERT ON users
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, new_data, changed_by)
    VALUES ('users', NEW.id, 'INSERT',
            json_object('username', NEW.username, 'role', NEW.role, 'is_permanent', NEW.is_permanent),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_users_update
AFTER UPDATE ON users
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, new_data, changed_by)
    VALUES ('users', NEW.id, 'UPDATE',
            json_object('username', OLD.username, 'role', OLD.role, 'is_permanent', OLD.is_permanent),
            json_object('username', NEW.username, 'role', NEW.role, 'is_permanent', NEW.is_permanent),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_users_delete
AFTER DELETE ON users
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, changed_by)
    VALUES ('users', OLD.id, 'DELETE',
            json_object('username', OLD.username, 'role', OLD.role, 'is_permanent', OLD.is_permanent),
            'system');
END;

-- =============================================================================
-- 2. permissions 表审计触发器（关键字段: role, resource, can_read/write/delete/modify）
-- =============================================================================

CREATE TRIGGER IF NOT EXISTS audit_permissions_insert
AFTER INSERT ON permissions
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, new_data, changed_by)
    VALUES ('permissions', NEW.id, 'INSERT',
            json_object('role', NEW.role, 'resource', NEW.resource,
                        'can_read', NEW.can_read, 'can_write', NEW.can_write,
                        'can_delete', NEW.can_delete, 'can_modify', NEW.can_modify),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_permissions_update
AFTER UPDATE ON permissions
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, new_data, changed_by)
    VALUES ('permissions', NEW.id, 'UPDATE',
            json_object('role', OLD.role, 'resource', OLD.resource,
                        'can_read', OLD.can_read, 'can_write', OLD.can_write,
                        'can_delete', OLD.can_delete, 'can_modify', OLD.can_modify),
            json_object('role', NEW.role, 'resource', NEW.resource,
                        'can_read', NEW.can_read, 'can_write', NEW.can_write,
                        'can_delete', NEW.can_delete, 'can_modify', NEW.can_modify),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_permissions_delete
AFTER DELETE ON permissions
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, changed_by)
    VALUES ('permissions', OLD.id, 'DELETE',
            json_object('role', OLD.role, 'resource', OLD.resource,
                        'can_read', OLD.can_read, 'can_write', OLD.can_write,
                        'can_delete', OLD.can_delete, 'can_modify', OLD.can_modify),
            'system');
END;

-- =============================================================================
-- 3. ai_models 表审计触发器（关键字段: name, provider, is_local, multimodal）
-- =============================================================================

CREATE TRIGGER IF NOT EXISTS audit_ai_models_insert
AFTER INSERT ON ai_models
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, new_data, changed_by)
    VALUES ('ai_models', NEW.id, 'INSERT',
            json_object('name', NEW.name, 'provider', NEW.provider,
                        'is_local', NEW.is_local, 'multimodal', NEW.multimodal),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_ai_models_update
AFTER UPDATE ON ai_models
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, new_data, changed_by)
    VALUES ('ai_models', NEW.id, 'UPDATE',
            json_object('name', OLD.name, 'provider', OLD.provider,
                        'is_local', OLD.is_local, 'multimodal', OLD.multimodal),
            json_object('name', NEW.name, 'provider', NEW.provider,
                        'is_local', NEW.is_local, 'multimodal', NEW.multimodal),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_ai_models_delete
AFTER DELETE ON ai_models
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, changed_by)
    VALUES ('ai_models', OLD.id, 'DELETE',
            json_object('name', OLD.name, 'provider', OLD.provider,
                        'is_local', OLD.is_local, 'multimodal', OLD.multimodal),
            'system');
END;

-- =============================================================================
-- 4. ai_agents 表审计触发器（关键字段: name, model_id）
-- =============================================================================

CREATE TRIGGER IF NOT EXISTS audit_ai_agents_insert
AFTER INSERT ON ai_agents
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, new_data, changed_by)
    VALUES ('ai_agents', NEW.id, 'INSERT',
            json_object('name', NEW.name, 'model_id', NEW.model_id),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_ai_agents_update
AFTER UPDATE ON ai_agents
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, new_data, changed_by)
    VALUES ('ai_agents', NEW.id, 'UPDATE',
            json_object('name', OLD.name, 'model_id', OLD.model_id),
            json_object('name', NEW.name, 'model_id', NEW.model_id),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_ai_agents_delete
AFTER DELETE ON ai_agents
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, changed_by)
    VALUES ('ai_agents', OLD.id, 'DELETE',
            json_object('name', OLD.name, 'model_id', OLD.model_id),
            'system');
END;

-- =============================================================================
-- 5. kb_categories 表审计触发器（关键字段: name, parent_id, library）
-- =============================================================================

CREATE TRIGGER IF NOT EXISTS audit_kb_categories_insert
AFTER INSERT ON kb_categories
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, new_data, changed_by)
    VALUES ('kb_categories', NEW.id, 'INSERT',
            json_object('name', NEW.name, 'parent_id', NEW.parent_id, 'library', NEW.library),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_kb_categories_update
AFTER UPDATE ON kb_categories
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, new_data, changed_by)
    VALUES ('kb_categories', NEW.id, 'UPDATE',
            json_object('name', OLD.name, 'parent_id', OLD.parent_id, 'library', OLD.library),
            json_object('name', NEW.name, 'parent_id', NEW.parent_id, 'library', NEW.library),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_kb_categories_delete
AFTER DELETE ON kb_categories
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, changed_by)
    VALUES ('kb_categories', OLD.id, 'DELETE',
            json_object('name', OLD.name, 'parent_id', OLD.parent_id, 'library', OLD.library),
            'system');
END;

-- =============================================================================
-- 6. kb_entries 表审计触发器（关键字段: name, category_id, entry_type）
-- =============================================================================

CREATE TRIGGER IF NOT EXISTS audit_kb_entries_insert
AFTER INSERT ON kb_entries
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, new_data, changed_by)
    VALUES ('kb_entries', NEW.id, 'INSERT',
            json_object('name', NEW.name, 'category_id', NEW.category_id, 'entry_type', NEW.entry_type),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_kb_entries_update
AFTER UPDATE ON kb_entries
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, new_data, changed_by)
    VALUES ('kb_entries', NEW.id, 'UPDATE',
            json_object('name', OLD.name, 'category_id', OLD.category_id, 'entry_type', OLD.entry_type),
            json_object('name', NEW.name, 'category_id', NEW.category_id, 'entry_type', NEW.entry_type),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_kb_entries_delete
AFTER DELETE ON kb_entries
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, changed_by)
    VALUES ('kb_entries', OLD.id, 'DELETE',
            json_object('name', OLD.name, 'category_id', OLD.category_id, 'entry_type', OLD.entry_type),
            'system');
END;

-- =============================================================================
-- 7. system_config 表审计触发器（关键字段: config_key, config_value）
-- =============================================================================

CREATE TRIGGER IF NOT EXISTS audit_system_config_insert
AFTER INSERT ON system_config
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, new_data, changed_by)
    VALUES ('system_config', NEW.id, 'INSERT',
            json_object('config_key', NEW.config_key, 'config_value', NEW.config_value),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_system_config_update
AFTER UPDATE ON system_config
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, new_data, changed_by)
    VALUES ('system_config', NEW.id, 'UPDATE',
            json_object('config_key', OLD.config_key, 'config_value', OLD.config_value),
            json_object('config_key', NEW.config_key, 'config_value', NEW.config_value),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_system_config_delete
AFTER DELETE ON system_config
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, changed_by)
    VALUES ('system_config', OLD.id, 'DELETE',
            json_object('config_key', OLD.config_key, 'config_value', OLD.config_value),
            'system');
END;

-- =============================================================================
-- 8. conversations 表审计触发器（关键字段: title, type, is_temp）
-- =============================================================================

CREATE TRIGGER IF NOT EXISTS audit_conversations_insert
AFTER INSERT ON conversations
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, new_data, changed_by)
    VALUES ('conversations', NEW.id, 'INSERT',
            json_object('title', NEW.title, 'type', NEW.type, 'is_temp', NEW.is_temp),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_conversations_update
AFTER UPDATE ON conversations
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, new_data, changed_by)
    VALUES ('conversations', NEW.id, 'UPDATE',
            json_object('title', OLD.title, 'type', OLD.type, 'is_temp', OLD.is_temp),
            json_object('title', NEW.title, 'type', NEW.type, 'is_temp', NEW.is_temp),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_conversations_delete
AFTER DELETE ON conversations
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, changed_by)
    VALUES ('conversations', OLD.id, 'DELETE',
            json_object('title', OLD.title, 'type', OLD.type, 'is_temp', OLD.is_temp),
            'system');
END;

-- =============================================================================
-- 9. messages 表审计触发器（关键字段: conversation_id, sender_type, content 截断 500 字符）
-- =============================================================================

CREATE TRIGGER IF NOT EXISTS audit_messages_insert
AFTER INSERT ON messages
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, new_data, changed_by)
    VALUES ('messages', NEW.id, 'INSERT',
            json_object('conversation_id', NEW.conversation_id,
                        'sender_type', NEW.sender_type,
                        'content', substr(NEW.content, 1, 500)),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_messages_update
AFTER UPDATE ON messages
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, new_data, changed_by)
    VALUES ('messages', NEW.id, 'UPDATE',
            json_object('conversation_id', OLD.conversation_id,
                        'sender_type', OLD.sender_type,
                        'content', substr(OLD.content, 1, 500)),
            json_object('conversation_id', NEW.conversation_id,
                        'sender_type', NEW.sender_type,
                        'content', substr(NEW.content, 1, 500)),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_messages_delete
AFTER DELETE ON messages
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, changed_by)
    VALUES ('messages', OLD.id, 'DELETE',
            json_object('conversation_id', OLD.conversation_id,
                        'sender_type', OLD.sender_type,
                        'content', substr(OLD.content, 1, 500)),
            'system');
END;

-- =============================================================================
-- 10. todos 表审计触发器（关键字段: title, completed, priority）
-- =============================================================================

CREATE TRIGGER IF NOT EXISTS audit_todos_insert
AFTER INSERT ON todos
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, new_data, changed_by)
    VALUES ('todos', NEW.id, 'INSERT',
            json_object('title', NEW.title, 'completed', NEW.completed, 'priority', NEW.priority),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_todos_update
AFTER UPDATE ON todos
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, new_data, changed_by)
    VALUES ('todos', NEW.id, 'UPDATE',
            json_object('title', OLD.title, 'completed', OLD.completed, 'priority', OLD.priority),
            json_object('title', NEW.title, 'completed', NEW.completed, 'priority', NEW.priority),
            'system');
END;

CREATE TRIGGER IF NOT EXISTS audit_todos_delete
AFTER DELETE ON todos
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_data, changed_by)
    VALUES ('todos', OLD.id, 'DELETE',
            json_object('title', OLD.title, 'completed', OLD.completed, 'priority', OLD.priority),
            'system');
END;
