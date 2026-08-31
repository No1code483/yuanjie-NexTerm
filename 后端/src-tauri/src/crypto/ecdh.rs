//! A5.2.6.8 ECDH 密钥交换（NIST P-256 曲线）
//!
//! 设计文档：功能展望/平台级增强/04_离线与同步机制.md §2.6.3 / §2.6.8
//!
//! ## 用途
//!
//! 用于设备间端到端加密（E2EE）的密钥协商：
//! 1. 设备注册时生成密钥对（私钥本地存储，公钥上传到云端）
//! 2. 同步数据时，用对方公钥 + 自身私钥协商共享密钥
//! 3. 用共享密钥派生 AES-256 密钥加密同步数据
//! 4. 仅目标设备可解密，云端无法解密
//!
//! ## 选型
//!
//! - 曲线：NIST P-256（secp256r1），广泛部署，性能良好
//! - 库：p256 crate（RustCrypto 同栈，与 aes-gcm / sha2 兼容）
//! - 派生：HKDF-SHA256（从 ECDH 共享密钥派生 32 字节 AES 密钥）
//!
//! ## 安全性
//!
//! - 私钥永不出本设备（仅在内存中或加密存储在 sync_devices.public_key 反向不存）
//! - 公钥可公开传输（无法反推私钥）
//! - 共享密钥仅在内存中存在，用后即焚
//! - 派生 HKDF 时加入盐值（设备 ID 拼接）防止重放

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use hkdf::Hkdf;
use p256::ecdh::EphemeralSecret;
use p256::{EncodedPoint, PublicKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::error::app_error::AppError;

/// ECDH 密钥对
///
/// 注：`EphemeralSecret` 不实现 `Debug`（出于安全考虑），因此本结构也不实现 `Debug`。
pub struct EcdhKeyPair {
    /// 私钥（仅本设备持有，永不上传）
    pub secret: EphemeralSecret,
    /// 公钥（Base64 编码的 SEC1 uncompressed 点，可上传到云端）
    pub public_key_b64: String,
}

/// 序列化的密钥对（仅用于本地持久化，切勿上传）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedKeyPair {
    /// 私钥（Base64 编码的 PKCS#8）
    pub private_key_b64: String,
    /// 公钥（Base64 编码的 SEC1 uncompressed 点）
    pub public_key_b64: String,
}

impl EcdhKeyPair {
    /// 生成新的密钥对
    pub fn generate() -> Result<Self, AppError> {
        // 直接生成 EphemeralSecret（p256 0.13 API）
        let secret = EphemeralSecret::random(&mut OsRng);
        let public_key = secret.public_key();

        // 公钥编码为 SEC1 未压缩点（65 字节：04 || X || Y）
        let public_point: EncodedPoint = public_key.into();
        let public_key_b64 = STANDARD.encode(public_point.as_bytes());

        Ok(Self {
            secret,
            public_key_b64,
        })
    }

    /// 从序列化数据恢复密钥对
    ///
    /// 注：p256 0.13 的 `EphemeralSecret` 不支持从字节恢复（设计为短期密钥）。
    /// 此函数当前返回错误，TODO 在 Phase 3 改用 `NonZeroScalar` 支持长期密钥。
    pub fn from_serialized(_data: &SerializedKeyPair) -> Result<Self, AppError> {
        // TODO: Phase 3 改用 p256::NonZeroScalar + 长期密钥对，支持序列化与反序列化
        Err(AppError::Crypto(
            "EphemeralSecret 不支持从序列化数据恢复，请使用 generate() 重新生成密钥对".into(),
        ))
    }

    /// 序列化密钥对（用于本地持久化）
    ///
    /// **警告**：返回的数据包含私钥，必须加密后存储，绝不能上传到云端。
    ///
    /// 注：p256 0.13 的 `EphemeralSecret` 不支持序列化（设计为短期密钥）。
    /// 此函数当前返回错误，TODO 在 Phase 3 改用 `NonZeroScalar` 支持长期密钥。
    pub fn serialize(&self) -> Result<SerializedKeyPair, AppError> {
        Err(AppError::Crypto(
            "EphemeralSecret 不可序列化，请使用 generate() 重新生成或改用长期密钥".into(),
        ))
    }

    /// 与对方公钥协商共享密钥
    ///
    /// 参数 `peer_public_key_b64` 为对方公钥的 Base64 编码（SEC1 未压缩点）。
    /// 返回 32 字节共享密钥（已通过 HKDF-SHA256 派生）。
    pub fn derive_shared_secret(
        &self,
        peer_public_key_b64: &str,
        salt: &[u8],
        info: &[u8],
    ) -> Result<[u8; 32], AppError> {
        // 1. 解析对方公钥
        let peer_bytes = STANDARD
            .decode(peer_public_key_b64)
            .map_err(|e| AppError::Crypto(format!("对方公钥 Base64 解码失败: {}", e)))?;

        let peer_public_key = PublicKey::from_sec1_bytes(&peer_bytes)
            .map_err(|e| AppError::Crypto(format!("对方公钥 SEC1 解析失败: {}", e)))?;

        // 2. ECDH 协商（p256 0.13: diffie_hellman 直接返回 SharedSecret，非 Result）
        let shared_secret = self
            .secret
            .diffie_hellman(&peer_public_key);

        // 3. HKDF-SHA256 派生 32 字节密钥
        let raw_secret = shared_secret.raw_secret_bytes();
        let hk = Hkdf::<Sha256>::new(Some(salt), raw_secret.as_ref());
        let mut okm = [0u8; 32];
        hk.expand(info, &mut okm)
            .map_err(|e| AppError::Crypto(format!("HKDF 派生失败: {}", e)))?;

        Ok(okm)
    }
}

/// 工具函数：解析公钥 Base64 字符串，返回原始字节
pub fn decode_public_key(public_key_b64: &str) -> Result<Vec<u8>, AppError> {
    STANDARD
        .decode(public_key_b64)
        .map_err(|e| AppError::Crypto(format!("公钥 Base64 解码失败: {}", e)))
}

/// 工具函数：验证公钥格式是否合法
pub fn validate_public_key(public_key_b64: &str) -> Result<bool, AppError> {
    let bytes = decode_public_key(public_key_b64)?;
    // SEC1 未压缩点应为 65 字节（04 || X[32] || Y[32]）或 33 字节（压缩点）
    if bytes.len() != 65 && bytes.len() != 33 {
        return Ok(false);
    }
    // 尝试解析为 PublicKey
    PublicKey::from_sec1_bytes(&bytes)
        .map(|_| true)
        .map_err(|e| AppError::Crypto(format!("公钥格式无效: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let pair = EcdhKeyPair::generate().unwrap();
        assert!(!pair.public_key_b64.is_empty());

        // 公钥解码后应为 65 字节（未压缩点）
        let bytes = STANDARD.decode(&pair.public_key_b64).unwrap();
        assert_eq!(bytes.len(), 65);
        assert_eq!(bytes[0], 0x04); // 未压缩点标识
    }

    #[test]
    fn test_validate_public_key() {
        let pair = EcdhKeyPair::generate().unwrap();
        assert!(validate_public_key(&pair.public_key_b64).unwrap());

        // 无效公钥
        assert!(!validate_public_key("aGVsbG8=").unwrap()); // "hello" 的 base64
        assert!(validate_public_key("").is_err());
    }

    #[test]
    fn test_derive_shared_secret_symmetric() {
        // 两端独立生成的密钥对，交换公钥后应派生出相同的共享密钥
        let alice = EcdhKeyPair::generate().unwrap();
        let bob = EcdhKeyPair::generate().unwrap();

        let salt = b"nexterm-sync-salt";
        let info = b"e2ee-key-derivation";

        let alice_shared = alice
            .derive_shared_secret(&bob.public_key_b64, salt, info)
            .unwrap();
        let bob_shared = bob
            .derive_shared_secret(&alice.public_key_b64, salt, info)
            .unwrap();

        // ECDH 共享密钥应相等
        assert_eq!(alice_shared, bob_shared);
    }

    #[test]
    fn test_derive_shared_secret_with_different_info() {
        let alice = EcdhKeyPair::generate().unwrap();
        let bob = EcdhKeyPair::generate().unwrap();

        let salt = b"nexterm-sync-salt";
        let key1 = alice
            .derive_shared_secret(&bob.public_key_b64, salt, b"info-1")
            .unwrap();
        let key2 = alice
            .derive_shared_secret(&bob.public_key_b64, salt, b"info-2")
            .unwrap();

        // 不同 info 派生不同密钥
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_shared_secret_invalid_peer_key() {
        let pair = EcdhKeyPair::generate().unwrap();
        let result = pair.derive_shared_secret("invalid-base64!!!", b"", b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_ephemeral_not_supported() {
        let pair = EcdhKeyPair::generate().unwrap();
        // EphemeralSecret 不支持序列化（设计如此）
        assert!(pair.serialize().is_err());
    }

    #[test]
    fn test_keypair_uniqueness() {
        let pair1 = EcdhKeyPair::generate().unwrap();
        let pair2 = EcdhKeyPair::generate().unwrap();
        // 每次生成的密钥对应不同
        assert_ne!(pair1.public_key_b64, pair2.public_key_b64);
    }

    #[test]
    fn test_decode_public_key() {
        let pair = EcdhKeyPair::generate().unwrap();
        let bytes = decode_public_key(&pair.public_key_b64).unwrap();
        assert_eq!(bytes.len(), 65);
        assert_eq!(bytes[0], 0x04);
    }
}
