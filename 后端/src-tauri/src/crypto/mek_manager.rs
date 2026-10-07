//! MEK 内核门面（阶段3 批次1a-2a：crypto/MEK 收编）。
//!
//! 契约依据：`02_核心机制设计/02_后端插件架构设计.md` §七
//! 「`crypto/`（用户密钥）→ auth 插件私有 + kernel 仅留加密基础设施接口」。
//!
//! 结构：
//! - 本文件是**内核侧门面**：保留原类型名与方法签名，供全仓 53 个消费方
//!   （`&Arc<RwLock<MekManager>>` 形参）零改动继续使用；
//! - 实际实现由 `customs.auth` 插件提供（`kernel_api::MekProvider`），
//!   装配期在 `AppState::new` 注入，禁止反向依赖具体实现；
//! - `crypto/aes_gcm.rs` / `ecdh.rs` / `key_derivation.rs` 属加密原语，
//!   按契约留在内核（加密基础设施），不在本批收编范围。
//!
//! 并发：外层 `Arc<RwLock<MekManager>>` 承担串行化，门面自身不含锁。

pub use kernel_api::MekProvider;

use crate::error::app_error::AppError;

pub struct MekManager {
    /// 由 `customs.auth` 注入的密钥设施实现（唯一实例，装配期确定）
    provider: Box<dyn MekProvider>,
}

impl MekManager {
    pub fn new(provider: Box<dyn MekProvider>) -> Self {
        Self { provider }
    }

    pub fn cache_mek(&mut self, user_id: i64, mek: [u8; 32]) {
        self.provider.cache_mek(user_id, mek);
    }

    pub fn get_mek(&self, user_id: i64) -> Option<&[u8; 32]> {
        self.provider.get_mek(user_id)
    }

    pub fn clear_mek(&mut self, user_id: i64) {
        self.provider.clear_mek(user_id);
    }

    pub fn clear_all(&mut self) {
        self.provider.clear_all();
    }

    pub fn re_encrypt_mek(
        &self,
        mek: &[u8; 32],
        new_kek: &[u8; 32],
    ) -> Result<(Vec<u8>, [u8; 12]), AppError> {
        self.provider
            .re_encrypt_mek(mek, new_kek)
            .map_err(AppError::Crypto)
    }
}
