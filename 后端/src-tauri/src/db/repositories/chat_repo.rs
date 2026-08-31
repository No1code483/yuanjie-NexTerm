// 安全审计修复（多用户隔离批次 2）：所有方法添加 user_id 参数 + WHERE user_id = ? 过滤。
// 原实现：任何登录用户可读取/修改/删除他人的会话历史和消息。
// 现实现：所有查询/更新/删除操作强制按 user_id 过滤。
// 规范：安全审计报告/2026-07-25-代码安全审计报告.md 附录七
//
// 设计说明：
// - conversations / messages / conversation_participants 三张表均有 user_id 字段（migration 0117）
// - messages 表通过 conversation_id 间接关联，但加冗余 user_id 简化查询 + 防御深度
// - prompt_templates 表为公共模板，无用户隔离需求，保持原样

use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::ai_model::ConversationWithPreview;
use crate::models::chat::{Conversation, ConversationParticipant, Message};

pub async fn create_conversation(
    pool: &SqlitePool,
    user_id: i64,
    title: Option<&str>,
    r#type: &str,
    is_temp: bool,
    token_budget: i64,
    now: i64,
) -> Result<Conversation, AppError> {
    sqlx::query_as::<_, Conversation>(
        "INSERT INTO conversations (user_id, title, type, is_temp, token_budget, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(user_id)
    .bind(title)
    .bind(r#type)
    .bind(is_temp)
    .bind(token_budget)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_conversations(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<Conversation>, AppError> {
    sqlx::query_as::<_, Conversation>(
        "SELECT * FROM conversations WHERE user_id = ? ORDER BY updated_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_conversations_by_type(
    pool: &SqlitePool,
    user_id: i64,
    r#type: &str,
) -> Result<Vec<Conversation>, AppError> {
    sqlx::query_as::<_, Conversation>(
        "SELECT * FROM conversations WHERE user_id = ? AND type = ? ORDER BY updated_at DESC",
    )
    .bind(user_id)
    .bind(r#type)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

// spec ai-chat-enhancement Phase 2 §2.1: 会话列表联表查询（带最后消息预览）
//
// 与 get_conversations / get_conversations_by_type 的差异：
// - 返回 ConversationWithPreview（含 unread_count / sort_order / 最后消息预览三元组）
// - 排序改为 sort_order ASC + updated_at DESC（支持拖拽自定义排序）
// - 最后消息用相关子查询获取（兼容 SQLite 3.25 以下版本，无需窗口函数）
// - content 用 substr 截断 50 字符作为预览
// - 子查询中 m.user_id = ? 过滤确保不读取其他用户的消息（防御深度）
//
// 性能说明: 相关子查询每个会话执行 3 次 LIMIT 1 查询，对常见规模（< 100 会话）足够。
//           若后续会话量级增长，可迁移到 LEFT JOIN + 窗口函数方案。
pub async fn get_conversations_with_preview(
    pool: &SqlitePool,
    user_id: i64,
    conv_type: Option<&str>,
) -> Result<Vec<ConversationWithPreview>, AppError> {
    let query_str = if conv_type.is_some() {
        r#"SELECT c.id, c.user_id, c.title, c.type, c.is_temp, c.token_budget, c.starred,
                  c.created_at, c.updated_at, c.unread_count, c.sort_order,
                  (SELECT substr(m.content, 1, 50) FROM messages m
                   WHERE m.conversation_id = c.id AND m.user_id = c.user_id
                   ORDER BY m.id DESC LIMIT 1) AS last_message_preview,
                  (SELECT m.sender_type FROM messages m
                   WHERE m.conversation_id = c.id AND m.user_id = c.user_id
                   ORDER BY m.id DESC LIMIT 1) AS last_message_sender_type,
                  (SELECT m.created_at FROM messages m
                   WHERE m.conversation_id = c.id AND m.user_id = c.user_id
                   ORDER BY m.id DESC LIMIT 1) AS last_message_at
           FROM conversations c
           WHERE c.user_id = ? AND c.type = ?
           ORDER BY c.sort_order ASC, c.updated_at DESC"#
    } else {
        r#"SELECT c.id, c.user_id, c.title, c.type, c.is_temp, c.token_budget, c.starred,
                  c.created_at, c.updated_at, c.unread_count, c.sort_order,
                  (SELECT substr(m.content, 1, 50) FROM messages m
                   WHERE m.conversation_id = c.id AND m.user_id = c.user_id
                   ORDER BY m.id DESC LIMIT 1) AS last_message_preview,
                  (SELECT m.sender_type FROM messages m
                   WHERE m.conversation_id = c.id AND m.user_id = c.user_id
                   ORDER BY m.id DESC LIMIT 1) AS last_message_sender_type,
                  (SELECT m.created_at FROM messages m
                   WHERE m.conversation_id = c.id AND m.user_id = c.user_id
                   ORDER BY m.id DESC LIMIT 1) AS last_message_at
           FROM conversations c
           WHERE c.user_id = ?
           ORDER BY c.sort_order ASC, c.updated_at DESC"#
    };

    let query = sqlx::query_as::<_, ConversationWithPreview>(query_str);
    let rows = if let Some(t) = conv_type {
        query.bind(user_id).bind(t).fetch_all(pool).await.map_err(AppError::Database)?
    } else {
        query.bind(user_id).fetch_all(pool).await.map_err(AppError::Database)?
    };
    Ok(rows)
}

pub async fn update_conversation(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
    title: Option<&str>,
    token_budget: Option<i64>,
    now: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE conversations SET
            title = COALESCE(?, title),
            token_budget = COALESCE(?, token_budget),
            updated_at = ?
         WHERE id = ? AND user_id = ?",
    )
    .bind(title)
    .bind(token_budget)
    .bind(now)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn get_conversation_by_id(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
) -> Result<Option<Conversation>, AppError> {
    sqlx::query_as::<_, Conversation>(
        "SELECT * FROM conversations WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_conversation(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
) -> Result<(), AppError> {
    // ON DELETE CASCADE 会自动级联删除 messages 和 conversation_participants
    sqlx::query("DELETE FROM conversations WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn add_participant(
    pool: &SqlitePool,
    user_id: i64,
    conversation_id: i64,
    model_id: Option<i64>,
    agent_id: Option<i64>,
    role: &str,
) -> Result<ConversationParticipant, AppError> {
    sqlx::query_as::<_, ConversationParticipant>(
        "INSERT INTO conversation_participants (user_id, conversation_id, model_id, agent_id, role)
         VALUES (?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(user_id)
    .bind(conversation_id)
    .bind(model_id)
    .bind(agent_id)
    .bind(role)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_participants(
    pool: &SqlitePool,
    conversation_id: i64,
    user_id: i64,
) -> Result<Vec<ConversationParticipant>, AppError> {
    sqlx::query_as::<_, ConversationParticipant>(
        "SELECT * FROM conversation_participants WHERE conversation_id = ? AND user_id = ?",
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn add_message(
    pool: &SqlitePool,
    user_id: i64,
    conversation_id: i64,
    sender_type: &str,
    sender_id: Option<i64>,
    content: &str,
    round: i32,
    now: i64,
) -> Result<Message, AppError> {
    sqlx::query_as::<_, Message>(
        "INSERT INTO messages (user_id, conversation_id, sender_type, sender_id, content, round, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(user_id)
    .bind(conversation_id)
    .bind(sender_type)
    .bind(sender_id)
    .bind(content)
    .bind(round)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_messages(
    pool: &SqlitePool,
    conversation_id: i64,
    user_id: i64,
) -> Result<Vec<Message>, AppError> {
    sqlx::query_as::<_, Message>(
        "SELECT * FROM messages WHERE conversation_id = ? AND user_id = ? ORDER BY round ASC, created_at ASC",
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_max_round(
    pool: &SqlitePool,
    conversation_id: i64,
    user_id: i64,
) -> Result<i32, AppError> {
    let row: (Option<i32>,) = sqlx::query_as(
        "SELECT MAX(round) FROM messages WHERE conversation_id = ? AND user_id = ?",
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(row.0.unwrap_or(0))
}

pub async fn update_conversation_timestamp(
    pool: &SqlitePool,
    conversation_id: i64,
    user_id: i64,
    now: i64,
) -> Result<(), AppError> {
    sqlx::query("UPDATE conversations SET updated_at = ? WHERE id = ? AND user_id = ?")
        .bind(now)
        .bind(conversation_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

// spec ai-chat-enhancement Phase 2 §3.1: AI 回复 unread_count += 1
// 由 send_message 在 INSERT 'model' 消息后调用，前端打开会话时通过
// mark_conversation_read 清零。
pub async fn increment_unread_count(
    pool: &SqlitePool,
    conversation_id: i64,
    user_id: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE conversations SET unread_count = unread_count + 1 WHERE id = ? AND user_id = ?",
    )
    .bind(conversation_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

// spec ai-chat-enhancement Phase 2 §3.2: 标记会话已读（unread_count 清零）
// 前端打开会话时调用。
pub async fn mark_conversation_read(
    pool: &SqlitePool,
    conversation_id: i64,
    user_id: i64,
) -> Result<(), AppError> {
    sqlx::query("UPDATE conversations SET unread_count = 0 WHERE id = ? AND user_id = ?")
        .bind(conversation_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

// spec ai-chat-enhancement Phase 2 §3.3: 批量更新会话排序（拖拽自定义排序）
// 在事务内逐条 UPDATE，保证原子性。每条 UPDATE 带 user_id 过滤。
pub async fn reorder_conversations(
    pool: &SqlitePool,
    user_id: i64,
    items: &[crate::models::ai_model::ReorderItem],
) -> Result<(), AppError> {
    let mut tx = pool.begin().await.map_err(AppError::Database)?;
    for item in items {
        sqlx::query("UPDATE conversations SET sort_order = ? WHERE id = ? AND user_id = ?")
            .bind(item.sort_order)
            .bind(item.id)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
    }
    tx.commit().await.map_err(AppError::Database)?;
    Ok(())
}

pub async fn delete_message(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM messages WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

// ===== 搜索 =====

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct ConversationSearchResult {
    pub conversation_id: i64,
    pub title: Option<String>,
    pub matched_message_preview: String,
    pub updated_at: i64,
}

pub async fn search_conversations(
    pool: &SqlitePool,
    user_id: i64,
    query: &str,
) -> Result<Vec<ConversationSearchResult>, AppError> {
    let pattern = format!("%{}%", query);
    sqlx::query_as::<_, ConversationSearchResult>(
        "SELECT DISTINCT c.id as conversation_id, c.title,
            SUBSTR(m.content, 1, 100) as matched_message_preview,
            c.updated_at
         FROM conversations c
         LEFT JOIN messages m ON m.conversation_id = c.id AND m.user_id = c.user_id
         WHERE c.user_id = ? AND (c.title LIKE ? OR m.content LIKE ?)
         ORDER BY c.updated_at DESC
         LIMIT 50",
    )
    .bind(user_id)
    .bind(&pattern)
    .bind(&pattern)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

// ===== 星标 =====

pub async fn toggle_star_conversation(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
) -> Result<bool, AppError> {
    let row: (i64,) = sqlx::query_as(
        "UPDATE conversations SET starred = CASE WHEN starred = 1 THEN 0 ELSE 1 END
         WHERE id = ? AND user_id = ?
         RETURNING starred",
    )
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(row.0 == 1)
}

pub async fn get_starred_conversations(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<Conversation>, AppError> {
    sqlx::query_as::<_, Conversation>(
        "SELECT * FROM conversations WHERE user_id = ? AND starred = 1 ORDER BY updated_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

// ===== 分支 =====

pub async fn copy_messages_to_conversation(
    pool: &SqlitePool,
    user_id: i64,
    source_conv_id: i64,
    target_conv_id: i64,
    up_to_message_id: i64,
    _now: i64,
) -> Result<(), AppError> {
    // 复制时携带 user_id，确保分支后的消息仍属同一用户
    sqlx::query(
        "INSERT INTO messages (user_id, conversation_id, sender_type, sender_id, content, round, created_at)
         SELECT ?, ?, sender_type, sender_id, content, round, created_at
         FROM messages
         WHERE conversation_id = ? AND user_id = ? AND created_at <= (
            SELECT created_at FROM messages WHERE id = ? AND user_id = ?
         )
         ORDER BY round ASC, created_at ASC",
    )
    .bind(user_id)
    .bind(target_conv_id)
    .bind(source_conv_id)
    .bind(user_id)
    .bind(up_to_message_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

// ===== Prompt 模板 =====
//
// 设计说明：prompt_templates 表为公共模板（无 user_id 字段），所有用户共享。
// 如未来需要用户私有模板，可单独添加 user_id 字段并过滤。
// 当前保持原样，不在多用户隔离批次 2 范围内。

pub async fn get_prompt_templates(
    pool: &SqlitePool,
) -> Result<Vec<crate::models::chat::PromptTemplate>, AppError> {
    sqlx::query_as::<_, crate::models::chat::PromptTemplate>(
        "SELECT * FROM prompt_templates ORDER BY category, updated_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn create_prompt_template(
    pool: &SqlitePool,
    title: &str,
    category: &str,
    content: &str,
) -> Result<crate::models::chat::PromptTemplate, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query_as::<_, crate::models::chat::PromptTemplate>(
        "INSERT INTO prompt_templates (title, category, content, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(title)
    .bind(category)
    .bind(content)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn update_prompt_template(
    pool: &SqlitePool,
    id: i64,
    title: Option<&str>,
    category: Option<&str>,
    content: Option<&str>,
) -> Result<(), AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        "UPDATE prompt_templates SET
            title = COALESCE(?, title),
            category = COALESCE(?, category),
            content = COALESCE(?, content),
            updated_at = ?
         WHERE id = ?",
    )
    .bind(title)
    .bind(category)
    .bind(content)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn delete_prompt_template(pool: &SqlitePool, id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM prompt_templates WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}
