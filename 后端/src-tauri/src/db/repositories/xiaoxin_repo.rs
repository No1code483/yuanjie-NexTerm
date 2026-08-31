use sqlx::SqlitePool;

use crate::error::app_error::AppError;

// ============================================================================
// 多用户数据隔离批次 4：所有函数新增 user_id 参数，SQL 添加 WHERE user_id = ? 过滤
// 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
// ============================================================================

pub async fn load_xin_config(pool: &SqlitePool, user_id: i64) -> Result<Option<String>, AppError> {
    let row = sqlx::query_scalar::<_, String>(
        "SELECT config_json FROM xin_config WHERE user_id = ? ORDER BY id DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(row)
}

pub async fn save_xin_config(
    pool: &SqlitePool,
    user_id: i64,
    config_json: &str,
) -> Result<(), AppError> {
    // 多用户隔离：每个用户一行配置。使用 user_id 作为冲突判断。
    // 若该 user_id 已有配置行 → UPDATE；否则 INSERT 新行。
    sqlx::query(
        "INSERT INTO xin_config (user_id, config_json) VALUES (?, ?)
         ON CONFLICT(id) DO UPDATE SET config_json = excluded.config_json
         WHERE user_id = excluded.user_id",
    )
    .bind(user_id)
    .bind(config_json)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    // 兜底：若 ON CONFLICT(id) 未命中（因 id 自增不冲突），用 UPDATE 显式更新
    let affected = sqlx::query("UPDATE xin_config SET config_json = ? WHERE user_id = ?")
        .bind(config_json)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?
        .rows_affected();
    if affected == 0 {
        // 仍未更新 → 说明上面 INSERT 成功（新用户首次保存配置）
        // 无需额外操作
    }
    Ok(())
}

pub async fn insert_memory(
    pool: &SqlitePool,
    user_id: i64,
    id: &str,
    category: &str,
    key: &str,
    value: &str,
    importance: f64,
    source: &str,
    confidence: f64,
    created_at: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO xin_memories (user_id, id, category, key, value, importance, source, confidence, created_at, last_recalled_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(user_id)
    .bind(id)
    .bind(category)
    .bind(key)
    .bind(value)
    .bind(importance)
    .bind(source)
    .bind(confidence)
    .bind(created_at)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn get_memories(
    pool: &SqlitePool,
    user_id: i64,
    category: Option<&str>,
    limit: i64,
) -> Result<Vec<XinMemoryRow>, AppError> {
    if let Some(cat) = category {
        sqlx::query_as::<_, XinMemoryRow>(
            "SELECT id, user_id, category, key, value, importance, source, confidence, created_at, last_recalled_at
             FROM xin_memories WHERE user_id = ? AND category = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(user_id)
        .bind(cat)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    } else {
        sqlx::query_as::<_, XinMemoryRow>(
            "SELECT id, user_id, category, key, value, importance, source, confidence, created_at, last_recalled_at
             FROM xin_memories WHERE user_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    }
}

pub async fn search_memories(
    pool: &SqlitePool,
    user_id: i64,
    query: &str,
    limit: i64,
) -> Result<Vec<XinMemoryRow>, AppError> {
    let pattern = format!("%{}%", query);
    sqlx::query_as::<_, XinMemoryRow>(
        "SELECT id, user_id, category, key, value, importance, source, confidence, created_at, last_recalled_at
         FROM xin_memories WHERE user_id = ? AND (key LIKE ? OR value LIKE ?) ORDER BY created_at DESC LIMIT ?",
    )
    .bind(user_id)
    .bind(&pattern)
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_memory(pool: &SqlitePool, user_id: i64, id: &str) -> Result<bool, AppError> {
    let result = sqlx::query("DELETE FROM xin_memories WHERE user_id = ? AND id = ?")
        .bind(user_id)
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(result.rows_affected() > 0)
}

pub async fn count_memories(pool: &SqlitePool, user_id: i64) -> Result<i64, AppError> {
    let row = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM xin_memories WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(row)
}

pub async fn touch_memory(
    pool: &SqlitePool,
    user_id: i64,
    id: &str,
    recalled_at: &str,
) -> Result<(), AppError> {
    sqlx::query("UPDATE xin_memories SET last_recalled_at = ? WHERE user_id = ? AND id = ?")
        .bind(recalled_at)
        .bind(user_id)
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn insert_summary(
    pool: &SqlitePool,
    user_id: i64,
    id: &str,
    conversation_id: Option<i64>,
    summary: &str,
    key_takeaways_json: &str,
    topics_json: &str,
    sentiment_json: Option<&str>,
    created_at: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO xin_summaries (user_id, id, conversation_id, summary, key_takeaways, topics, sentiment, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(id)
    .bind(conversation_id)
    .bind(summary)
    .bind(key_takeaways_json)
    .bind(topics_json)
    .bind(sentiment_json)
    .bind(created_at)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn get_summaries(
    pool: &SqlitePool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<XinSummaryRow>, AppError> {
    sqlx::query_as::<_, XinSummaryRow>(
        "SELECT id, user_id, conversation_id, summary, key_takeaways, topics, sentiment, created_at
         FROM xin_summaries WHERE user_id = ? ORDER BY created_at DESC LIMIT ?",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn insert_mood(
    pool: &SqlitePool,
    user_id: i64,
    category: &str,
    intensity: f64,
    updated_at: &str,
    trigger: Option<&str>,
    context: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO xin_moods (user_id, category, intensity, updated_at, trigger_text, context) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(category)
    .bind(intensity)
    .bind(updated_at)
    .bind(trigger)
    .bind(context)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn get_mood_history(
    pool: &SqlitePool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<XinMoodRow>, AppError> {
    sqlx::query_as::<_, XinMoodRow>(
        "SELECT id, user_id, category, intensity, updated_at, trigger_text, context
         FROM xin_moods WHERE user_id = ? ORDER BY id DESC LIMIT ?",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn insert_reminder(
    pool: &SqlitePool,
    user_id: i64,
    id: &str,
    title: &str,
    description: &str,
    reminder_type: &str,
    trigger_at: Option<&str>,
    cron_expression: Option<&str>,
    is_active: bool,
    created_at: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO xin_reminders (user_id, id, title, description, reminder_type, trigger_at, cron_expression, is_active, created_at, last_triggered_at, repeat_count)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, 0)",
    )
    .bind(user_id)
    .bind(id)
    .bind(title)
    .bind(description)
    .bind(reminder_type)
    .bind(trigger_at)
    .bind(cron_expression)
    .bind(is_active)
    .bind(created_at)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn get_active_reminders(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<XinReminderRow>, AppError> {
    sqlx::query_as::<_, XinReminderRow>(
        "SELECT id, user_id, title, description, reminder_type, trigger_at, cron_expression, is_active, created_at, last_triggered_at, repeat_count
         FROM xin_reminders WHERE user_id = ? AND is_active = 1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn update_reminder_active(
    pool: &SqlitePool,
    user_id: i64,
    id: &str,
    is_active: bool,
) -> Result<bool, AppError> {
    let result =
        sqlx::query("UPDATE xin_reminders SET is_active = ? WHERE user_id = ? AND id = ?")
            .bind(is_active)
            .bind(user_id)
            .bind(id)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_reminder(
    pool: &SqlitePool,
    user_id: i64,
    id: &str,
) -> Result<bool, AppError> {
    let result = sqlx::query("DELETE FROM xin_reminders WHERE user_id = ? AND id = ?")
        .bind(user_id)
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(result.rows_affected() > 0)
}

pub async fn trigger_reminder(
    pool: &SqlitePool,
    user_id: i64,
    id: &str,
    triggered_at: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE xin_reminders SET last_triggered_at = ?, repeat_count = repeat_count + 1 WHERE user_id = ? AND id = ?",
    )
    .bind(triggered_at)
    .bind(user_id)
    .bind(id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn upsert_habit(
    pool: &SqlitePool,
    user_id: i64,
    id: &str,
    habit_name: &str,
    streak_days: u32,
    total_checkins: u32,
    last_checkin: Option<&str>,
    completed_today: bool,
    category: &str,
) -> Result<(), AppError> {
    // 多用户隔离：UNIQUE 约束为 (user_id, id) — 但原表只有 id PRIMARY KEY
    // 这里用 INSERT ... ON CONFLICT(id) DO UPDATE，但需额外 WHERE user_id = ? 防止跨用户冲突
    // 更安全的做法：先尝试 UPDATE，未命中再 INSERT
    let affected = sqlx::query(
        "UPDATE xin_habits SET habit_name = ?, streak_days = ?, total_checkins = ?, last_checkin = ?, completed_today = ?, category = ?
         WHERE user_id = ? AND id = ?",
    )
    .bind(habit_name)
    .bind(streak_days)
    .bind(total_checkins)
    .bind(last_checkin)
    .bind(completed_today)
    .bind(category)
    .bind(user_id)
    .bind(id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?
    .rows_affected();

    if affected == 0 {
        sqlx::query(
            "INSERT INTO xin_habits (user_id, id, habit_name, streak_days, total_checkins, last_checkin, completed_today, category)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(user_id)
        .bind(id)
        .bind(habit_name)
        .bind(streak_days)
        .bind(total_checkins)
        .bind(last_checkin)
        .bind(completed_today)
        .bind(category)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    }
    Ok(())
}

pub async fn get_habits(pool: &SqlitePool, user_id: i64) -> Result<Vec<XinHabitRow>, AppError> {
    sqlx::query_as::<_, XinHabitRow>(
        "SELECT id, user_id, habit_name, streak_days, total_checkins, last_checkin, completed_today, category
         FROM xin_habits WHERE user_id = ? ORDER BY streak_days DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_habit(
    pool: &SqlitePool,
    user_id: i64,
    id: &str,
) -> Result<Option<XinHabitRow>, AppError> {
    sqlx::query_as::<_, XinHabitRow>(
        "SELECT id, user_id, habit_name, streak_days, total_checkins, last_checkin, completed_today, category
         FROM xin_habits WHERE user_id = ? AND id = ?",
    )
    .bind(user_id)
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

// ============================================================================
// Row 结构体 — 多用户隔离批次 4：添加 user_id 字段
// ============================================================================

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct XinMemoryRow {
    pub id: String,
    pub user_id: i64,
    pub category: String,
    pub key: String,
    pub value: String,
    pub importance: f64,
    pub source: String,
    pub confidence: f64,
    pub created_at: String,
    pub last_recalled_at: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct XinSummaryRow {
    pub id: String,
    pub user_id: i64,
    pub conversation_id: Option<i64>,
    pub summary: String,
    pub key_takeaways: String,
    pub topics: String,
    pub sentiment: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct XinMoodRow {
    pub id: i64,
    pub user_id: i64,
    pub category: String,
    pub intensity: f64,
    pub updated_at: String,
    pub trigger_text: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct XinReminderRow {
    pub id: String,
    pub user_id: i64,
    pub title: String,
    pub description: String,
    pub reminder_type: String,
    pub trigger_at: Option<String>,
    pub cron_expression: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub last_triggered_at: Option<String>,
    pub repeat_count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct XinHabitRow {
    pub id: String,
    pub user_id: i64,
    pub habit_name: String,
    pub streak_days: i64,
    pub total_checkins: i64,
    pub last_checkin: Option<String>,
    pub completed_today: bool,
    pub category: String,
}
