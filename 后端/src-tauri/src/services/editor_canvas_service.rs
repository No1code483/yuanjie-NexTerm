use std::fs;

use crate::error::app_error::AppError;
use crate::models::editor::{CanvasData, CanvasSaveRequest};
use crate::services::editor_storage_service::EditorStorageService;

const CANVAS_SUBDIR: &str = "canvas";
const CANVAS_FILE: &str = "canvas.enc";

pub fn save_canvas(
    storage: &EditorStorageService,
    mek: &[u8; 32],
    req: CanvasSaveRequest,
) -> Result<CanvasData, AppError> {
    let canvas_dir = storage.base_dir().join(CANVAS_SUBDIR).join(&req.doc_uuid);
    fs::create_dir_all(&canvas_dir).map_err(AppError::FileSystem)?;

    let canvas_path = canvas_dir.join(CANVAS_FILE);
    let json = serde_json::to_string(&req.objects)
        .map_err(|e| AppError::Crypto(format!("序列化失败: {}", e)))?;

    storage.write_encrypted_to(&canvas_path, &json, mek)?;

    let now = chrono::Utc::now().timestamp_millis();

    Ok(CanvasData {
        doc_uuid: req.doc_uuid,
        objects: req.objects,
        width: req.width,
        height: req.height,
        updated_at: now,
    })
}

pub fn load_canvas(
    storage: &EditorStorageService,
    mek: &[u8; 32],
    doc_uuid: &str,
) -> Result<Option<CanvasData>, AppError> {
    let canvas_path = storage.base_dir().join(CANVAS_SUBDIR).join(doc_uuid).join(CANVAS_FILE);

    if !canvas_path.exists() {
        return Ok(None);
    }

    let json = storage.read_encrypted_from(&canvas_path, mek)?;
    let objects = serde_json::from_str(&json)
        .map_err(|e| AppError::Crypto(format!("反序列化失败: {}", e)))?;

    let metadata = fs::metadata(&canvas_path).map_err(AppError::FileSystem)?;
    let updated_at = metadata
        .modified()
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64
        })
        .unwrap_or(0);

    Ok(Some(CanvasData {
        doc_uuid: doc_uuid.to_string(),
        objects,
        width: 0.0,
        height: 0.0,
        updated_at,
    }))
}

pub fn delete_canvas(
    storage: &EditorStorageService,
    doc_uuid: &str,
) -> Result<(), AppError> {
    let canvas_dir = storage.base_dir().join(CANVAS_SUBDIR).join(doc_uuid);
    if canvas_dir.exists() {
        fs::remove_dir_all(&canvas_dir).map_err(AppError::FileSystem)?;
    }
    Ok(())
}