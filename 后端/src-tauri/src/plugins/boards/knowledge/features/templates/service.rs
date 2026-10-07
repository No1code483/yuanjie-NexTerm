//! knowledge.templates 插件服务边界说明。
//!
//! 自有表：kb_templates（自 L1 迁入，features/templates/migrations/0001_baseline.sql
//! 执行归属 UPDATE）。实现层 `kb_commands` 模版函数留主应用，dispatcher 复用
//!（2b-2 裁定 15 同口径，实现层迁移登记阶段4）。

pub const OWNED_TABLES: &[&str] = &["kb_templates"];
