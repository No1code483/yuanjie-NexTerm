-- Migration v7: create_conversation_participants_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_conversation_participants_table"（T2.6.1 兼容性设计）
-- 说明: 对话参与者关联表，三元唯一约束 + 三外键

CREATE TABLE IF NOT EXISTS conversation_participants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL,
    model_id INTEGER,
    agent_id INTEGER,
    role TEXT NOT NULL,
    UNIQUE(conversation_id, model_id, agent_id),
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (model_id) REFERENCES ai_models(id) ON DELETE SET NULL,
    FOREIGN KEY (agent_id) REFERENCES ai_agents(id) ON DELETE SET NULL
);
