//! customs.systemtools 插件服务边界说明。
//!
//! 自有表：system_config（批次4c S1 登记）。
//!
//! 跨模块访问：
//!  - system_config 的部分 key 由 customs.auth 2FA 实现读写（`2fa_secret_*` / `2fa_enabled_*`），
//!    该跨模块受限写保留原实现以维持行为等价，登记为待收编项（与 customs.auth service.rs 同表注解）。
//!
pub const OWNED_TABLES: &[&str] = &["system_config"];

/// 跨模块受限键前缀（供 4c S1 边界记录；后续 auth 插件仍读写 system_config 2fa 键）。
pub const CROSS_MODULE_KEY_PREFIXED_TABLES: &[(&str, &str)] = &[("system_config", "2fa_")];
