use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::services::xin_context_service::ContextWindow;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ConversationCheckpoint {
    pub id: String,
    pub conversation_id: String,
    pub checkpoint_type: String,
    pub context_json: String,
    pub partial_response: Option<String>,
    pub message_count: i64,
    pub total_tokens: i64,
    pub title: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSummary {
    pub id: String,
    pub conversation_id: String,
    pub checkpoint_type: String,
    pub message_count: i64,
    pub total_tokens: i64,
    pub title: Option<String>,
    pub has_partial_response: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    pub checkpoint: CheckpointSummary,
    pub context: ContextWindow,
    pub partial_response: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointConfig {
    pub auto_save_interval_seconds: u64,
    pub max_checkpoints_per_conversation: usize,
    pub max_age_hours: u64,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            auto_save_interval_seconds: 60,
            max_checkpoints_per_conversation: 20,
            max_age_hours: 168,
        }
    }
}

pub struct XinCheckpointService;

impl XinCheckpointService {
    pub async fn create_table(pool: &SqlitePool) -> Result<(), AppError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS xin_checkpoints (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                user_id INTEGER NOT NULL DEFAULT 1,
                checkpoint_type TEXT NOT NULL DEFAULT 'auto',
                context_json TEXT NOT NULL,
                partial_response TEXT,
                message_count INTEGER NOT NULL DEFAULT 0,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                title TEXT,
                created_at TEXT NOT NULL
            );",
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_xin_checkpoints_user_conv ON xin_checkpoints(user_id, conversation_id, created_at DESC);",
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    pub async fn save_checkpoint(
        pool: &SqlitePool,
        user_id: i64,
        conversation_id: &str,
        context: &ContextWindow,
        partial_response: Option<&str>,
        title: Option<&str>,
        msg_count: i64,
        total_tokens: i64,
    ) -> Result<ConversationCheckpoint, AppError> {
        let id = format!("ckpt_{}", uuid::Uuid::new_v4());
        let now = chrono::Utc::now().to_rfc3339();
        let context_json = serde_json::to_string(context).unwrap_or_default();
        let checkpoint_type = if partial_response.is_some() { "interrupted" } else { "auto" };

        let checkpoint = ConversationCheckpoint {
            id: id.clone(),
            conversation_id: conversation_id.to_string(),
            checkpoint_type: checkpoint_type.to_string(),
            context_json,
            partial_response: partial_response.map(|s| s.to_string()),
            message_count: msg_count,
            total_tokens,
            title: title.map(|s| s.to_string()),
            created_at: now.clone(),
        };

        sqlx::query(
            "INSERT INTO xin_checkpoints (id, conversation_id, user_id, checkpoint_type, context_json, partial_response, message_count, total_tokens, title, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&checkpoint.id)
        .bind(&checkpoint.conversation_id)
        .bind(user_id)
        .bind(&checkpoint.checkpoint_type)
        .bind(&checkpoint.context_json)
        .bind(&checkpoint.partial_response)
        .bind(checkpoint.message_count)
        .bind(checkpoint.total_tokens)
        .bind(&checkpoint.title)
        .bind(&checkpoint.created_at)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(checkpoint)
    }

    pub async fn save_manual_checkpoint(
        pool: &SqlitePool,
        user_id: i64,
        conversation_id: &str,
        context: &ContextWindow,
        title: Option<&str>,
        msg_count: i64,
        total_tokens: i64,
    ) -> Result<ConversationCheckpoint, AppError> {
        let id = format!("ckpt_{}", uuid::Uuid::new_v4());
        let now = chrono::Utc::now().to_rfc3339();
        let context_json = serde_json::to_string(context).unwrap_or_default();

        let checkpoint = ConversationCheckpoint {
            id: id.clone(),
            conversation_id: conversation_id.to_string(),
            checkpoint_type: "manual".to_string(),
            context_json,
            partial_response: None,
            message_count: msg_count,
            total_tokens,
            title: title.map(|s| s.to_string()),
            created_at: now.clone(),
        };

        sqlx::query(
            "INSERT INTO xin_checkpoints (id, conversation_id, user_id, checkpoint_type, context_json, partial_response, message_count, total_tokens, title, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&checkpoint.id)
        .bind(&checkpoint.conversation_id)
        .bind(user_id)
        .bind(&checkpoint.checkpoint_type)
        .bind(&checkpoint.context_json)
        .bind(&checkpoint.partial_response)
        .bind(checkpoint.message_count)
        .bind(checkpoint.total_tokens)
        .bind(&checkpoint.title)
        .bind(&checkpoint.created_at)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(checkpoint)
    }

    pub async fn restore_checkpoint(
        pool: &SqlitePool,
        user_id: i64,
        checkpoint_id: &str,
    ) -> Result<RestoreResult, AppError> {
        let checkpoint = sqlx::query_as::<_, ConversationCheckpoint>(
            "SELECT id, conversation_id, checkpoint_type, context_json, partial_response, message_count, total_tokens, title, created_at
             FROM xin_checkpoints WHERE user_id = ? AND id = ?",
        )
        .bind(user_id)
        .bind(checkpoint_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound)?;

        let context: ContextWindow = serde_json::from_str(&checkpoint.context_json)
            .map_err(|e| AppError::AiApi(format!("检查点上下文反序列化失败: {}", e)))?;

        Ok(RestoreResult {
            checkpoint: CheckpointSummary {
                id: checkpoint.id,
                conversation_id: checkpoint.conversation_id,
                checkpoint_type: checkpoint.checkpoint_type,
                message_count: checkpoint.message_count,
                total_tokens: checkpoint.total_tokens,
                title: checkpoint.title,
                has_partial_response: checkpoint.partial_response.is_some(),
                created_at: checkpoint.created_at,
            },
            context,
            partial_response: checkpoint.partial_response,
        })
    }

    pub async fn get_latest_checkpoint(
        pool: &SqlitePool,
        user_id: i64,
        conversation_id: &str,
    ) -> Result<Option<RestoreResult>, AppError> {
        let checkpoint = sqlx::query_as::<_, ConversationCheckpoint>(
            "SELECT id, conversation_id, checkpoint_type, context_json, partial_response, message_count, total_tokens, title, created_at
             FROM xin_checkpoints WHERE user_id = ? AND conversation_id = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(user_id)
        .bind(conversation_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        match checkpoint {
            Some(cp) => {
                let context: ContextWindow = serde_json::from_str(&cp.context_json)
                    .map_err(|e| AppError::AiApi(format!("检查点上下文反序列化失败: {}", e)))?;

                Ok(Some(RestoreResult {
                    checkpoint: CheckpointSummary {
                        id: cp.id,
                        conversation_id: cp.conversation_id,
                        checkpoint_type: cp.checkpoint_type,
                        message_count: cp.message_count,
                        total_tokens: cp.total_tokens,
                        title: cp.title,
                        has_partial_response: cp.partial_response.is_some(),
                        created_at: cp.created_at,
                    },
                    context,
                    partial_response: cp.partial_response,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn list_checkpoints(
        pool: &SqlitePool,
        user_id: i64,
        conversation_id: &str,
    ) -> Result<Vec<CheckpointSummary>, AppError> {
        let checkpoints = sqlx::query_as::<_, ConversationCheckpoint>(
            "SELECT id, conversation_id, checkpoint_type, context_json, partial_response, message_count, total_tokens, title, created_at
             FROM xin_checkpoints WHERE user_id = ? AND conversation_id = ? ORDER BY created_at DESC",
        )
        .bind(user_id)
        .bind(conversation_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(checkpoints
            .into_iter()
            .map(|cp| CheckpointSummary {
                id: cp.id,
                conversation_id: cp.conversation_id,
                checkpoint_type: cp.checkpoint_type,
                message_count: cp.message_count,
                total_tokens: cp.total_tokens,
                title: cp.title,
                has_partial_response: cp.partial_response.is_some(),
                created_at: cp.created_at,
            })
            .collect())
    }

    pub async fn delete_checkpoint(
        pool: &SqlitePool,
        user_id: i64,
        checkpoint_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM xin_checkpoints WHERE user_id = ? AND id = ?")
            .bind(user_id)
            .bind(checkpoint_id)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn delete_conversation_checkpoints(
        pool: &SqlitePool,
        user_id: i64,
        conversation_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM xin_checkpoints WHERE user_id = ? AND conversation_id = ?")
            .bind(user_id)
            .bind(conversation_id)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn cleanup_old_checkpoints(
        pool: &SqlitePool,
        user_id: i64,
        config: &CheckpointConfig,
    ) -> Result<usize, AppError> {
        let cutoff = chrono::Utc::now()
            - chrono::Duration::hours(config.max_age_hours as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let deleted = sqlx::query(
            "DELETE FROM xin_checkpoints WHERE user_id = ? AND created_at < ? AND checkpoint_type != 'manual'",
        )
        .bind(user_id)
        .bind(&cutoff_str)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(deleted.rows_affected() as usize)
    }

    pub async fn enforce_limit(
        pool: &SqlitePool,
        user_id: i64,
        conversation_id: &str,
        config: &CheckpointConfig,
    ) -> Result<(), AppError> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM xin_checkpoints WHERE user_id = ? AND conversation_id = ?",
        )
        .bind(user_id)
        .bind(conversation_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

        if count.0 as usize <= config.max_checkpoints_per_conversation {
            return Ok(());
        }

        let excess = count.0 as usize - config.max_checkpoints_per_conversation;
        sqlx::query(
            "DELETE FROM xin_checkpoints WHERE user_id = ? AND id IN (
                SELECT id FROM xin_checkpoints WHERE user_id = ? AND conversation_id = ? AND checkpoint_type = 'auto'
                ORDER BY created_at ASC LIMIT ?
            )",
        )
        .bind(user_id)
        .bind(user_id)
        .bind(conversation_id)
        .bind(excess as i64)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::xin_context_service::{ChatMessage, ChatRole};

    #[tokio::test]
    async fn test_save_and_restore_checkpoint() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        XinCheckpointService::create_table(&pool).await.unwrap();

        let ctx = ContextWindow {
            messages: vec![
                ChatMessage {
                    role: ChatRole::User,
                    content: "你好".into(),
                    name: None,
                    timestamp: None,
                },
                ChatMessage {
                    role: ChatRole::Assistant,
                    content: "你好！有什么可以帮你的？".into(),
                    name: None,
                    timestamp: None,
                },
            ],
            model_id: "gpt-4".into(),
            max_tokens: 8192,
            used_tokens: 50,
            summary: None,
        };

        let saved = XinCheckpointService::save_checkpoint(
            &pool,
            1,
            "conv-test-1",
            &ctx,
            None,
            Some("测试对话"),
            2,
            50,
        )
        .await
        .unwrap();

        assert_eq!(saved.checkpoint_type, "auto");
        assert_eq!(saved.message_count, 2);

        let restored = XinCheckpointService::restore_checkpoint(&pool, 1, &saved.id)
            .await
            .unwrap();
        assert_eq!(restored.context.messages.len(), 2);
        assert_eq!(restored.partial_response, None);

        let list = XinCheckpointService::list_checkpoints(&pool, 1, "conv-test-1")
            .await
            .unwrap();
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn test_interrupted_checkpoint() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        XinCheckpointService::create_table(&pool).await.unwrap();

        let ctx = ContextWindow {
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "测试".into(),
                name: None,
                timestamp: None,
            }],
            model_id: "gpt-4".into(),
            max_tokens: 8192,
            used_tokens: 10,
            summary: None,
        };

        let saved = XinCheckpointService::save_checkpoint(
            &pool,
            1,
            "conv-test-2",
            &ctx,
            Some("回应内容的前半段..."),
            None,
            1,
            10,
        )
        .await
        .unwrap();

        assert_eq!(saved.checkpoint_type, "interrupted");

        let restored = XinCheckpointService::restore_checkpoint(&pool, 1, &saved.id)
            .await
            .unwrap();
        assert!(restored.partial_response.is_some());
    }

    #[tokio::test]
    async fn test_manual_checkpoint() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        XinCheckpointService::create_table(&pool).await.unwrap();

        let ctx = ContextWindow {
            messages: vec![],
            model_id: "claude-3".into(),
            max_tokens: 200000,
            used_tokens: 0,
            summary: None,
        };

        let saved = XinCheckpointService::save_manual_checkpoint(
            &pool,
            1,
            "conv-manual",
            &ctx,
            Some("手动快照"),
            0,
            0,
        )
        .await
        .unwrap();

        assert_eq!(saved.checkpoint_type, "manual");

        let latest = XinCheckpointService::get_latest_checkpoint(&pool, 1, "conv-manual")
            .await
            .unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().context.model_id, "claude-3");
    }

    #[tokio::test]
    async fn test_cleanup_old() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        XinCheckpointService::create_table(&pool).await.unwrap();

        let ctx = ContextWindow {
            messages: vec![],
            model_id: "gpt-4".into(),
            max_tokens: 8192,
            used_tokens: 0,
            summary: None,
        };

        let config = CheckpointConfig {
            max_age_hours: 0,
            ..Default::default()
        };

        XinCheckpointService::save_checkpoint(
            &pool, 1, "conv-clean", &ctx, None, None, 0, 0,
        )
        .await
        .unwrap();

        let deleted = XinCheckpointService::cleanup_old_checkpoints(&pool, 1, &config)
            .await
            .unwrap();
        assert!(deleted >= 1);
    }

    #[tokio::test]
    async fn test_delete_checkpoint() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        XinCheckpointService::create_table(&pool).await.unwrap();

        let ctx = ContextWindow {
            messages: vec![],
            model_id: "gpt-4".into(),
            max_tokens: 8192,
            used_tokens: 0,
            summary: None,
        };

        let saved = XinCheckpointService::save_checkpoint(
            &pool, 1, "conv-del", &ctx, None, None, 0, 0,
        )
        .await
        .unwrap();

        XinCheckpointService::delete_checkpoint(&pool, 1, &saved.id)
            .await
            .unwrap();

        let list = XinCheckpointService::list_checkpoints(&pool, 1, "conv-del")
            .await
            .unwrap();
        assert_eq!(list.len(), 0);
    }

    #[tokio::test]
    async fn test_enforce_limit() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        XinCheckpointService::create_table(&pool).await.unwrap();

        let ctx = ContextWindow {
            messages: vec![],
            model_id: "gpt-4".into(),
            max_tokens: 8192,
            used_tokens: 0,
            summary: None,
        };

        let config = CheckpointConfig {
            max_checkpoints_per_conversation: 2,
            ..Default::default()
        };

        for i in 0..5 {
            XinCheckpointService::save_checkpoint(
                &pool, 1, "conv-limit", &ctx, None, Some(&format!("第{}轮", i)), i, 0,
            )
            .await
            .unwrap();
        }

        XinCheckpointService::enforce_limit(&pool, 1, "conv-limit", &config)
            .await
            .unwrap();

        let list = XinCheckpointService::list_checkpoints(&pool, 1, "conv-limit")
            .await
            .unwrap();
        assert!(list.len() <= 2);
    }
}