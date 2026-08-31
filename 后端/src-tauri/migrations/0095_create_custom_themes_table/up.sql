-- Migration v95: create_custom_themes_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/体验深化/01_主题自定义系统_未来展望.md §2.5（C1.5 后端持久化）
-- hash 源: 函数名 "create_custom_themes_table"（T2.6.1 兼容性设计）
-- 说明: 用户自定义主题持久化表，替代 localStorage 仅本地存储，支持跨设备同步基础
-- 字段：
--   id           - 主键自增
--   name         - 主题名称（用户可读，如"我的暗色"）
--   base_theme   - 基础主题名（terminal/matrix/dracula 等 10 套预设之一）
--   variables    - JSON 字符串，{ "--theme-bg-primary": "#xxx", ... }
--   created_at   - 创建时间（毫秒时间戳）
--   updated_at   - 更新时间（毫秒时间戳）

CREATE TABLE IF NOT EXISTS custom_themes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    base_theme TEXT NOT NULL DEFAULT 'terminal',
    variables TEXT NOT NULL DEFAULT '{}',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 按名称唯一约束（防止重复保存同名主题，UPSERT 时按 name 冲突处理）
CREATE UNIQUE INDEX IF NOT EXISTS idx_custom_themes_name ON custom_themes(name);
