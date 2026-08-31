use sqlx::SqlitePool;
use std::path::PathBuf;

use crate::db::repositories::editor_repo;
use crate::error::app_error::AppError;
use crate::models::editor::{
    AutoSaveRequest, EditorContent, EditorVersion, OpenDocumentRequest,
    RecoveredSession, SaveDocumentRequest,
};
use crate::services::editor_storage_service::EditorStorageService;
use crate::services::storage_service::StorageService;

const DEBOUNCE_MS: u64 = 300;

static LAST_AUTO_SAVE_TS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

pub async fn open_document(
    pool: &SqlitePool,
    storage: &EditorStorageService,
    mek: &[u8; 32],
    req: OpenDocumentRequest,
) -> Result<EditorContent, AppError> {
    let now = chrono::Utc::now().timestamp_millis();

    let doc = if let Some(source_id) = req.source_id {
        editor_repo::get_document_by_source(pool, &req.source_type, source_id).await?
    } else {
        None
    };

    let (doc_uuid, content) = if let Some(existing) = doc {
        let stored_content = if storage.content_path(&existing.doc_uuid).exists() {
            storage.read_encrypted(&storage.content_path(&existing.doc_uuid), mek)?
        } else {
            req.initial_content.unwrap_or_default()
        };

        editor_repo::update_document_size(
            pool,
            &existing.doc_uuid,
            stored_content.len() as i64,
            now,
        )
        .await?;

        (existing.doc_uuid, stored_content)
    } else {
        let new_uuid = uuid::Uuid::new_v4().to_string();
        let initial_content = req.initial_content.unwrap_or_default();
        let content_path = storage.content_path(&new_uuid);
        let file_size = storage.write_encrypted(&content_path, &initial_content, mek)?;

        editor_repo::create_document(
            pool,
            &new_uuid,
            &req.title,
            &req.source_type,
            req.source_id,
            &req.content_type,
            file_size,
            now,
        )
        .await?;

        (new_uuid, initial_content)
    };

    let language = detect_language_by_type(&req.content_type);

    let versions = editor_repo::get_versions(pool, &doc_uuid).await?;
    let current_doc = editor_repo::get_document(pool, &doc_uuid)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(EditorContent {
        doc_uuid,
        title: current_doc.title,
        content,
        content_type: req.content_type,
        language,
        file_size: current_doc.file_size,
        version_count: current_doc.version_count,
        versions,
    })
}

pub async fn save_document(
    pool: &SqlitePool,
    storage: &EditorStorageService,
    mek: &[u8; 32],
    req: SaveDocumentRequest,
) -> Result<EditorContent, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let doc = editor_repo::get_document(pool, &req.doc_uuid)
        .await?
        .ok_or(AppError::NotFound)?;

    let content_path = storage.content_path(&req.doc_uuid);
    let file_size = storage.write_encrypted(&content_path, &req.content, mek)?;

    let rows = editor_repo::update_document_with_version(
        pool,
        &req.doc_uuid,
        file_size,
        doc.version,
        now,
    )
    .await?;

    if rows == 0 {
        return Err(AppError::Conflict(
            "版本冲突: 文档已被其他操作修改，请重新打开后再保存".into(),
        ));
    }

    if req.create_version.unwrap_or(true) {
        let max_ver = editor_repo::get_max_version_num(pool, &req.doc_uuid).await?;
        let new_ver_num = max_ver + 1;

        storage.versions_dir(&req.doc_uuid);
        let ver_path = storage.version_path(&req.doc_uuid, new_ver_num);

        storage.write_encrypted(&ver_path, &req.content, mek)?;

        editor_repo::create_version(
            pool,
            &req.doc_uuid,
            new_ver_num,
            &ver_path.to_string_lossy(),
            file_size,
            req.change_summary.as_deref(),
            now,
        )
        .await?;

        editor_repo::increment_version_count(pool, &req.doc_uuid, now).await?;
    }

    editor_repo::delete_session(pool, &req.doc_uuid).await?;
    storage.delete_session(&req.doc_uuid)?;

    let updated_doc = editor_repo::get_document(pool, &req.doc_uuid)
        .await?
        .ok_or(AppError::NotFound)?;
    let versions = editor_repo::get_versions(pool, &req.doc_uuid).await?;
    let language = detect_language_by_type(&updated_doc.content_type);

    Ok(EditorContent {
        doc_uuid: req.doc_uuid,
        title: updated_doc.title,
        content: req.content,
        content_type: updated_doc.content_type,
        language,
        file_size,
        version_count: updated_doc.version_count,
        versions,
    })
}

pub async fn auto_save(
    pool: &SqlitePool,
    storage: &EditorStorageService,
    mek: &[u8; 32],
    req: AutoSaveRequest,
) -> Result<(), AppError> {
    let _doc = editor_repo::get_document(pool, &req.doc_uuid)
        .await?
        .ok_or(AppError::NotFound)?;

    let now = chrono::Utc::now().timestamp_millis();

    let last_ts = LAST_AUTO_SAVE_TS.load(std::sync::atomic::Ordering::Relaxed);
    if now - last_ts < DEBOUNCE_MS as i64 {
        return Ok(());
    }

    LAST_AUTO_SAVE_TS.store(now, std::sync::atomic::Ordering::Relaxed);

    StorageService::auto_save(|| async {
        let session_path = storage.session_path(&req.doc_uuid);
        storage.write_encrypted(&session_path, &req.content, mek)?;

        editor_repo::upsert_session(
            pool,
            &req.doc_uuid,
            1,
            req.cursor_line,
            req.cursor_column,
            now,
        )
        .await?;

        Ok::<_, AppError>(())
    })
    .await
}

pub async fn close_document(
    pool: &SqlitePool,
    storage: &EditorStorageService,
    doc_uuid: &str,
) -> Result<(), AppError> {
    editor_repo::delete_session(pool, doc_uuid).await?;
    storage.delete_session(doc_uuid)?;
    Ok(())
}

pub async fn delete_document(
    pool: &SqlitePool,
    storage: &EditorStorageService,
    doc_uuid: &str,
) -> Result<(), AppError> {
    editor_repo::delete_session(pool, doc_uuid).await?;
    storage.delete_session(doc_uuid)?;
    storage.delete_document_dir(doc_uuid)?;
    editor_repo::delete_versions_by_doc(pool, doc_uuid).await?;
    editor_repo::delete_fts_index(pool, doc_uuid).await?;
    editor_repo::delete_document(pool, doc_uuid).await?;
    Ok(())
}

pub async fn get_versions(
    pool: &SqlitePool,
    _storage: &EditorStorageService,
    _mek: &[u8; 32],
    doc_uuid: &str,
) -> Result<Vec<EditorVersion>, AppError> {
    editor_repo::get_versions(pool, doc_uuid).await
}

pub async fn get_version_content(
    pool: &SqlitePool,
    storage: &EditorStorageService,
    mek: &[u8; 32],
    doc_uuid: &str,
    version_num: i64,
) -> Result<String, AppError> {
    let version = editor_repo::get_version(pool, doc_uuid, version_num)
        .await?
        .ok_or(AppError::NotFound)?;

    let path = PathBuf::from(&version.file_path);
    storage.read_encrypted(&path, mek)
}

pub async fn restore_version(
    pool: &SqlitePool,
    storage: &EditorStorageService,
    mek: &[u8; 32],
    doc_uuid: &str,
    version_num: i64,
) -> Result<EditorContent, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let doc = editor_repo::get_document(pool, doc_uuid)
        .await?
        .ok_or(AppError::NotFound)?;

    let version = editor_repo::get_version(pool, doc_uuid, version_num)
        .await?
        .ok_or(AppError::NotFound)?;

    let version_path = PathBuf::from(&version.file_path);
    let content = storage.read_encrypted(&version_path, mek)?;

    let content_path = storage.content_path(doc_uuid);
    let file_size = storage.write_encrypted(&content_path, &content, mek)?;

    let rows = editor_repo::update_document_with_version(
        pool, doc_uuid, file_size, doc.version, now,
    ).await?;

    if rows == 0 {
        return Err(AppError::Conflict(
            "版本冲突: 文档已被其他操作修改，请重试".into(),
        ));
    }

    let max_ver = editor_repo::get_max_version_num(pool, doc_uuid).await?;
    let new_ver_num = max_ver + 1;
    let new_ver_path = storage.version_path(doc_uuid, new_ver_num);
    storage.write_encrypted(&new_ver_path, &content, mek)?;

    editor_repo::create_version(
        pool, doc_uuid, new_ver_num, &new_ver_path.to_string_lossy(), file_size,
        Some(&format!("回滚到版本 v{}", version_num)), now,
    ).await?;

    editor_repo::increment_version_count(pool, doc_uuid, now).await?;

    let versions = editor_repo::get_versions(pool, doc_uuid).await?;
    let updated_doc = editor_repo::get_document(pool, doc_uuid)
        .await?
        .ok_or(AppError::NotFound)?;
    let language = detect_language_by_type(&updated_doc.content_type);

    Ok(EditorContent {
        doc_uuid: doc_uuid.to_string(),
        title: updated_doc.title,
        content,
        content_type: updated_doc.content_type,
        language,
        file_size,
        version_count: updated_doc.version_count,
        versions,
    })
}

pub async fn recover_session(
    pool: &SqlitePool,
    storage: &EditorStorageService,
    mek: &[u8; 32],
    doc_uuid: &str,
) -> Result<Option<RecoveredSession>, AppError> {
    let session = editor_repo::get_session(pool, doc_uuid).await?;
    let session = match session {
        Some(s) => s,
        None => return Ok(None),
    };

    if session.is_dirty == 0 {
        return Ok(None);
    }

    let doc = editor_repo::get_document(pool, doc_uuid)
        .await?
        .ok_or(AppError::NotFound)?;

    let session_path = storage.session_path(doc_uuid);
    if !session_path.exists() {
        return Ok(None);
    }

    let content = storage.read_encrypted(&session_path, mek)?;

    Ok(Some(RecoveredSession {
        doc_uuid: doc_uuid.to_string(),
        title: doc.title,
        content,
        content_type: doc.content_type,
        cursor_line: session.cursor_line,
        cursor_column: session.cursor_column,
        last_activity: session.last_activity,
    }))
}

pub async fn recover_all_sessions(
    pool: &SqlitePool,
    storage: &EditorStorageService,
    mek: &[u8; 32],
) -> Result<Vec<RecoveredSession>, AppError> {
    let dirty_sessions = editor_repo::get_all_dirty_sessions(pool).await?;
    let mut recovered = Vec::new();

    for session in &dirty_sessions {
        let doc = match editor_repo::get_document(pool, &session.doc_uuid).await? {
            Some(d) => d,
            None => continue,
        };

        let session_path = storage.session_path(&session.doc_uuid);
        if !session_path.exists() {
            continue;
        }

        let content = match storage.read_encrypted(&session_path, mek) {
            Ok(c) => c,
            Err(_) => continue,
        };

        recovered.push(RecoveredSession {
            doc_uuid: session.doc_uuid.clone(),
            title: doc.title,
            content,
            content_type: doc.content_type,
            cursor_line: session.cursor_line,
            cursor_column: session.cursor_column,
            last_activity: session.last_activity,
        });
    }

    Ok(recovered)
}

pub async fn cleanup_expired_sessions(
    pool: &SqlitePool,
    _storage: &EditorStorageService,
    max_age_hours: i64,
) -> Result<u64, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let threshold_ms = now - max_age_hours * 3600 * 1000;

    let removed = editor_repo::cleanup_expired_sessions(pool, threshold_ms).await?;

    if removed > 0 {
        tracing::info!(count = removed, "清理过期会话缓存");
    }

    Ok(removed)
}

pub async fn list_documents(pool: &SqlitePool) -> Result<Vec<crate::models::editor::EditorDocument>, AppError> {
    editor_repo::list_documents(pool).await
}

fn detect_language_by_type(content_type: &str) -> Option<String> {
    match content_type {
        "text/rust" | "text/rs" => Some("rust".into()),
        "text/typescript" | "text/ts" => Some("typescript".into()),
        "text/javascript" | "text/js" => Some("javascript".into()),
        "text/python" | "text/py" => Some("python".into()),
        "text/html" => Some("html".into()),
        "text/css" => Some("css".into()),
        "text/json" => Some("json".into()),
        "text/markdown" | "text/md" => Some("markdown".into()),
        "text/toml" => Some("toml".into()),
        "text/yaml" | "text/yml" => Some("yaml".into()),
        "text/sql" => Some("sql".into()),
        "text/shell" | "text/sh" => Some("shell".into()),
        "text/xml" => Some("xml".into()),
        _ => None,
    }
}