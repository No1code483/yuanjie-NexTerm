//! home.todo 插件服务边界说明。
//!
//! 自有表：todos（归属登记见 migrations/0001_baseline.sql）。
//!
//! 跨模块写入（1 处，契约登记为档案 boards.home §十二 C1）：删除待办时由
//! `todo_service::delete_todo` 在单事务内写 `recycle_bin`（属 customs.recycle，
//! 阶段4 迁移）。本批保持现状，零行为变更。

pub const OWNED_TABLES: &[&str] = &["todos"];
