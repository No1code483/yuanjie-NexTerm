//! A5 离线与同步机制 - Phase 3 Task 3 新闻源离线缓存
//!
//! 设计文档：功能展望/平台级增强/04_离线与同步机制.md §Phase 3 Task 3
//!
//! ## 职责
//!
//! - `cache_news_items`: 在线拉取成功后将新闻项批量写入 `news_offline_cache` 表（UPSERT）
//! - `get_cached_news`: 离线时按 source 读取缓存新闻
//! - `get_cache_status`: 返回最新缓存时间 + 缓存条数，供前端展示「上次更新 N 分钟前」
//! - `cache_age_minutes`: 计算 cached_at 与当前的分钟差
//!
//! ## 与 news_service 的关系
//!
//! news_service::fetch_and_cache_news 在线拉取后写入 news_cache（主存储），
//! 调用方在拉取成功后调用本 service 的 cache_news_items 同步写入 news_offline_cache。
//! 本 service 不直接参与 RSS 拉取，仅负责离线快照的写入与读取。

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::error::app_error::AppError;

/// 离线缓存新闻项（对齐 news_offline_cache 表结构）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NewsCacheItem {
    pub id: String,
    pub source: String,
    pub title: String,
    pub content: Option<String>,
    pub url: Option<String>,
    pub published_at: Option<String>,
    pub fetched_at: String,
    pub cached_at: String,
}

/// 缓存状态摘要（供前端展示「上次更新 N 分钟前」+ 条数）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatus {
    /// 最新一次缓存写入时间（ISO 8601 字符串，对应 cached_at 字段）
    pub last_cached_at: Option<String>,
    /// 当前缓存条数
    pub total_count: i64,
    /// 缓存年龄（分钟），None 表示从未缓存过
    pub age_minutes: Option<i64>,
}

/// 批量写缓存（INSERT OR REPLACE）
///
/// - 调用方在 fetch_and_cache_news 成功后构造 NewsCacheItem 列表传入
/// - 使用 INSERT OR REPLACE 语义：相同 id（基于 source+url+title 哈希）覆盖旧记录
/// - cached_at 统一使用传入的 item.cached_at（由调用方生成一次，保证整批一致）
pub async fn cache_news_items(
    pool: &SqlitePool,
    items: Vec<NewsCacheItem>,
) -> Result<usize, AppError> {
    if items.is_empty() {
        return Ok(0);
    }

    let mut inserted = 0usize;
    for item in &items {
        sqlx::query(
            "INSERT OR REPLACE INTO news_offline_cache
             (id, source, title, content, url, published_at, fetched_at, cached_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&item.id)
        .bind(&item.source)
        .bind(&item.title)
        .bind(item.content.as_ref())
        .bind(item.url.as_ref())
        .bind(item.published_at.as_ref())
        .bind(&item.fetched_at)
        .bind(&item.cached_at)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        inserted += 1;
    }

    tracing::info!(
        inserted = inserted,
        "[A5-Phase3-Task3] 新闻离线缓存已写入 {} 条",
        inserted
    );
    Ok(inserted)
}

/// 读缓存（按 source 过滤，limit 限制条数）
///
/// - source = None：返回所有缓存的新闻（按 cached_at DESC 排序）
/// - source = Some(s)：仅返回该 source 的新闻
/// - limit 默认 50，上限 200（防止一次性返回过多数据）
pub async fn get_cached_news(
    pool: &SqlitePool,
    source: Option<&str>,
    limit: i64,
) -> Result<Vec<NewsCacheItem>, AppError> {
    let limit = limit.clamp(1, 200);

    let items: Vec<NewsCacheItem> = if let Some(src) = source {
        sqlx::query_as::<_, NewsCacheItem>(
            "SELECT id, source, title, content, url, published_at, fetched_at, cached_at
             FROM news_offline_cache
             WHERE source = ?
             ORDER BY cached_at DESC
             LIMIT ?",
        )
        .bind(src)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?
    } else {
        sqlx::query_as::<_, NewsCacheItem>(
            "SELECT id, source, title, content, url, published_at, fetched_at, cached_at
             FROM news_offline_cache
             ORDER BY cached_at DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?
    };

    Ok(items)
}

/// 返回最新缓存时间 + 条数 + 年龄（分钟）
///
/// 前端通过此接口展示「上次更新 N 分钟前」+ 📴 离线标识。
/// 若缓存为空，last_cached_at 与 age_minutes 均为 None。
pub async fn get_cache_status(pool: &SqlitePool) -> Result<CacheStatus, AppError> {
    let row: Option<(Option<String>, i64)> = sqlx::query_as(
        "SELECT MAX(cached_at) AS max_cached_at, COUNT(*) AS total
         FROM news_offline_cache",
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    match row {
        Some((Some(last_cached_at), total_count)) if total_count > 0 => {
            let age_minutes = cache_age_minutes(&last_cached_at);
            Ok(CacheStatus {
                last_cached_at: Some(last_cached_at),
                total_count,
                age_minutes: Some(age_minutes),
            })
        }
        _ => Ok(CacheStatus {
            last_cached_at: None,
            total_count: 0,
            age_minutes: None,
        }),
    }
}

/// 计算缓存年龄（分钟）
///
/// cached_at 期望为 ISO 8601 字符串（如 "2026-07-24T12:34:56+08:00"）。
/// 解析失败或时间反转时返回 0（视为刚缓存）。
pub fn cache_age_minutes(cached_at: &str) -> i64 {
    // 尝试多种格式解析，统一映射为 i64 时间戳避免类型不匹配
    let cached_ts = chrono::DateTime::parse_from_str(cached_at, "%Y-%m-%dT%H:%M:%S%:z")
        .map(|dt| dt.timestamp())
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(cached_at, "%Y-%m-%dT%H:%M:%S")
                .map(|ndt| ndt.and_utc().timestamp())
        })
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(cached_at, "%Y-%m-%d %H:%M:%S")
                .map(|ndt| ndt.and_utc().timestamp())
        })
        .unwrap_or(0);

    let now = chrono::Utc::now().timestamp();
    let diff_secs = (now - cached_ts).max(0);
    diff_secs / 60
}

/// 基于 source + url + title 生成稳定的缓存 ID
///
/// - 用于 news_offline_cache.id（TEXT PRIMARY KEY）
/// - 同一 source+url+title 组合始终生成相同 ID，保证 UPSERT 语义
pub fn build_cache_id(source: &str, url: Option<&str>, title: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    url.unwrap_or("").hash(&mut hasher);
    title.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
