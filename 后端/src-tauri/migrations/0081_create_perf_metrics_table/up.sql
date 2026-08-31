-- Migration v81: create_perf_metrics_table (静态部分)
-- 类型: Mixed（CREATE TABLE 静态 + inline CREATE INDEX）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
--       + 功能展望/平台级增强/01_性能优化_首屏200ms计划.md §2.1.3
-- hash 源: 函数名 "create_perf_metrics_table"（T2.6.1 兼容性设计）
-- 说明: v1.52 性能基线 - 性能指标采集表（启动耗时/FCP/LCP/TTI/路由切换/IPC 延迟）。
--       本 .sql 仅包含 CREATE TABLE 静态部分；
--       idx_perf_metrics_name_time 索引由 inline 语句创建。

CREATE TABLE IF NOT EXISTS perf_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    metric_name TEXT NOT NULL,
    metric_value_ms INTEGER NOT NULL,
    route TEXT,
    command_name TEXT,
    recorded_at TEXT NOT NULL,
    metadata TEXT
);
