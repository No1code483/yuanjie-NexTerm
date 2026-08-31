use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::terminal::{TerminalConfig, TerminalConfigInput, TerminalHistory, TerminalHistoryInput, TerminalSession, TerminalSessionInput, TerminalTabLayout, TerminalTabLayoutInput};

pub async fn save_history(
    pool: &SqlitePool,
    user_id: i64,
    input: &TerminalHistoryInput,
    now: i64,
) -> Result<TerminalHistory, AppError> {
    sqlx::query_as::<_, TerminalHistory>(
        "INSERT INTO terminal_history (user_id, command, output, exit_code, session_type, duration_ms, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(user_id)
    .bind(&input.command)
    .bind(&input.output)
    .bind(input.exit_code)
    .bind(&input.session_type)
    .bind(input.duration_ms)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_history(
    pool: &SqlitePool,
    user_id: i64,
    limit: i64,
    session_type: Option<&str>,
) -> Result<Vec<TerminalHistory>, AppError> {
    if let Some(st) = session_type {
        sqlx::query_as::<_, TerminalHistory>(
            "SELECT * FROM terminal_history WHERE user_id = ? AND session_type = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(user_id)
        .bind(st)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    } else {
        sqlx::query_as::<_, TerminalHistory>(
            "SELECT * FROM terminal_history WHERE user_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    }
}

pub async fn clear_history(pool: &SqlitePool, user_id: i64, session_type: Option<&str>) -> Result<u64, AppError> {
    let affected = if let Some(st) = session_type {
        sqlx::query("DELETE FROM terminal_history WHERE user_id = ? AND session_type = ?")
            .bind(user_id)
            .bind(st)
            .execute(pool)
            .await
            .map_err(AppError::Database)?
            .rows_affected()
    } else {
        sqlx::query("DELETE FROM terminal_history WHERE user_id = ?")
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(AppError::Database)?
            .rows_affected()
    };
    Ok(affected)
}

pub async fn get_history_count(pool: &SqlitePool, user_id: i64) -> Result<i64, AppError> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM terminal_history WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(row.0)
}

pub async fn search_history(
    pool: &SqlitePool,
    user_id: i64,
    keyword: &str,
    limit: i64,
) -> Result<Vec<TerminalHistory>, AppError> {
    let pattern = format!("%{}%", keyword);
    sqlx::query_as::<_, TerminalHistory>(
        "SELECT * FROM terminal_history WHERE user_id = ? AND (command LIKE ? OR output LIKE ?) ORDER BY created_at DESC LIMIT ?",
    )
    .bind(user_id)
    .bind(&pattern)
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 使用 FTS5 全文搜索（支持拼音、模糊匹配、排名）
pub async fn search_history_fts(
    pool: &SqlitePool,
    user_id: i64,
    keyword: &str,
    limit: i64,
) -> Result<Vec<TerminalHistory>, AppError> {
    let fts_query = keyword
        .split_whitespace()
        .map(|w| format!("\"{}\"*", w.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" OR ");

    sqlx::query_as::<_, TerminalHistory>(
        "SELECT th.* FROM terminal_history th
         INNER JOIN terminal_history_fts fts ON th.id = fts.rowid
         WHERE terminal_history_fts MATCH ? AND th.user_id = ?
         ORDER BY rank
         LIMIT ?",
    )
    .bind(&fts_query)
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn save_tab_layout(
    pool: &SqlitePool,
    user_id: i64,
    tabs: &[TerminalTabLayoutInput],
    now: i64,
) -> Result<(), AppError> {
    // 仅删除当前用户的标签布局（多用户隔离）
    sqlx::query("DELETE FROM terminal_tab_layout WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

    for tab in tabs {
        sqlx::query(
            "INSERT INTO terminal_tab_layout (user_id, tab_id, tab_type, title, sort_order, is_active, pane_data, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(user_id)
        .bind(&tab.tab_id)
        .bind(&tab.tab_type)
        .bind(&tab.title)
        .bind(tab.sort_order)
        .bind(tab.is_active)
        .bind(&tab.pane_data)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    }
    Ok(())
}

pub async fn load_tab_layout(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<TerminalTabLayout>, AppError> {
    sqlx::query_as::<_, TerminalTabLayout>(
        "SELECT * FROM terminal_tab_layout WHERE user_id = ? ORDER BY sort_order ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn clear_tab_layout(pool: &SqlitePool, user_id: i64) -> Result<u64, AppError> {
    let affected = sqlx::query("DELETE FROM terminal_tab_layout WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?
        .rows_affected();
    Ok(affected)
}

pub async fn save_session(
    pool: &SqlitePool,
    user_id: i64,
    input: &TerminalSessionInput,
    now: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT OR REPLACE INTO terminal_sessions (id, user_id, session_type, tab_id, pane_id, cols, rows, status, created_at, killed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, 'active', ?, NULL)",
    )
    .bind(&input.id)
    .bind(user_id)
    .bind(&input.session_type)
    .bind(&input.tab_id)
    .bind(&input.pane_id)
    .bind(input.cols)
    .bind(input.rows)
    .bind(now)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn update_session_killed(
    pool: &SqlitePool,
    user_id: i64,
    session_id: &str,
    now: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE terminal_sessions SET status = 'killed', killed_at = ? WHERE id = ? AND user_id = ?",
    )
    .bind(now)
    .bind(session_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn get_active_sessions(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<TerminalSession>, AppError> {
    sqlx::query_as::<_, TerminalSession>(
        "SELECT * FROM terminal_sessions WHERE status = 'active' AND user_id = ? ORDER BY created_at ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_session(
    pool: &SqlitePool,
    user_id: i64,
    session_id: &str,
) -> Result<u64, AppError> {
    let affected = sqlx::query("DELETE FROM terminal_sessions WHERE id = ? AND user_id = ?")
        .bind(session_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?
        .rows_affected();
    Ok(affected)
}

pub async fn delete_all_sessions(pool: &SqlitePool, user_id: i64) -> Result<u64, AppError> {
    let affected = sqlx::query("DELETE FROM terminal_sessions WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?
        .rows_affected();
    Ok(affected)
}

// ========== 终端配置 ==========

pub async fn get_terminal_config(pool: &SqlitePool) -> Result<TerminalConfig, AppError> {
    sqlx::query_as::<_, TerminalConfig>(
        "SELECT * FROM terminal_config WHERE id = 'default'",
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::TerminalError("终端配置不存在".into()))
}

pub async fn save_terminal_config(
    pool: &SqlitePool,
    input: &TerminalConfigInput,
    now: i64,
) -> Result<TerminalConfig, AppError> {
    let current = get_terminal_config(pool).await.unwrap_or(TerminalConfig {
        id: "default".into(),
        font_family: "Cascadia Code, Consolas, monospace".into(),
        font_size: 14,
        line_height: 1.2,
        cursor_style: "block".into(),
        cursor_blink: true,
        theme_name: "Dark+".into(),
        updated_at: 0,
    });

    sqlx::query_as::<_, TerminalConfig>(
        "INSERT OR REPLACE INTO terminal_config (id, font_family, font_size, line_height, cursor_style, cursor_blink, theme_name, updated_at)
         VALUES ('default', ?, ?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(input.font_family.as_deref().unwrap_or(&current.font_family))
    .bind(input.font_size.unwrap_or(current.font_size))
    .bind(input.line_height.unwrap_or(current.line_height))
    .bind(input.cursor_style.as_deref().unwrap_or(&current.cursor_style))
    .bind(input.cursor_blink.unwrap_or(current.cursor_blink))
    .bind(input.theme_name.as_deref().unwrap_or(&current.theme_name))
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}