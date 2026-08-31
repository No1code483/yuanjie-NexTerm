use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::permission::Permission;

pub async fn get_permissions_by_role(
    pool: &SqlitePool,
    role: &str,
) -> Result<Vec<Permission>, AppError> {
    sqlx::query_as::<_, Permission>("SELECT * FROM permissions WHERE role = ?")
        .bind(role)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn check_permission(
    pool: &SqlitePool,
    role: &str,
    resource: &str,
    action: &str,
) -> Result<bool, AppError> {
    let perm = sqlx::query_as::<_, Permission>(
        "SELECT * FROM permissions WHERE role = ? AND resource = ?",
    )
    .bind(role)
    .bind(resource)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    match perm {
        Some(p) => match action {
            "read" => Ok(p.can_read),
            "write" => Ok(p.can_write),
            "delete" => Ok(p.can_delete),
            "modify" => Ok(p.can_modify),
            _ => Ok(false),
        },
        None => Ok(false),
    }
}

pub async fn init_default_permissions(pool: &SqlitePool) -> Result<(), AppError> {
    let defaults = vec![
        // ===== admin: 全部资源完全访问 =====
        ("admin", "home", true, true, true, true),
        ("admin", "home_news", true, true, true, true),
        ("admin", "home_todo", true, true, true, true),
        ("admin", "home_log", true, true, true, true),
        ("admin", "home_timer", true, true, true, true),
        ("admin", "ai_chat", true, true, true, true),
        ("admin", "knowledge", true, true, true, true),
        ("admin", "terminal", true, true, true, true),
        ("admin", "terminal_manual", true, false, false, false),
        ("admin", "terminal_yuancode", true, true, true, true),
        ("admin", "terminal_linux", true, true, true, true),
        ("admin", "game", true, true, true, true),
        ("admin", "profile_settings", true, true, true, true),
        ("admin", "recycle_bin", true, true, true, true),
        ("admin", "search", true, true, false, true),
        ("admin", "xin", true, true, false, true),
        ("admin", "spyglass", true, true, false, true),

        // ===== user: 普通永久账号（有限制）=====
        ("user", "home", true, true, false, true),
        ("user", "home_news", true, true, false, true),
        ("user", "home_todo", true, true, false, true),
        ("user", "home_log", true, true, false, true),
        ("user", "home_timer", true, true, false, true),
        ("user", "ai_chat", true, true, false, true),
        ("user", "knowledge", true, true, false, true),
        ("user", "terminal", true, true, false, true),
        ("user", "terminal_manual", true, false, false, false),
        ("user", "terminal_yuancode", true, true, false, true),
        ("user", "terminal_linux", true, true, false, true),
        ("user", "game", true, false, false, false),
        ("user", "profile_settings", true, true, false, true),
        ("user", "recycle_bin", true, true, false, true),
        ("user", "search", true, true, false, true),
        ("user", "xin", true, true, false, true),
        ("user", "spyglass", true, true, false, true),

        // ===== guest: 临时账号（只读为主）=====
        ("guest", "home", true, false, false, false),
        ("guest", "home_news", true, false, false, false),
        ("guest", "home_todo", true, false, false, false),
        ("guest", "home_log", true, false, false, false),
        ("guest", "home_timer", true, false, false, false),
        ("guest", "ai_chat", true, true, false, false),
        ("guest", "knowledge", true, false, false, false),
        ("guest", "terminal", false, false, false, false),
        ("guest", "terminal_manual", true, false, false, false),
        ("guest", "terminal_yuancode", true, false, false, false),
        ("guest", "terminal_linux", true, false, false, false),
        ("guest", "game", true, false, false, false),
        ("guest", "profile_settings", true, false, false, false),
        ("guest", "recycle_bin", false, false, false, false),
        ("guest", "search", true, false, false, false),
        ("guest", "xin", true, false, false, false),
        ("guest", "spyglass", false, false, false, false),
    ];

    for (role, resource, can_read, can_write, can_delete, can_modify) in defaults {
        sqlx::query(
            "INSERT OR REPLACE INTO permissions (role, resource, can_read, can_write, can_delete, can_modify)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(role)
        .bind(resource)
        .bind(can_read)
        .bind(can_write)
        .bind(can_delete)
        .bind(can_modify)
        .execute(pool)
        .await?;
    }

    Ok(())
}