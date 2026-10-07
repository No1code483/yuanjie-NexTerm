//! customs.auth 私有加密设施（阶段3 批次1a-2a：crypto/MEK 收编）。
//!
//! 契约依据：`02_核心机制设计/02_后端插件架构设计.md` §七
//! 「`crypto/`（用户密钥）→ auth 插件私有 + kernel 仅留加密基础设施接口」。
//!
//! 模块边界：
//! - `mek_manager`：MEK 会话缓存（`AuthMekProvider`）与版本/轮换/历史 DB 逻辑；
//! - `mek_rotation_commands`：3 条 `mek_rotation_*` 命令实现（dispatcher 与旧 transport 共用）；
//! - `mek_rotation_scheduler`：90 天周期轮换检查后台任务；
//! - `crypto_service`：v1 遗留薄封装（零生产消费者，仅测试引用）。
//!
//! 本模块**不对外暴露实现**：仅 `auth/mod.rs` 按需 re-export（门面装配工厂与命令模块）。

pub mod crypto_service;
pub mod mek_manager;
pub mod mek_rotation_commands;
pub mod mek_rotation_scheduler;
