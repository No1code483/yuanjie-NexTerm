-- Migration v5: create_ai_agents_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_ai_agents_table"（T2.6.1 兼容性设计）
-- 说明: AI 智能体定义表，FK 引用 ai_models(id)

CREATE TABLE IF NOT EXISTS ai_agents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    system_prompt TEXT,
    model_id INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (model_id) REFERENCES ai_models(id) ON DELETE SET NULL
);
