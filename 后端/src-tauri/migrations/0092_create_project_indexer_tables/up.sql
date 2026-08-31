-- Migration v92: create_project_indexer_tables
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §2.2.2
-- 说明: D1.4 项目索引系统 — 为 Yuan Code v3 提供项目级上下文
--       支持：文件元数据、符号索引、导入关系、依赖图、变更历史
--       为代码补全/分析/重构提供项目级理解能力

-- 项目表：每个被索引的项目根目录
CREATE TABLE IF NOT EXISTS project_index_projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    root_path TEXT NOT NULL UNIQUE,              -- 项目根目录绝对路径
    name TEXT,                                    -- 项目名（默认取 root_path 最后一段）
    language_major TEXT,                          -- 主语言（如 'rust'/'typescript'）
    indexed_at INTEGER,                           -- 上次全量索引时间戳（秒）
    file_count INTEGER NOT NULL DEFAULT 0,        -- 已索引文件数
    symbol_count INTEGER NOT NULL DEFAULT 0,      -- 已索引符号数
    status TEXT NOT NULL DEFAULT 'idle',          -- 'idle' | 'indexing' | 'error'
    last_error TEXT
);

-- 文件元数据表
CREATE TABLE IF NOT EXISTS project_index_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    path TEXT NOT NULL,                           -- 相对于 root_path 的路径
    absolute_path TEXT NOT NULL,                  -- 绝对路径
    language TEXT,                                -- 检测到的语言
    size_bytes INTEGER NOT NULL DEFAULT 0,
    last_modified INTEGER,                        -- 文件 mtime（Unix 秒）
    content_hash TEXT,                            -- 内容 SHA-256（用于增量索引）
    line_count INTEGER NOT NULL DEFAULT 0,
    indexed_at INTEGER NOT NULL,                  -- 本次索引时间戳
    FOREIGN KEY (project_id) REFERENCES project_index_projects(id) ON DELETE CASCADE,
    UNIQUE(project_id, path)
);

CREATE INDEX IF NOT EXISTS idx_project_files_project ON project_index_files(project_id);
CREATE INDEX IF NOT EXISTS idx_project_files_language ON project_index_files(language);
CREATE INDEX IF NOT EXISTS idx_project_files_hash ON project_index_files(content_hash);

-- 符号表：函数 / 类 / 类型 / 变量等
CREATE TABLE IF NOT EXISTS project_index_symbols (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL,
    name TEXT NOT NULL,                           -- 符号名
    kind TEXT NOT NULL,                           -- 'function' | 'class' | 'type' | 'variable' | 'module' | 'interface' | 'enum' | 'constant'
    line_start INTEGER NOT NULL,
    line_end INTEGER NOT NULL,
    column_start INTEGER NOT NULL DEFAULT 0,
    column_end INTEGER NOT NULL DEFAULT 0,
    signature TEXT,                               -- 函数签名 / 类型声明
    documentation TEXT,                           -- 文档注释
    is_exported INTEGER NOT NULL DEFAULT 0,       -- 是否导出（1=是, 0=否）
    FOREIGN KEY (file_id) REFERENCES project_index_files(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_project_symbols_file ON project_index_symbols(file_id);
CREATE INDEX IF NOT EXISTS idx_project_symbols_name ON project_index_symbols(name);
CREATE INDEX IF NOT EXISTS idx_project_symbols_kind ON project_index_symbols(kind);
CREATE INDEX IF NOT EXISTS idx_project_symbols_exported ON project_index_symbols(is_exported);

-- 导入关系表：当前文件导入了哪些路径/符号
CREATE TABLE IF NOT EXISTS project_index_imports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL,                     -- 导入所在的文件
    imported_path TEXT NOT NULL,                  -- 被导入的路径（可能是相对/绝对/模块名）
    imported_symbol TEXT,                         -- 被导入的具体符号（NULL 表示整模块导入）
    line_number INTEGER NOT NULL,                 -- import 语句所在行
    import_kind TEXT NOT NULL DEFAULT 'import',   -- 'import' | 'export' | 'require' | 'use' | 'include'
    FOREIGN KEY (file_id) REFERENCES project_index_files(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_project_imports_file ON project_index_imports(file_id);
CREATE INDEX IF NOT EXISTS idx_project_imports_symbol ON project_index_imports(imported_symbol);
CREATE INDEX IF NOT EXISTS idx_project_imports_path ON project_index_imports(imported_path);

-- 依赖图：文件间依赖关系（已解析的具体依赖）
CREATE TABLE IF NOT EXISTS project_index_dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    from_file_id INTEGER NOT NULL,                -- 依赖发起方
    to_file_id INTEGER,                           -- 依赖目标（NULL 表示外部依赖未匹配到文件）
    from_path TEXT NOT NULL,                      -- 冗余存储：发起方相对路径
    to_path TEXT NOT NULL,                        -- 冗余存储：目标相对路径或外部模块名
    dependency_type TEXT NOT NULL,                -- 'import' | 'call' | 'extend' | 'implement'
    strength REAL NOT NULL DEFAULT 1.0,           -- 0.0~1.0 依赖强度
    FOREIGN KEY (project_id) REFERENCES project_index_projects(id) ON DELETE CASCADE,
    FOREIGN KEY (from_file_id) REFERENCES project_index_files(id) ON DELETE CASCADE,
    UNIQUE(project_id, from_file_id, to_path, dependency_type)
);

CREATE INDEX IF NOT EXISTS idx_project_deps_from ON project_index_dependencies(from_file_id);
CREATE INDEX IF NOT EXISTS idx_project_deps_to ON project_index_dependencies(to_file_id);
CREATE INDEX IF NOT EXISTS idx_project_deps_project ON project_index_dependencies(project_id);

-- 变更历史表：跟踪文件变更（用于增量索引 + 智能优化）
CREATE TABLE IF NOT EXISTS project_index_changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    file_path TEXT NOT NULL,                      -- 相对路径
    change_type TEXT NOT NULL,                    -- 'created' | 'modified' | 'deleted'
    changed_at INTEGER NOT NULL,                  -- 变更时间戳
    diff_summary TEXT,                            -- 简要 diff 摘要
    commit_hash TEXT,                             -- 关联的 git commit（可空）
    FOREIGN KEY (project_id) REFERENCES project_index_projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_project_changes_project ON project_index_changes(project_id, changed_at DESC);
CREATE INDEX IF NOT EXISTS idx_project_changes_file ON project_index_changes(file_path);
