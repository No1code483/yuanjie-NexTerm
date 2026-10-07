//! ai.models 插件服务边界说明。
//!
//! 自有表：ai_models（归属登记见 migrations/0001_baseline.sql）。
//! 暂跨引用（登记批次 2b-2 收口）：`check_all_models_health` 经
//! `chat_service::check_model_health` 复用单模型检测逻辑（chat_service 归 2b-2）。

pub const OWNED_TABLES: &[&str] = &["ai_models"];