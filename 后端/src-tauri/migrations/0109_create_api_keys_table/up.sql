-- Migration v109: create_api_keys_table
-- 类型: Static CREATE TABLE
-- 规范: 功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.5.1 + 项目核心设计意图 §三（云端 API 强制）
-- hash 源: 函数名 "create_api_keys_table"（T2.6.1 兼容性设计）
-- 说明: Yuan Code v3.1 云端 API 专用密钥存储表。
--       与 ai_models 表的区别：
--         ai_models 表混合存储本地 ollama 模型 + 云端模型，每条记录一个模型；
--         api_keys 表按 provider 维度集中存储云端编程 API Key（一个 provider 一条），
--         专为 Yuan Code 编程 AI 强制走云端 API 设计，禁止本地底层智能模型用于编程生成。
--       加密：api_key_enc + api_key_nonce 通过 crypto::aes_gcm 加密（复用 MEK）。
--       约束：provider 唯一（每 provider 只允许一个 Key），is_cloud_only 强制为 1。

CREATE TABLE IF NOT EXISTS api_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL UNIQUE,
    display_name TEXT,
    api_key_enc BLOB NOT NULL,
    api_key_nonce BLOB NOT NULL,
    api_url TEXT,
    is_cloud_only INTEGER NOT NULL DEFAULT 1,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    last_used_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_api_keys_provider ON api_keys(provider);
CREATE INDEX IF NOT EXISTS idx_api_keys_enabled ON api_keys(is_enabled);
