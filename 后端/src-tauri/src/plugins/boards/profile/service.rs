//! profile 插件服务边界说明。
//!
//! profile 自有服务只写 user_profiles、resumes、quotes。export_user_data 保留对
//! todos、journals、kb_entries、conversations 的只读聚合，不提供跨模块写入能力。

pub const OWNED_TABLES: &[&str] = &["user_profiles", "resumes", "quotes"];
pub const EXPORT_READ_ONLY_TABLES: &[&str] = &["todos", "journals", "kb_entries", "conversations"];
