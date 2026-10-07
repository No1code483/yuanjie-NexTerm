//! home.journal 插件服务边界说明。
//!
//! 自有表：journals（归属登记见 migrations/0001_baseline.sql）。
//! 无跨模块读写（`recycle_bin` 的写入只发生在 home.todo 的删除待办路径）。

pub const OWNED_TABLES: &[&str] = &["journals"];
