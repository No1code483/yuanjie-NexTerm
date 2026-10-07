//! xin.wellness（L2）服务边界说明。
//!
//! 批次4a-1：承载 `xin_wellness_commands` 17 条命令（内存单例实现，不落库）。
//! 2 张表 `xin_reminders` / `xin_habits` 为零写入预留（`xiaoxin_repo` 5 函数无调用方），
//! 仅作归属登记。实现层 `xin_wellness_service`（tracker 类型定义）留主应用，dispatcher
//! 复用（2b-2 裁定 15 同口径）。
//! **越权收口（裁定 3）**：原全局单例 `MOOD_TRACKER` / `REMINDER_MANAGER` /
//! `HABIT_TRACKER` / `POMODORO_MANAGER` 改为按 `user_id` 分片（消除多用户状态共享），
//! 收口实现位于 `commands/xin_wellness_commands.rs`（随本批改造）。

pub const OWNED_TABLES: &[&str] = &["xin_reminders", "xin_habits"];