-- Migration v110: create_model_routing_rules_table
-- 类型: Static CREATE TABLE
-- 规范: 功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.4.1（Phase 6 多模型与协作）
--       + 项目核心设计意图 §三 / §八（强制规则 8.1.1：Yuan Code 编程 AI 必须走云端 API）
-- hash 源: 函数名 "create_model_routing_rules_table"（T2.6.1 兼容性设计）
--
-- 说明:
--   Yuan Code v3.2 模型路由配置表 — 按任务类型（编程/分析/审查/文档）路由到不同云端 API 模型。
--   强制约束（is_cloud_only = 1）：
--     编程任务路由规则只能选云端 API provider（openai/anthropic/deepseek/...），
--     禁止选本地底层智能模型（ollama qwen3:8b 等）。
--     后端在 upsert 时通过 CLOUD_API_PROVIDERS 白名单强制校验；
--     在路由解析时再次校验（双层防护）。
--
-- 与 api_keys 表的关系：
--   - api_keys 表：按 provider 维度存储云端 API Key（密文）
--   - model_routing_rules 表：按 task_type 维度存储"使用哪个 provider+model"的路由规则
--   - 路由解析：task_type → rule.provider → api_keys.provider → 取密钥 → 调云端 API

CREATE TABLE IF NOT EXISTS model_routing_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- 任务类型：编程/分析/审查/文档/通用（每类型至多一条规则，UNIQUE 约束）
    task_type TEXT NOT NULL UNIQUE,
    -- 云端 API provider（必须属于 CLOUD_API_PROVIDERS 白名单；禁止 ollama 等本地模型）
    provider TEXT NOT NULL,
    -- 具体模型名（gpt-4o / claude-sonnet-4-20250514 / deepseek-chat / ...）
    model_name TEXT NOT NULL,
    -- 温度（NULL = 由路由层用默认值：编程 0.2 / 分析 0.0 / 文档 0.4）
    temperature REAL,
    -- 最大 token（NULL = 用 provider 默认）
    max_tokens INTEGER,
    -- 是否启用此规则（false 时回退到该 task_type 的默认 provider+model）
    is_enabled INTEGER NOT NULL DEFAULT 1,
    -- 强制云端：1 = 不允许将 provider 改为本地底层智能模型（守卫由后端 service 层强制）
    is_cloud_only INTEGER NOT NULL DEFAULT 1,
    -- 优先级（多条规则匹配时按 priority 降序；当前每 task_type 仅 1 条，预留扩展）
    priority INTEGER NOT NULL DEFAULT 100,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_model_routing_task_type ON model_routing_rules(task_type);
CREATE INDEX IF NOT EXISTS idx_model_routing_enabled ON model_routing_rules(is_enabled);
