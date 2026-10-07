//! ai.agent 插件服务边界说明。
//!
//! 自有表：ai_agents（归属登记见 migrations/0001_baseline.sql）。
//! 无跨模块读写（agent 的 model_id 仅经 `ai_repo::get_model_by_id` 校验存在性，
//! 属同域 ai.models 表，登记批次 2b-2 收口跨 L2 引用形态）。

pub const OWNED_TABLES: &[&str] = &["ai_agents"];