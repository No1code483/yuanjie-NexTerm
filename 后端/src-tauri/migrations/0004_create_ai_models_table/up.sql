-- Migration v4: create_ai_models_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + 内部调用 migrate_ai_models_table 动态补字段）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_ai_models_table"（T2.6.1 兼容性设计）
-- 说明: AI 模型定义表。本 .sql 仅包含 CREATE TABLE 静态部分；
--       动态字段补丁（api_format/display_name/multimodal/system_prompt/context_window/temperature）
--       由 inline 函数 migrate_ai_models_table 通过 PRAGMA + ALTER TABLE 完成。

CREATE TABLE IF NOT EXISTS ai_models (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    api_url TEXT,
    api_key_enc BLOB,
    api_key_nonce BLOB,
    api_format TEXT,
    model_name TEXT,
    display_name TEXT,
    is_local INTEGER NOT NULL DEFAULT 0,
    multimodal INTEGER NOT NULL DEFAULT 0,
    system_prompt TEXT,
    context_window INTEGER NOT NULL DEFAULT 8192,
    temperature REAL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
