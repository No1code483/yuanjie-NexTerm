use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rand::RngCore;

use crate::error::app_error::AppError;

const NONCE_LENGTH: usize = 12;
const MAX_PLAINTEXT_SIZE: usize = 10 * 1024 * 1024;

pub fn encrypt_bytes(plaintext: &[u8], key: &[u8; 32]) -> Result<(Vec<u8>, [u8; 12]), AppError> {
    if plaintext.len() > MAX_PLAINTEXT_SIZE {
        return Err(AppError::Crypto(format!(
            "明文数据过大: {} bytes (最大 {} bytes)",
            plaintext.len(),
            MAX_PLAINTEXT_SIZE
        )));
    }
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| AppError::Crypto("无效的密钥长度".into()))?;

    let mut nonce_bytes = [0u8; NONCE_LENGTH];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| AppError::Crypto(format!("加密失败: {}", e)))?;

    Ok((ciphertext, nonce_bytes))
}

pub fn decrypt_bytes(
    ciphertext: &[u8],
    key: &[u8; 32],
    nonce: &[u8; 12],
) -> Result<Vec<u8>, AppError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| AppError::Crypto("无效的密钥长度".into()))?;
    let nonce = Nonce::from_slice(nonce);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| AppError::Crypto("解密失败：密钥错误或数据损坏".into()))
}

pub fn encrypt(plaintext: &str, key: &[u8; 32]) -> Result<String, AppError> {
    let (ciphertext, nonce_bytes) = encrypt_bytes(plaintext.as_bytes(), key)?;

    let mut combined = Vec::with_capacity(NONCE_LENGTH + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(STANDARD.encode(&combined))
}

pub fn decrypt(ciphertext_b64: &str, key: &[u8; 32]) -> Result<String, AppError> {
    let combined = STANDARD
        .decode(ciphertext_b64)
        .map_err(|e| AppError::Crypto(format!("Base64 解码失败: {}", e)))?;

    if combined.len() < NONCE_LENGTH {
        return Err(AppError::Crypto("密文数据不完整".into()));
    }

    let (nonce_bytes, encrypted_data) = combined.split_at(NONCE_LENGTH);
    let nonce_ref: &[u8; 12] = nonce_bytes
        .try_into()
        .map_err(|_| AppError::Crypto("nonce 长度错误".into()))?;

    let plaintext = decrypt_bytes(encrypted_data, key, nonce_ref)?;

    String::from_utf8(plaintext)
        .map_err(|e| AppError::Crypto(format!("UTF-8 解码失败: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = i as u8;
        }
        key
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = make_key();
        let plaintext = "Hello, NexTerm·元界! 这是一条测试消息。";
        let encrypted = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_bytes_encrypt_decrypt_roundtrip() {
        let key = make_key();
        let plaintext = b"Hello, NexTerm! Binary data test.";
        let (ciphertext, nonce) = encrypt_bytes(plaintext, &key).unwrap();
        let decrypted = decrypt_bytes(&ciphertext, &key, &nonce).unwrap();
        assert_eq!(&decrypted[..], &plaintext[..]);
    }

    #[test]
    fn test_encrypt_produces_different_output() {
        let key = make_key();
        let plaintext = "same message";
        let enc1 = encrypt(plaintext, &key).unwrap();
        let enc2 = encrypt(plaintext, &key).unwrap();
        assert_ne!(enc1, enc2);
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let key = make_key();
        let mut wrong_key = make_key();
        wrong_key[0] = wrong_key[0].wrapping_add(1);

        let encrypted = encrypt("secret", &key).unwrap();
        let result = decrypt(&encrypted, &wrong_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_wrong_key_fails() {
        let key1 = make_key();
        let mut key2 = make_key();
        key2[0] = key2[0].wrapping_add(1);
        let plaintext = b"secret data";
        let (ciphertext, nonce) = encrypt_bytes(plaintext, &key1).unwrap();
        assert!(decrypt_bytes(&ciphertext, &key2, &nonce).is_err());
    }

    #[test]
    fn test_nonce_uniqueness() {
        let key = make_key();
        let plaintext = b"same data";
        let (_, nonce1) = encrypt_bytes(plaintext, &key).unwrap();
        let (_, nonce2) = encrypt_bytes(plaintext, &key).unwrap();
        assert_ne!(nonce1, nonce2);
    }

    #[test]
    fn test_decrypt_invalid_base64() {
        let key = make_key();
        let result = decrypt("not-valid-base64!!!", &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_truncated_data() {
        let key = make_key();
        let result = decrypt("YWJj", &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_string_roundtrip() {
        let key = make_key();
        let encrypted = encrypt("", &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!("", decrypted);
    }

    #[test]
    fn test_unicode_roundtrip() {
        let key = make_key();
        let plaintext = "🚀 元界 · 日本語 · 한국어 · Español";
        let encrypted = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_long_text_roundtrip() {
        let key = make_key();
        let plaintext = "A".repeat(10000);
        let encrypted = encrypt(&plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(plaintext, decrypted);
    }
}