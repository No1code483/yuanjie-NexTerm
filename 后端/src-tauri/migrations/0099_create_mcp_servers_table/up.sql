-- D1.6 MCP 协议生态：MCP 服务器配置持久化
-- 存储 MCP 服务器注册信息，应用重启后可自动恢复

CREATE TABLE IF NOT EXISTS mcp_servers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    command TEXT NOT NULL,
    args TEXT NOT NULL DEFAULT '[]',      -- JSON 数组
    env TEXT NOT NULL DEFAULT '{}',        -- JSON 对象
    working_dir TEXT,
    auto_connect INTEGER NOT NULL DEFAULT 0,
    lifecycle_config TEXT NOT NULL DEFAULT '{}',  -- JSON: health_check_interval, max_retries, etc.
    enabled INTEGER NOT NULL DEFAULT 1,
    is_builtin INTEGER NOT NULL DEFAULT 0,  -- 1=预置模板，0=用户自定义
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 预置 3 个常用 MCP server 模板（D1.6 验收标准：接入 ≥ 3 个 MCP server）
INSERT OR IGNORE INTO mcp_servers (id, name, command, args, env, auto_connect, is_builtin) VALUES
('builtin-filesystem', 'filesystem', 'npx', '["-y","@modelcontextprotocol/server-filesystem"]', '{}', 0, 1),
('builtin-git', 'git', 'npx', '["-y","@modelcontextprotocol/server-git"]', '{}', 0, 1),
('builtin-fetch', 'fetch', 'npx', '["-y","@modelcontextprotocol/server-fetch"]', '{}', 0, 1);

CREATE INDEX IF NOT EXISTS idx_mcp_servers_enabled ON mcp_servers(enabled);
CREATE INDEX IF NOT EXISTS idx_mcp_servers_builtin ON mcp_servers(is_builtin);
