//! customs.search 插件服务边界说明。
//!
//! search 插件不直接拥有表，搜索操作跨模块查询（knowledge_base / conversations / notes /
//! code_files / todos / journals）。无 OWNED_TABLES。

pub const OWNED_TABLES: &[&str] = &[];
