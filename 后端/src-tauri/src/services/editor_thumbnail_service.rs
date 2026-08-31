use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::imageops::FilterType;
use image::GenericImageView;

use crate::crypto::aes_gcm;
use crate::error::app_error::AppError;
use crate::models::editor::ThumbnailResult;
use crate::services::editor_storage_service::EditorStorageService;

const DEFAULT_MAX_WIDTH: u32 = 480;
const DEFAULT_MAX_HEIGHT: u32 = 360;

pub fn generate_thumbnail(
    storage: &EditorStorageService,
    mek: &[u8; 32],
    doc_uuid: &str,
    max_width: Option<u32>,
    max_height: Option<u32>,
) -> Result<ThumbnailResult, AppError> {
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

    let img = image::load_from_memory(&plaintext).map_err(|e| {
        AppError::Validation(format!("图片解析失败: {}", e))
    })?;

    let (orig_w, orig_h) = img.dimensions();
    let max_w = max_width.unwrap_or(DEFAULT_MAX_WIDTH);
    let max_h = max_height.unwrap_or(DEFAULT_MAX_HEIGHT);

    let (thumb_w, thumb_h) = if orig_w <= max_w && orig_h <= max_h {
        (orig_w, orig_h)
    } else {
        let ratio_w = max_w as f64 / orig_w as f64;
        let ratio_h = max_h as f64 / orig_h as f64;
        let ratio = ratio_w.min(ratio_h);
        (
            (orig_w as f64 * ratio) as u32,
            (orig_h as f64 * ratio) as u32,
        )
    };

    let thumbnail = img.resize_exact(thumb_w, thumb_h, FilterType::Lanczos3);

    let mut png_buf = Vec::new();
    thumbnail
        .write_to(
            &mut std::io::Cursor::new(&mut png_buf),
            image::ImageFormat::Png,
        )
        .map_err(|e| AppError::Crypto(format!("缩略图编码失败: {}", e)))?;

    let base64_png = BASE64.encode(&png_buf);

    Ok(ThumbnailResult {
        doc_uuid: doc_uuid.to_string(),
        width: thumb_w,
        height: thumb_h,
        base64_png,
    })
}