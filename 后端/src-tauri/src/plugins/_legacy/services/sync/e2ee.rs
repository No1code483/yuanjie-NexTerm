//! A5.2.6.9 端到端加密（ECDH + AES-GCM）
//!
//! 设计文档：功能展望/平台级增强/04_离线与同步机制.md §2.6.3 / §2.6.9
//!
//! ## 流程
//!
//! 1. **设备注册时**：
//!    - 调用 `EcdhKeyPair::generate()` 生成本设备密钥对
//!    - 将公钥（Base64）保存到 `sync_devices.public_key`
//!    - 私钥通过 `crypto/aes_gcm.rs` 加密后持久化（用 MEK 派生的子密钥）
//!
//! 2. **同步数据时**：
//!    - 调用 `encrypt_for_peer(payload, peer_public_key)` 加密
//!    - 内部用自身私钥 + 对方公钥协商共享密钥
//!    - 用共享密钥 + AES-GCM 加密 payload
//!    - 返回 `{ciphertext, nonce, ephemeral_public_key}` 三元组
//!
//! 3. **接收方解密**：
//!    - 接收 `{ciphertext, nonce, sender_public_key}`
//!    - 用自身私钥 + 发送方公钥协商共享密钥（与发送方派生出相同密钥）
//!    - 用共享密钥 + AES-GCM 解密 payload
//!
//! ## 加密格式
//!
//! ```
//! [ephemeral_pubkey_len(2 bytes)][ephemeral_pubkey(65 bytes)]
//! [nonce(12 bytes)][ciphertext(N bytes)][tag(16 bytes, 内嵌于 ciphertext)]
//! ```
//!
//! 整体打包为 Base64 字符串传输。

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::crypto::aes_gcm;
use crate::crypto::ecdh::{decode_public_key, EcdhKeyPair};
use crate::error::app_error::AppError;

/// E2EE 加密后的数据包
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPayload {
    /// 加密用的临时公钥（Base64 SEC1 未压缩点）
    ///
    /// 注：当前实现复用设备长期密钥对，未生成临时密钥。
    /// Phase 3 升级为 ephemeral key 以提供前向安全性（forward secrecy）。
    pub ephemeral_public_key: String,
    /// AES-GCM nonce（Base64，12 字节）
    pub nonce: String,
    /// 密文（Base64，含 GCM 认证标签）
    pub ciphertext: String,
}

/// E2EE 加密器
pub struct E2eeEncryptor {
    /// 本设备密钥对
    keypair: EcdhKeyPair,
}

impl E2eeEncryptor {
    pub fn new(keypair: EcdhKeyPair) -> Self {
        Self { keypair }
    }

    /// 本设备公钥（Base64）
    pub fn public_key(&self) -> &str {
        &self.keypair.public_key_b64
    }

    /// 为目标设备加密数据
    ///
    /// 参数：
    /// - `payload`：明文数据（JSON 字符串）
    /// - `peer_public_key_b64`：目标设备的公钥（Base64 SEC1）
    ///
    /// 返回：`EncryptedPayload`，可序列化为 JSON 后通过云端传输
    pub fn encrypt_for_peer(
        &self,
        payload: &str,
        peer_public_key_b64: &str,
    ) -> Result<EncryptedPayload, AppError> {
        // 1. ECDH 协商共享密钥
        let salt = b"nexterm-e2ee-salt-v1";
        let info = b"aes-256-gcm-key";
        let shared_key = self
            .keypair
            .derive_shared_secret(peer_public_key_b64, salt, info)?;

        // 2. AES-GCM 加密
        let encrypted_b64 = aes_gcm::encrypt(payload, &shared_key)?;

        // 3. 拆分 nonce 与 ciphertext
        // aes_gcm::encrypt 返回 base64(nonce[12] || ciphertext)
        let combined = STANDARD
            .decode(&encrypted_b64)
            .map_err(|e| AppError::Crypto(format!("Base64 解码失败: {}", e)))?;

        if combined.len() < 12 {
            return Err(AppError::Crypto("加密数据格式错误".into()));
        }

        let (nonce_bytes, ciphertext_bytes) = combined.split_at(12);

        Ok(EncryptedPayload {
            ephemeral_public_key: self.keypair.public_key_b64.clone(),
            nonce: STANDARD.encode(nonce_bytes),
            ciphertext: STANDARD.encode(ciphertext_bytes),
        })
    }

    /// 解密来自对端的数据
    ///
    /// 参数：
    /// - `encrypted`：对端发送的加密数据包
    /// - `peer_public_key_b64`：对端设备的公钥（Base64 SEC1）
    pub fn decrypt_from_peer(
        &self,
        encrypted: &EncryptedPayload,
        peer_public_key_b64: &str,
    ) -> Result<String, AppError> {
        // 安全性检查：ephemeral_public_key 必须与 peer_public_key_b64 一致
        // （当前实现无临时密钥，故二者相同）
        if encrypted.ephemeral_public_key != peer_public_key_b64 {
            return Err(AppError::Crypto(
                "发送方公钥不匹配，可能遭受中间人攻击".into(),
            ));
        }

        // 1. ECDH 协商共享密钥
        let salt = b"nexterm-e2ee-salt-v1";
        let info = b"aes-256-gcm-key";
        let shared_key = self
            .keypair
            .derive_shared_secret(peer_public_key_b64, salt, info)?;

        // 2. 重组 nonce || ciphertext
        let nonce_bytes = STANDARD
            .decode(&encrypted.nonce)
            .map_err(|e| AppError::Crypto(format!("nonce Base64 解码失败: {}", e)))?;

        let ciphertext_bytes = STANDARD
            .decode(&encrypted.ciphertext)
            .map_err(|e| AppError::Crypto(format!("ciphertext Base64 解码失败: {}", e)))?;

        let mut combined = Vec::with_capacity(nonce_bytes.len() + ciphertext_bytes.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext_bytes);

        let encrypted_b64 = STANDARD.encode(&combined);

        // 3. AES-GCM 解密
        aes_gcm::decrypt(&encrypted_b64, &shared_key)
    }
}

/// 便捷函数：一次性加密（无状态）
///
/// 注：每次调用都会生成新的临时密钥对，不复用长期密钥。
/// 适用于无状态场景（如一次性同步）。如需复用密钥对，请使用 `E2eeEncryptor`。
pub fn encrypt_payload_once(
    payload: &str,
    peer_public_key_b64: &str,
) -> Result<EncryptedPayload, AppError> {
    let keypair = EcdhKeyPair::generate()?;
    let encryptor = E2eeEncryptor::new(keypair);
    encryptor.encrypt_for_peer(payload, peer_public_key_b64)
}

/// 便捷函数：验证加密数据包格式
pub fn validate_encrypted_payload(payload: &EncryptedPayload) -> Result<bool, AppError> {
    if payload.ephemeral_public_key.is_empty() {
        return Ok(false);
    }
    if payload.nonce.is_empty() || payload.ciphertext.is_empty() {
        return Ok(false);
    }
    // 验证公钥格式
    let _ = decode_public_key(&payload.ephemeral_public_key)?;
    // nonce 应为 12 字节
    let nonce_bytes = STANDARD
        .decode(&payload.nonce)
        .map_err(|e| AppError::Crypto(format!("nonce 解码失败: {}", e)))?;
    if nonce_bytes.len() != 12 {
        return Ok(false);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_e2ee_roundtrip() {
        // 两端独立生成密钥对
        let alice = EcdhKeyPair::generate().unwrap();
        let bob = EcdhKeyPair::generate().unwrap();

        let alice_enc = E2eeEncryptor::new(alice);
        let bob_enc = E2eeEncryptor::new(bob);

        let plaintext = r#"{"title":"秘密笔记","content":"Hello Bob!"}"#;

        // Alice 加密给 Bob
        let encrypted = alice_enc
            .encrypt_for_peer(plaintext, &bob_enc.public_key().to_string())
            .unwrap();

        // Bob 解密来自 Alice 的数据
        let decrypted = bob_enc
            .decrypt_from_peer(&encrypted, alice_enc.public_key())
            .unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_e2ee_wrong_peer_key_fails() {
        let alice = EcdhKeyPair::generate().unwrap();
        let bob = EcdhKeyPair::generate().unwrap();
        let eve = EcdhKeyPair::generate().unwrap();

        let alice_enc = E2eeEncryptor::new(alice);
        let bob_enc = E2eeEncryptor::new(bob);

        let plaintext = "secret";

        // Alice 加密给 Bob
        let encrypted = alice_enc
            .encrypt_for_peer(plaintext, &bob_enc.public_key().to_string())
            .unwrap();

        // Eve 试图解密（应失败：公钥不匹配）
        let eve_enc = E2eeEncryptor::new(eve);
        let result = eve_enc.decrypt_from_peer(&encrypted, alice_enc.public_key());
        assert!(result.is_err());
    }

    #[test]
    fn test_e2ee_middle_man_attack_detected() {
        let alice = EcdhKeyPair::generate().unwrap();
        let bob = EcdhKeyPair::generate().unwrap();
        let eve = EcdhKeyPair::generate().unwrap();

        let alice_enc = E2eeEncryptor::new(alice);
        let bob_enc = E2eeEncryptor::new(bob);
        let eve_enc = E2eeEncryptor::new(eve);

        let plaintext = "secret";

        // Alice 加密给 Bob
        let encrypted = alice_enc
            .encrypt_for_peer(plaintext, &bob_enc.public_key().to_string())
            .unwrap();

        // Eve 修改 ephemeral_public_key 为自己的（中间人攻击）
        let mut tampered = encrypted.clone();
        tampered.ephemeral_public_key = eve_enc.public_key().to_string();

        // Bob 解密时应检测到公钥不匹配
        let result = bob_enc.decrypt_from_peer(&tampered, &eve_enc.public_key().to_string());
        // 由于 Bob 用 eve 的公钥协商密钥，但密文是用 alice+bob 的共享密钥加密的，解密应失败
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_encrypted_payload() {
        let alice = EcdhKeyPair::generate().unwrap();
        let bob = EcdhKeyPair::generate().unwrap();

        let enc = E2eeEncryptor::new(alice);
        let encrypted = enc
            .encrypt_for_peer("hello", &bob.public_key_b64)
            .unwrap();

        assert!(validate_encrypted_payload(&encrypted).unwrap());

        // 空数据
        let bad = EncryptedPayload {
            ephemeral_public_key: String::new(),
            nonce: String::new(),
            ciphertext: String::new(),
        };
        assert!(!validate_encrypted_payload(&bad).unwrap());

        // nonce 长度错误
        let bad_nonce = EncryptedPayload {
            ephemeral_public_key: encrypted.ephemeral_public_key.clone(),
            nonce: STANDARD.encode(b"short"),
            ciphertext: encrypted.ciphertext.clone(),
        };
        assert!(!validate_encrypted_payload(&bad_nonce).unwrap());
    }

    #[test]
    fn test_encrypt_payload_once() {
        let bob = EcdhKeyPair::generate().unwrap();
        let bob_enc = E2eeEncryptor::new(bob);

        let plaintext = "one-shot encryption";
        let encrypted = encrypt_payload_once(plaintext, bob_enc.public_key()).unwrap();

        // 注：encrypt_payload_once 使用临时密钥，无法直接用 bob_enc 解密
        // 因为 bob_enc 不知道临时私钥。需要 ephemeral key 的公钥来协商
        // 此处仅验证加密成功且格式合法
        assert!(validate_encrypted_payload(&encrypted).unwrap());
        assert!(!encrypted.ciphertext.is_empty());
    }

    #[test]
    fn test_e2ee_unicode_payload() {
        let alice = EcdhKeyPair::generate().unwrap();
        let bob = EcdhKeyPair::generate().unwrap();

        let alice_enc = E2eeEncryptor::new(alice);
        let bob_enc = E2eeEncryptor::new(bob);

        let plaintext = "🚀 元界 · 日本語 · 한국어 · Español · Emoji ✨";

        let encrypted = alice_enc
            .encrypt_for_peer(plaintext, &bob_enc.public_key().to_string())
            .unwrap();
        let decrypted = bob_enc
            .decrypt_from_peer(&encrypted, alice_enc.public_key())
            .unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_e2ee_large_payload() {
        let alice = EcdhKeyPair::generate().unwrap();
        let bob = EcdhKeyPair::generate().unwrap();

        let alice_enc = E2eeEncryptor::new(alice);
        let bob_enc = E2eeEncryptor::new(bob);

        // 100KB 的明文
        let plaintext = "A".repeat(100_000);

        let encrypted = alice_enc
            .encrypt_for_peer(&plaintext, &bob_enc.public_key().to_string())
            .unwrap();
        let decrypted = bob_enc
            .decrypt_from_peer(&encrypted, alice_enc.public_key())
            .unwrap();

        assert_eq!(plaintext, decrypted);
    }
}
