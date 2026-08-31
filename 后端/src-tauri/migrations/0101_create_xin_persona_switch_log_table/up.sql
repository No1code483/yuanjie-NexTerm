-- Migration v101: create_xin_persona_switch_log_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/模块深化/03_小欣_多模态融合_深度.md §2.7（D3.8 人格系统补全）
-- hash 源: 函数名 "create_xin_persona_switch_log_table"
-- 说明: D3.8 人格切换历史表 — 记录每次人格切换，用于
--       1) 前端人格切换时间线展示
--       2) 自生长人格的学习数据源（分析用户切换习惯）

CREATE TABLE IF NOT EXISTS xin_persona_switch_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_persona_id TEXT,
    to_persona_id TEXT NOT NULL,
    switched_at TEXT NOT NULL,
    trigger TEXT NOT NULL DEFAULT 'manual'
);

CREATE INDEX IF NOT EXISTS idx_xin_persona_switch_log_switched ON xin_persona_switch_log(switched_at DESC);
CREATE INDEX IF NOT EXISTS idx_xin_persona_switch_log_to_persona ON xin_persona_switch_log(to_persona_id);
