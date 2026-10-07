-- customs.systemtools 基线迁移（批次4c S1）：登记表归属。
-- 表结构由生产迁移创建，本脚本仅登记归属。
-- 短码 `st` + `_` = `st_`，以下表均不以 `st_` 前缀 → name_prefixed=0。

-- 批次4c：system_config 表归属（production migration 0033）
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('system_config', 'customs.systemtools', 0, datetime('now'));

-- 2026-09-26：原批次6a 在此登记的 mek_versions / mek_rotation_log 已撤销。
-- 二表仅由 customs.auth 读写，归属登记以 customs.auth/migrations/0002_mek_baseline.sql 为准。
