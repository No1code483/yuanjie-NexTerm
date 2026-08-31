use std::io::Cursor;

use image::GenericImageView;
use lopdf::{Document as PdfDocument, Object};

use crate::crypto::aes_gcm;
use crate::error::app_error::AppError;
use crate::models::editor::{MediaMetadata, MediaMetadataExtra};
use crate::services::editor_storage_service::EditorStorageService;

pub fn extract_metadata(
    storage: &EditorStorageService,
    mek: &[u8; 32],
    doc_uuid: &str,
    content_type: &str,
) -> Result<MediaMetadata, AppError> {
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
    let file_size = plaintext.len() as i64;

    let extra = match content_type {
        "image" | "png" | "jpeg" | "jpg" | "gif" | "webp" | "bmp" => {
            extract_image_metadata(&plaintext)?
        }
        "pdf" => extract_pdf_metadata(&plaintext)?,
        "video" | "mp4" | "webm" | "avi" | "mov" => extract_video_metadata(&plaintext),
        _ => MediaMetadataExtra {
            width: None,
            height: None,
            color_type: None,
            duration_secs: None,
            frame_count: None,
            page_count: None,
            author: None,
            title: None,
            extra_info: None,
        },
    };

    Ok(MediaMetadata {
        file_type: content_type.to_string(),
        file_size,
        extra,
    })
}

fn extract_image_metadata(data: &[u8]) -> Result<MediaMetadataExtra, AppError> {
    let img = image::load_from_memory(data).map_err(|e| {
        AppError::Validation(format!("图片解析失败: {}", e))
    })?;

    let (width, height) = img.dimensions();
    let color_type = match img.color() {
        image::ColorType::L8 => Some("L8".into()),
        image::ColorType::La8 => Some("La8".into()),
        image::ColorType::Rgb8 => Some("RGB8".into()),
        image::ColorType::Rgba8 => Some("RGBA8".into()),
        image::ColorType::L16 => Some("L16".into()),
        image::ColorType::La16 => Some("La16".into()),
        image::ColorType::Rgb16 => Some("RGB16".into()),
        image::ColorType::Rgba16 => Some("RGBA16".into()),
        image::ColorType::Rgb32F => Some("RGB32F".into()),
        image::ColorType::Rgba32F => Some("RGBA32F".into()),
        _ => Some("Unknown".into()),
    };

    Ok(MediaMetadataExtra {
        width: Some(width),
        height: Some(height),
        color_type,
        duration_secs: None,
        frame_count: None,
        page_count: None,
        author: None,
        title: None,
        extra_info: None,
    })
}

fn extract_pdf_metadata(data: &[u8]) -> Result<MediaMetadataExtra, AppError> {
    let cursor = Cursor::new(data);
    let doc = PdfDocument::load_from(cursor).map_err(|e| {
        AppError::Validation(format!("PDF 解析失败: {}", e))
    })?;

    let page_count = doc.get_pages().len() as u32;

    let info_ref = doc
        .trailer
        .get(b"Info")
        .map_err(|e| AppError::Validation(format!("PDF 信息获取失败: {}", e)))?;

    let info_dict = if let Object::Reference(id) = info_ref {
        doc.get_dictionary(*id).ok()
    } else if let Ok(d) = info_ref.as_dict() {
        Some(d)
    } else {
        None
    };

    let get_string_field = |field: &[u8]| -> Option<String> {
        info_dict
            .and_then(|dict| dict.get(field).ok())
            .and_then(|obj| {
                if let Object::Reference(id) = obj {
                    doc.get_object(*id).ok()
                } else {
                    Some(obj)
                }
            })
            .and_then(|obj| match obj {
                Object::String(s, _) => Some(String::from_utf8_lossy(s).to_string()),
                _ => None,
            })
    };

    let title = get_string_field(b"Title");
    let author = get_string_field(b"Author");

    Ok(MediaMetadataExtra {
        width: None,
        height: None,
        color_type: None,
        duration_secs: None,
        frame_count: None,
        page_count: Some(page_count),
        author,
        title,
        extra_info: None,
    })
}

fn extract_video_metadata(_data: &[u8]) -> MediaMetadataExtra {
    MediaMetadataExtra {
        width: None,
        height: None,
        color_type: None,
        duration_secs: None,
        frame_count: None,
        page_count: None,
        author: None,
        title: None,
        extra_info: Some("视频元数据需前端解析".into()),
    }
}

pub fn get_mime_type(content_type: &str) -> &str {
    match content_type {
        "image" | "png" => "image/png",
        "jpeg" | "jpg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "pdf" => "application/pdf",
        "video" | "mp4" => "video/mp4",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "plain" | "text" | "txt" => "text/plain",
        "markdown" | "md" => "text/markdown",
        "html" => "text/html",
        "css" => "text/css",
        "javascript" | "js" => "text/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        _ => "application/octet-stream",
    }
}