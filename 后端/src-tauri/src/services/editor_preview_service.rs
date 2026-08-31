use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

use crate::crypto::aes_gcm;
use crate::error::app_error::AppError;
use crate::models::editor::DecryptedPreview;
use crate::services::editor_metadata_service;
use crate::services::editor_storage_service::EditorStorageService;

pub fn get_decrypted_preview(
    storage: &EditorStorageService,
    mek: &[u8; 32],
    doc_uuid: &str,
    content_type: &str,
) -> Result<DecryptedPreview, AppError> {
    let content_path = storage.content_path(doc_uuid);
    let raw = std::fs::read(&content_path).map_err(AppError::FileSystem)?;

    if raw.len() < 12 {
        return Err(AppError::Validation("文件过小".into()));
    }

    let (nonce_bytes, ciphertext) = raw.split_at(12);
    let nonce: [u8; 12] = nonce_bytes
        .try_into()
        .map_err(|_| AppError::Crypto("nonce 错误".into()))?;

    let plaintext = aes_gcm::decrypt_bytes(ciphertext, mek, &nonce)?;

    let mime_type = editor_metadata_service::get_mime_type(content_type).to_string();
    let base64_content = BASE64.encode(&plaintext);
    let file_size = plaintext.len() as i64;

    Ok(DecryptedPreview {
        doc_uuid: doc_uuid.to_string(),
        mime_type,
        base64_content,
        file_size,
    })
}