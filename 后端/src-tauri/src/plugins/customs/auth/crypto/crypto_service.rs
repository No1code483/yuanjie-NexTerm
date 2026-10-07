//! customs.auth 私有加密门面（阶段3 批次1a-2a：自内核 `src/services/` 收编）。
//!
//! 说明：本文件为 v1 遗留的薄封装集合（11 个静态方法），**全后端零生产调用**，
//! 仅 `tests/crypto_unit.rs` 引用；按批次1a-2 S1 缺口表处置为「搬入插件并登记为零消费者」，
//! 删除决定保留到阶段4 清理（规避误删公开封装）。
//!
//! 加密原语（AES-GCM / Argon2）实际来自内核 `crate::crypto::*`（加密基础设施，未收编）。

use crate::crypto::aes_gcm;
use crate::crypto::key_derivation;
use crate::crypto::mek_manager::MekManager;
use crate::error::app_error::AppError;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct CryptoService;

impl CryptoService {
    pub fn encrypt_data(
        plaintext: &[u8],
        mek: &[u8; 32],
    ) -> Result<(Vec<u8>, [u8; 12]), AppError> {
        aes_gcm::encrypt_bytes(plaintext, mek)
    }

    pub fn decrypt_data(
        ciphertext: &[u8],
        mek: &[u8; 32],
        nonce: &[u8; 12],
    ) -> Result<Vec<u8>, AppError> {
        aes_gcm::decrypt_bytes(ciphertext, mek, nonce)
    }

    pub fn derive_kek(password: &str, salt: &[u8]) -> Result<[u8; 32], AppError> {
        key_derivation::derive_kek(password, salt)
    }

    pub fn generate_salt() -> Vec<u8> {
        key_derivation::generate_salt()
    }

    pub fn hash_password(password: &str, salt: &[u8]) -> Result<String, AppError> {
        key_derivation::hash_password(password, salt)
    }

    pub fn generate_recovery_phrase(word_count: usize) -> String {
        key_derivation::generate_recovery_phrase(word_count)
    }

    pub fn hash_recovery_phrase(phrase: &str) -> Result<String, AppError> {
        key_derivation::hash_recovery_phrase(phrase)
    }

    pub fn verify_recovery_phrase(phrase: &str, hash: &str) -> Result<bool, AppError> {
        key_derivation::verify_recovery_phrase(phrase, hash)
    }

    pub async fn cache_mek(
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        mek: [u8; 32],
    ) {
        let mut mgr = mek_manager.write().await;
        mgr.cache_mek(user_id, mek);
    }

    pub async fn get_mek(
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
    ) -> Option<[u8; 32]> {
        let mgr = mek_manager.read().await;
        mgr.get_mek(user_id).copied()
    }

    pub async fn clear_mek(mek_manager: &Arc<RwLock<MekManager>>, user_id: i64) {
        let mut mgr = mek_manager.write().await;
        mgr.clear_mek(user_id);
    }
}
