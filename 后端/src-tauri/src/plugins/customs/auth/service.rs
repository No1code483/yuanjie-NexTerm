//! customs.auth 插件服务边界说明。
//!
//! 自有表：users、permissions、auth_sessions（批次1a-1 登记）；
//! mek_versions、mek_rotation_log（批次1a-2a crypto 收编时登记）。
//!
//! 跨模块受限访问：system_config 的 `2fa_secret_*` / `2fa_enabled_*` 键位由既有
//! 2FA 实现读写。本批不改该设计（保留原实现以维持行为等价），登记为待收编项。
//!
//! 加密设施边界（批次1a-2a）：MEK 会话缓存与版本/轮换逻辑收归本插件私有
//! （`crypto/`），内核仅保留门面（`crate::crypto::mek_manager::MekManager`）与
//! 加密原语，接口契约为 `kernel_api::MekProvider`。

pub const OWNED_TABLES: &[&str] = &[
    "users",
    "permissions",
    "auth_sessions",
    "mek_versions",
    "mek_rotation_log",
];
pub const CROSS_MODULE_KEY_PREFIXED_TABLES: &[(&str, &str)] = &[("system_config", "2fa_")];
