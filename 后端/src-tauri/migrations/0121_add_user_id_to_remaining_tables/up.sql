-- Migration v121: add_user_id_to_remaining_tables
-- 类型: 表重建（3 张有 UNIQUE 约束）+ Static ALTER TABLE（11 张无 UNIQUE）（多用户数据隔离批次 6）
-- 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
-- 说明:
--   给剩余 14 张用户私有数据表添加 user_id 字段，实现多用户数据隔离。
--   DEFAULT 1 保证现有 admin 数据自动归属 user_id=1。
--
--   3 张表有 UNIQUE 约束，需重建表以改约束为 (user_id, 原列)：
--     - news_sources: UNIQUE(url) → UNIQUE(user_id, url)
--     - custom_themes: UNIQUE(name) → UNIQUE(user_id, name)
--     - model_routing_rules: UNIQUE(task_type) → UNIQUE(user_id, task_type)
--
--   11 张表无 UNIQUE 约束，直接 ALTER ADD COLUMN：
--     - ssh_profiles, prompt_templates, resumes, quotes
--     - yuan_code_snippets, yuan_code_workspaces, terminal_sessions
--     - recycle_bin, game_worlds, news_cache, mcp_servers

-- ===== 1. 无 UNIQUE 约束的 11 张表：直接 ALTER =====

-- ssh_profiles: SSH 连接配置（高敏感度：含私钥路径）
ALTER TABLE ssh_profiles ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_ssh_profiles_user_id ON ssh_profiles(user_id);

-- prompt_templates: 提示词模板
ALTER TABLE prompt_templates ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_prompt_templates_user_id ON prompt_templates(user_id);

-- resumes: 简历
ALTER TABLE resumes ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_resumes_user_id ON resumes(user_id);

-- quotes: 名言/语录
ALTER TABLE quotes ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_quotes_user_id ON quotes(user_id);

-- yuan_code_snippets: 元·代码片段
ALTER TABLE yuan_code_snippets ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_yuan_code_snippets_user_id ON yuan_code_snippets(user_id);

-- yuan_code_workspaces: 元·代码工作区
ALTER TABLE yuan_code_workspaces ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_yuan_code_workspaces_user_id ON yuan_code_workspaces(user_id);

-- terminal_sessions: 终端会话
ALTER TABLE terminal_sessions ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_terminal_sessions_user_id ON terminal_sessions(user_id);

-- recycle_bin: 回收站
ALTER TABLE recycle_bin ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_recycle_bin_user_id ON recycle_bin(user_id);

-- game_worlds: 游戏 3D 重构世界主表
ALTER TABLE game_worlds ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_game_worlds_user_id ON game_worlds(user_id);

-- news_cache: 新闻缓存
ALTER TABLE news_cache ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_news_cache_user_id ON news_cache(user_id);

-- mcp_servers: MCP 服务器配置
ALTER TABLE mcp_servers ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_mcp_servers_user_id ON mcp_servers(user_id);

-- ===== 2. 有 UNIQUE 约束的 3 张表：重建表 =====

-- news_sources: UNIQUE(url) → UNIQUE(user_id, url)
-- 原表字段：id, name, url, category, feed_type（无 created_at/updated_at）
CREATE TABLE news_sources_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL DEFAULT 1,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'security',
    feed_type TEXT NOT NULL DEFAULT 'rss',
    UNIQUE(user_id, url)
);
INSERT INTO news_sources_new (user_id, name, url, category, feed_type)
SELECT 1, name, url, category, feed_type FROM news_sources;
DROP TABLE news_sources;
ALTER TABLE news_sources_new RENAME TO news_sources;
CREATE INDEX IF NOT EXISTS idx_news_sources_user_id ON news_sources(user_id);

-- custom_themes: UNIQUE(name) → UNIQUE(user_id, name)
-- 原表字段：id, name, base_theme(DEFAULT 'terminal'), variables, created_at, updated_at
CREATE TABLE custom_themes_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL DEFAULT 1,
    name TEXT NOT NULL,
    base_theme TEXT NOT NULL DEFAULT 'terminal',
    variables TEXT NOT NULL DEFAULT '{}',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(user_id, name)
);
INSERT INTO custom_themes_new (user_id, name, base_theme, variables, created_at, updated_at)
SELECT 1, name, base_theme, variables, created_at, updated_at FROM custom_themes;
DROP TABLE custom_themes;
ALTER TABLE custom_themes_new RENAME TO custom_themes;
CREATE INDEX IF NOT EXISTS idx_custom_themes_user_id ON custom_themes(user_id);

-- model_routing_rules: UNIQUE(task_type) → UNIQUE(user_id, task_type)
-- 原表字段：id, task_type, provider, model_name, temperature(nullable), max_tokens(nullable),
--           is_enabled(DEFAULT 1), is_cloud_only(DEFAULT 1), priority(DEFAULT 100), created_at, updated_at
CREATE TABLE model_routing_rules_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL DEFAULT 1,
    task_type TEXT NOT NULL,
    provider TEXT NOT NULL,
    model_name TEXT NOT NULL,
    temperature REAL,
    max_tokens INTEGER,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    is_cloud_only INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 100,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(user_id, task_type)
);
INSERT INTO model_routing_rules_new (user_id, task_type, provider, model_name, temperature, max_tokens, is_enabled, is_cloud_only, priority, created_at, updated_at)
SELECT 1, task_type, provider, model_name, temperature, max_tokens, is_enabled, is_cloud_only, priority, created_at, updated_at FROM model_routing_rules;
DROP TABLE model_routing_rules;
ALTER TABLE model_routing_rules_new RENAME TO model_routing_rules;
CREATE INDEX IF NOT EXISTS idx_model_routing_rules_user_id ON model_routing_rules(user_id);
