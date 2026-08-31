-- Migration v42: create_terminal_config_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_terminal_config_table"（T2.6.1 兼容性设计）
-- 说明: 终端配置单行表（id='default'），存储字体、行高、光标样式、主题等

CREATE TABLE IF NOT EXISTS terminal_config (
    id TEXT PRIMARY KEY DEFAULT 'default',
    font_family TEXT NOT NULL DEFAULT 'Cascadia Code, Consolas, monospace',
    font_size INTEGER NOT NULL DEFAULT 14,
    line_height REAL NOT NULL DEFAULT 1.2,
    cursor_style TEXT NOT NULL DEFAULT 'block',
    cursor_blink INTEGER NOT NULL DEFAULT 1,
    theme_name TEXT NOT NULL DEFAULT 'Dark+',
    updated_at INTEGER NOT NULL DEFAULT 0
);
