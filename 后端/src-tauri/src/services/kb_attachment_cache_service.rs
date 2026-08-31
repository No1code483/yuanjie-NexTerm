//! A5 离线与同步机制 - Phase 3 Task 4 知识库附件预加载
//!
//! 设计文档：功能展望/平台级增强/04_离线与同步机制.md §Phase 3 Task 4
//!
//! ## 职责
//!
//! - `pin_attachment`: 标记附件为「常用」+ 复制文件到本地缓存目录
//! - `unpin_attachment`: 取消标记 + 删除本地缓存文件
//! - `preload_attachment`: 刷新缓存（重新复制文件，更新 file_size）
//! - `list_pinned`: 列出所有已标记的附件
//! - `get_cached_path`: 获取本地缓存路径（离线打开时使用）
//! - `touch_access`: 更新 last_accessed_at（打开时调用）
//! - `evict_lru`: LRU 淘汰（30 天未访问 + 500MB 容量上限）
//!
//! ## 缓存目录结构
//!
//! `{app_data_dir}/attachment_cache/{entry_id}.{ext}`
//!
//! ## LRU 淘汰策略
//!
//! - 默认容量上限：500MB
//! - 默认过期时间：30 天未访问
//! - 淘汰顺序：按 last_accessed_at ASC（最久未访问先淘汰）
//! - 淘汰触发时机：pin_attachment 时检查（避免缓存无限增长）

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::error::app_error::AppError;

/// 默认 LRU 容量上限（500MB）
const DEFAULT_MAX_CACHE_SIZE_BYTES: u64 = 500 * 1024 * 1024;
/// 默认 LRU 过期时间（30 天）
const DEFAULT_MAX_AGE_DAYS: i64 = 30;

/// 已标记的附件缓存记录（对齐 kb_attachment_cache 表结构）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KbAttachmentCache {
    pub id: String,
    pub user_id: i64,
    pub entry_id: i64,
    pub file_path: String,
    pub file_size: i64,
    pub pinned_at: String,
    pub last_accessed_at: String,
    pub local_cache_path: String,
}

/// 获取缓存目录路径（{data_dir}/attachment_cache/），不存在则创建
fn ensure_cache_dir(data_dir: &Path) -> Result<PathBuf, AppError> {
    let cache_dir = data_dir.join("attachment_cache");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| AppError::FileSystem(e))?;
    Ok(cache_dir)
}

/// 从文件路径提取扩展名（小写，不含点号）
fn extract_extension(file_path: &str) -> String {
    Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin")
        .to_lowercase()
}

/// 构造本地缓存文件路径：{cache_dir}/{entry_id}.{ext}
fn build_local_cache_path(cache_dir: &Path, entry_id: i64, file_path: &str) -> PathBuf {
    let ext = extract_extension(file_path);
    cache_dir.join(format!("{}.{}", entry_id, ext))
}

/// 获取当前时间的 ISO 8601 字符串
fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// 标记附件为「常用」+ 预加载到本地缓存
///
/// - 若已标记，则刷新缓存（重新复制文件）
/// - 复制源文件到 `{data_dir}/attachment_cache/{entry_id}.{ext}`
/// - 自动触发 LRU 淘汰
pub async fn pin_attachment(
    pool: &SqlitePool,
    user_id: i64,
    entry_id: i64,
    file_path: &str,
    data_dir: &Path,
) -> Result<KbAttachmentCache, AppError> {
    // 1. 校验源文件存在
    let source = Path::new(file_path);
    if !source.exists() {
        return Err(AppError::Validation(format!(
            "源文件不存在: {}",
            file_path
        )));
    }

    // 2. 获取文件大小
    let file_size = std::fs::metadata(source)
        .map_err(AppError::FileSystem)?
        .len() as i64;

    // 3. 复制文件到缓存目录
    let cache_dir = ensure_cache_dir(data_dir)?;
    let local_cache_path = build_local_cache_path(&cache_dir, entry_id, file_path);
    std::fs::copy(source, &local_cache_path).map_err(AppError::FileSystem)?;

    let local_cache_str = local_cache_path.to_string_lossy().to_string();
    let now = now_iso();
    let id = entry_id.to_string();

    // 4. INSERT OR REPLACE（已标记则覆盖）— 多用户隔离批次 3 补全：写入 user_id
    sqlx::query(
        "INSERT OR REPLACE INTO kb_attachment_cache
         (id, user_id, entry_id, file_path, file_size, pinned_at, last_accessed_at, local_cache_path)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(user_id)
    .bind(entry_id)
    .bind(file_path)
    .bind(file_size)
    .bind(&now)
    .bind(&now)
    .bind(&local_cache_str)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    // 5. 触发 LRU 淘汰
    if let Err(e) = evict_lru(pool, data_dir).await {
        tracing::warn!("[A5-Phase3-Task4] LRU 淘汰失败: {:?}", e);
    }

    tracing::info!(
        entry_id = entry_id,
        file_size = file_size,
        "[A5-Phase3-Task4] 附件已标记常用并预加载: entry_id={}",
        entry_id
    );

    Ok(KbAttachmentCache {
        id,
        user_id,
        entry_id,
        file_path: file_path.to_string(),
        file_size,
        pinned_at: now.clone(),
        last_accessed_at: now,
        local_cache_path: local_cache_str,
    })
}

/// 取消标记 + 删除本地缓存文件
pub async fn unpin_attachment(
    pool: &SqlitePool,
    user_id: i64,
    entry_id: i64,
    _data_dir: &Path,
) -> Result<(), AppError> {
    // 1. 查询缓存记录，获取 local_cache_path（多用户隔离：按 user_id 过滤）
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT local_cache_path FROM kb_attachment_cache WHERE user_id = ? AND entry_id = ?",
    )
    .bind(user_id)
    .bind(entry_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    if let Some((local_cache_path,)) = row {
        // 2. 删除缓存文件
        let cache_file = Path::new(&local_cache_path);
        if cache_file.exists() {
            if let Err(e) = std::fs::remove_file(cache_file) {
                tracing::warn!(
                    "[A5-Phase3-Task4] 删除缓存文件失败: {:?} — {}",
                    cache_file,
                    e
                );
            }
        }
    }

    // 3. 删除数据库记录（多用户隔离：按 user_id 过滤）
    sqlx::query("DELETE FROM kb_attachment_cache WHERE user_id = ? AND entry_id = ?")
        .bind(user_id)
        .bind(entry_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

    tracing::info!(
        entry_id = entry_id,
        "[A5-Phase3-Task4] 附件已取消常用标记: entry_id={}",
        entry_id
    );

    Ok(())
}

/// 刷新缓存（重新复制文件，更新 file_size 和 last_accessed_at）
///
/// 用于源文件变更后刷新缓存。仅对已标记的附件有效。
pub async fn preload_attachment(
    pool: &SqlitePool,
    user_id: i64,
    entry_id: i64,
    data_dir: &Path,
) -> Result<KbAttachmentCache, AppError> {
    // 1. 查询现有记录（多用户隔离：按 user_id 过滤）
    let existing: Option<KbAttachmentCache> = sqlx::query_as::<_, KbAttachmentCache>(
        "SELECT id, user_id, entry_id, file_path, file_size, pinned_at, last_accessed_at, local_cache_path
         FROM kb_attachment_cache WHERE user_id = ? AND entry_id = ?",
    )
    .bind(user_id)
    .bind(entry_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    let existing = existing.ok_or(AppError::NotFound)?;

    // 2. 重新复制文件
    let source = Path::new(&existing.file_path);
    if !source.exists() {
        return Err(AppError::Validation(format!(
            "源文件不存在: {}",
            existing.file_path
        )));
    }

    let file_size = std::fs::metadata(source)
        .map_err(AppError::FileSystem)?
        .len() as i64;

    let cache_dir = ensure_cache_dir(data_dir)?;
    let local_cache_path = build_local_cache_path(&cache_dir, entry_id, &existing.file_path);
    std::fs::copy(source, &local_cache_path).map_err(AppError::FileSystem)?;

    let local_cache_str = local_cache_path.to_string_lossy().to_string();
    let now = now_iso();

    // 3. 更新记录（多用户隔离：按 user_id 过滤）
    sqlx::query(
        "UPDATE kb_attachment_cache
         SET file_size = ?, last_accessed_at = ?, local_cache_path = ?
         WHERE user_id = ? AND entry_id = ?",
    )
    .bind(file_size)
    .bind(&now)
    .bind(&local_cache_str)
    .bind(user_id)
    .bind(entry_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    tracing::info!(
        entry_id = entry_id,
        "[A5-Phase3-Task4] 附件缓存已刷新: entry_id={}",
        entry_id
    );

    Ok(KbAttachmentCache {
        id: existing.id,
        user_id: existing.user_id,
        entry_id,
        file_path: existing.file_path,
        file_size,
        pinned_at: existing.pinned_at,
        last_accessed_at: now,
        local_cache_path: local_cache_str,
    })
}

/// 列出所有已标记的附件（按 last_accessed_at DESC 排序）— 多用户隔离：按 user_id 过滤
pub async fn list_pinned(pool: &SqlitePool, user_id: i64) -> Result<Vec<KbAttachmentCache>, AppError> {
    let items: Vec<KbAttachmentCache> = sqlx::query_as::<_, KbAttachmentCache>(
        "SELECT id, user_id, entry_id, file_path, file_size, pinned_at, last_accessed_at, local_cache_path
         FROM kb_attachment_cache
         WHERE user_id = ?
         ORDER BY last_accessed_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(items)
}

/// 获取本地缓存路径（离线打开时使用）
///
/// 同时更新 last_accessed_at（touch access）。
/// 返回 None 表示该附件未被标记为常用。
pub async fn get_cached_path(
    pool: &SqlitePool,
    user_id: i64,
    entry_id: i64,
) -> Result<Option<String>, AppError> {
    // 多用户隔离：按 user_id 过滤
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT local_cache_path FROM kb_attachment_cache WHERE user_id = ? AND entry_id = ?",
    )
    .bind(user_id)
    .bind(entry_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    if row.is_none() {
        return Ok(None);
    }

    // touch access（多用户隔离：按 user_id 过滤）
    let now = now_iso();
    let _ = sqlx::query(
        "UPDATE kb_attachment_cache SET last_accessed_at = ? WHERE user_id = ? AND entry_id = ?",
    )
    .bind(&now)
    .bind(user_id)
    .bind(entry_id)
    .execute(pool)
    .await;

    Ok(row.map(|(p,)| p))
}

/// 更新 last_accessed_at（打开附件时调用）— 多用户隔离：按 user_id 过滤
pub async fn touch_access(pool: &SqlitePool, user_id: i64, entry_id: i64) -> Result<(), AppError> {
    let now = now_iso();
    sqlx::query("UPDATE kb_attachment_cache SET last_accessed_at = ? WHERE user_id = ? AND entry_id = ?")
        .bind(&now)
        .bind(user_id)
        .bind(entry_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

/// LRU 淘汰：删除超过 30 天未访问的记录 + 总容量超 500MB 时按最久未访问淘汰
///
/// 在 pin_attachment 时自动调用，也可手动调用。
pub async fn evict_lru(pool: &SqlitePool, _data_dir: &Path) -> Result<u64, AppError> {
    let mut evicted = 0u64;

    // 1. 删除超过 30 天未访问的记录
    let cutoff = chrono::Utc::now() - chrono::Duration::days(DEFAULT_MAX_AGE_DAYS);
    let cutoff_str = cutoff.to_rfc3339();

    let expired_rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT entry_id, local_cache_path FROM kb_attachment_cache
         WHERE last_accessed_at < ?",
    )
    .bind(&cutoff_str)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    for (entry_id, local_cache_path) in &expired_rows {
        let cache_file = Path::new(local_cache_path);
        if cache_file.exists() {
            let _ = std::fs::remove_file(cache_file);
        }
        sqlx::query("DELETE FROM kb_attachment_cache WHERE entry_id = ?")
            .bind(entry_id)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        evicted += 1;
    }

    // 2. 检查总容量，超限则按最久未访问淘汰
    let total_size: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(file_size), 0) FROM kb_attachment_cache",
    )
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    if total_size as u64 > DEFAULT_MAX_CACHE_SIZE_BYTES {
        // 按 last_accessed_at ASC 淘汰，直到总容量降至上限以下
        let overflow_rows: Vec<(i64, String)> = sqlx::query_as(
            "SELECT entry_id, local_cache_path FROM kb_attachment_cache
             ORDER BY last_accessed_at ASC",
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        let mut current_size = total_size as u64;
        for (entry_id, local_cache_path) in &overflow_rows {
            if current_size <= DEFAULT_MAX_CACHE_SIZE_BYTES {
                break;
            }

            // 获取该记录的 file_size
            let file_size: Option<i64> = sqlx::query_scalar(
                "SELECT file_size FROM kb_attachment_cache WHERE entry_id = ?",
            )
            .bind(entry_id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::Database)?;

            // 删除缓存文件
            let cache_file = Path::new(local_cache_path);
            if cache_file.exists() {
                let _ = std::fs::remove_file(cache_file);
            }

            // 删除数据库记录
            sqlx::query("DELETE FROM kb_attachment_cache WHERE entry_id = ?")
                .bind(entry_id)
                .execute(pool)
                .await
                .map_err(AppError::Database)?;

            if let Some(sz) = file_size {
                current_size = current_size.saturating_sub(sz as u64);
            }
            evicted += 1;
        }
    }

    if evicted > 0 {
        tracing::info!(
            evicted = evicted,
            "[A5-Phase3-Task4] LRU 淘汰完成: 删除 {} 条过期/超限缓存",
            evicted
        );
    }

    Ok(evicted)
}
