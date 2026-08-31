use sqlx::{Row, SqlitePool};

use chrono::Utc;

use crate::error::app_error::AppError;
use crate::models::recycle::RecycleBinItem;

// 多用户数据隔离（批次 6，migration 0121）：
//   recycle_bin 表已添加 user_id 字段（DEFAULT 1）。
//   所有面向用户的查询/修改/删除均按 user_id 过滤，防止跨用户访问回收站内容。
//   cleanup_expired 是系统级定时清理任务（按时间清除过期项），无需 user_id 过滤。

/// 获取指定用户的回收站项目列表（按删除时间倒序）
pub async fn get_items(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<RecycleBinItem>, AppError> {
    sqlx::query_as::<_, RecycleBinItem>(
        "SELECT id, user_id, original_path, item_type, item_id, title, metadata_json,
                file_size, deleted_by, deleted_at, auto_delete_at
         FROM recycle_bin
         WHERE user_id = ?
         ORDER BY deleted_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 按 id + user_id 查询单个回收站项目
///
/// 多用户隔离（批次 6）：必须同时匹配 user_id，防止跨用户读取。
pub async fn get_item_by_id(
    executor: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    id: i64,
    user_id: i64,
) -> Result<RecycleBinItem, AppError> {
    sqlx::query_as::<_, RecycleBinItem>(
        "SELECT id, user_id, original_path, item_type, item_id, title, metadata_json,
                file_size, deleted_by, deleted_at, auto_delete_at
         FROM recycle_bin
         WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(executor)
    .await
    .map_err(AppError::Database)?
    .ok_or(AppError::NotFound)
}

/// 新增回收站项目
///
/// 多用户隔离（批次 6）：写入时显式绑定 user_id，确保项目归属当前用户。
pub async fn add_item(
    executor: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    user_id: i64,
    original_path: &str,
    item_type: &str,
    item_id: Option<i64>,
    title: Option<&str>,
    metadata_json: Option<&str>,
    file_size: Option<i64>,
    deleted_by: Option<i64>,
    now: i64,
    auto_delete_at: i64,
) -> Result<RecycleBinItem, AppError> {
    sqlx::query_as::<_, RecycleBinItem>(
        "INSERT INTO recycle_bin
            (user_id, original_path, item_type, item_id, title, metadata_json,
             file_size, deleted_by, deleted_at, auto_delete_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id, user_id, original_path, item_type, item_id, title, metadata_json,
                   file_size, deleted_by, deleted_at, auto_delete_at",
    )
    .bind(user_id)
    .bind(original_path)
    .bind(item_type)
    .bind(item_id)
    .bind(title)
    .bind(metadata_json)
    .bind(file_size)
    .bind(deleted_by)
    .bind(now)
    .bind(auto_delete_at)
    .fetch_one(executor)
    .await
    .map_err(AppError::Database)
}

/// 删除单个回收站项目
///
/// 多用户隔离（批次 6）：必须同时匹配 user_id，防止跨用户删除。
pub async fn delete_item(
    executor: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    id: i64,
    user_id: i64,
) -> Result<(), AppError> {
    let rows = sqlx::query("DELETE FROM recycle_bin WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(executor)
        .await
        .map_err(AppError::Database)?
        .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

/// 批量删除回收站项目
///
/// 多用户隔离（批次 6）：必须同时匹配 user_id，防止跨用户删除。
pub async fn delete_items_batch(
    executor: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    user_id: i64,
    ids: &[i64],
) -> Result<u64, AppError> {
    if ids.is_empty() {
        return Ok(0);
    }

    let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "DELETE FROM recycle_bin WHERE id IN ({}) AND user_id = ?",
        placeholders.join(",")
    );

    let mut query = sqlx::query(&sql);
    for id in ids {
        query = query.bind(id);
    }
    query = query.bind(user_id);

    query
        .execute(executor)
        .await
        .map(|r| r.rows_affected())
        .map_err(AppError::Database)
}

/// 系统级定时清理：删除所有已过期的回收站项目（不区分用户）
///
/// 说明：这是后台维护任务，按 auto_delete_at 时间清除过期项。
/// 不按 user_id 过滤，因为过期清理对所有用户一视同仁。
pub async fn cleanup_expired(pool: &SqlitePool, now: i64) -> Result<u64, AppError> {
    sqlx::query("DELETE FROM recycle_bin WHERE auto_delete_at < ?")
        .bind(now)
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
        .map_err(AppError::Database)
}

/// 清空指定用户的回收站
///
/// 多用户隔离（批次 6）：仅清空当前用户的回收站，不影响其他用户。
pub async fn clear_all(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM recycle_bin WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

/// 获取指定用户回收站的统计信息
///
/// 多用户隔离（批次 6）：仅统计当前用户的回收站数据。
pub async fn get_stats(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<crate::models::recycle::RecycleStats, AppError> {
    let now = Utc::now().timestamp_millis();
    let three_days_later = now + 3 * 24 * 3600 * 1000;

    let row = sqlx::query(
        "SELECT
            COALESCE(SUM(file_size), 0) as total_size,
            COUNT(*) as file_count,
            COALESCE(MIN(deleted_at), 0) as oldest_date,
            COALESCE(SUM(CASE WHEN auto_delete_at < ? THEN 1 ELSE 0 END), 0) as expiring_count
         FROM recycle_bin
         WHERE user_id = ?",
    )
    .bind(three_days_later)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(crate::models::recycle::RecycleStats {
        total_size: row.get("total_size"),
        file_count: row.get("file_count"),
        oldest_date: row.get("oldest_date"),
        expiring_count: row.get("expiring_count"),
    })
}
