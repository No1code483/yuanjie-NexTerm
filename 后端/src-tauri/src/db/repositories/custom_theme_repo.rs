//! 自定义主题仓储层（C1.5 / v1.51.7）
//!
//! 数据访问层，仅负责 SQL 操作。业务封装见 `services/custom_theme_service.rs`。
//! 规范：`功能展望/体验深化/01_主题自定义系统_未来展望.md` §2.5

use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::custom_theme::CustomTheme;

/// 列出所有自定义主题（按更新时间倒序）
pub async fn list(pool: &SqlitePool, user_id: i64) -> Result<Vec<CustomTheme>, AppError> {
    sqlx::query_as::<_, CustomTheme>(
        "SELECT * FROM custom_themes WHERE user_id = ? ORDER BY updated_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 按 id 查询单个自定义主题
pub async fn get_by_id(pool: &SqlitePool, user_id: i64, id: i64) -> Result<Option<CustomTheme>, AppError> {
    sqlx::query_as::<_, CustomTheme>("SELECT * FROM custom_themes WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

/// 按 name 查询单个自定义主题
pub async fn get_by_name(pool: &SqlitePool, user_id: i64, name: &str) -> Result<Option<CustomTheme>, AppError> {
    sqlx::query_as::<_, CustomTheme>("SELECT * FROM custom_themes WHERE name = ? AND user_id = ?")
        .bind(name)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

/// 新增或更新自定义主题（UPSERT 语义：按 name 唯一约束冲突时更新）
///
/// 返回写入后的完整记录（含 id 和时间戳）
pub async fn upsert(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    base_theme: &str,
    variables: &str,
    now: i64,
) -> Result<CustomTheme, AppError> {
    sqlx::query_as::<_, CustomTheme>(
        "INSERT INTO custom_themes (user_id, name, base_theme, variables, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(user_id, name) DO UPDATE SET base_theme = ?, variables = ?, updated_at = ?
         RETURNING *",
    )
    .bind(user_id)
    .bind(name)
    .bind(base_theme)
    .bind(variables)
    .bind(now)
    .bind(now)
    .bind(base_theme)
    .bind(variables)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

/// 按 id 删除自定义主题
///
/// 返回受影响行数（0 表示未找到）
pub async fn delete_by_id(pool: &SqlitePool, user_id: i64, id: i64) -> Result<u64, AppError> {
    let result = sqlx::query("DELETE FROM custom_themes WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(result.rows_affected())
}
