-- xin.wellness 基线迁移（批次4a-1）：只登记既有表归属，不重放历史表 DDL。
-- xin_reminders / xin_habits 由主应用历史迁移创建（含 user_id 列），当前为
-- **零写入预留**（xiaoxin_repo 5 函数无调用方；wellness 命令走内存单例）。
-- 前缀口径：L2 短码 `xw` + `_` = `xw_`，两表均以 `xin_` 开头 → name_prefixed=0（裁定 2）。
INSERT OR IGNORE INTO kernel_table_ownership
    (table_name, plugin_id, name_prefixed, created_at)
VALUES
    ('xin_reminders', 'xin.wellness', 0, datetime('now')),
    ('xin_habits',    'xin.wellness', 0, datetime('now'));