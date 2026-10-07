//! home.timer 插件服务边界说明。
//!
//! 自有表：timers（归属登记见 migrations/0001_baseline.sql）。
//! 无跨模块读写（`timers` 的直连读发生在 customs.recycle 的恢复路径，属阶段4 收口项）。

pub const OWNED_TABLES: &[&str] = &["timers"];
