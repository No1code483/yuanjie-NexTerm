-- Migration v51: create_xin_config_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_xin_config_table"（T2.6.1 兼容性设计）
-- 说明: 小欣配置单行表（id=1），存储 config_json 配置 blob

CREATE TABLE IF NOT EXISTS xin_config (
    id INTEGER PRIMARY KEY DEFAULT 1,
    config_json TEXT NOT NULL DEFAULT '{}'
);
