use sqlx::{Row, SqlitePool};

use crate::db::repositories::recycle_repo;
use crate::error::app_error::AppError;
use crate::models::recycle::RecycleBinItem;

pub async fn get_items(pool: &SqlitePool, user_id: i64) -> Result<Vec<RecycleBinItem>, AppError> {
    recycle_repo::get_items(pool, user_id).await
}

pub async fn move_to_recycle(
    pool: &SqlitePool,
    user_id: i64,
    item_type: &str,
    item_ids: &[i64],
    deleted_by: Option<i64>,
) -> Result<(), AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let auto_delete_at = now + 30 * 24 * 3600 * 1000;

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    for &id in item_ids {
        let (title, metadata_json, file_size) =
            fetch_source_row(&mut *tx, item_type, id, user_id).await?;

        let original_path = format!("{}/{}", item_type, id);

        recycle_repo::add_item(
            &mut *tx,
            user_id,
            &original_path,
            item_type,
            Some(id),
            title.as_deref(),
            metadata_json.as_deref(),
            file_size,
            deleted_by,
            now,
            auto_delete_at,
        )
        .await?;

        delete_from_source(&mut *tx, item_type, id, user_id).await?;
    }

    tx.commit().await.map_err(AppError::Database)?;

    tracing::info!(
        item_type = item_type,
        count = item_ids.len(),
        "批量移入回收站"
    );

    Ok(())
}

pub async fn restore_items(pool: &SqlitePool, user_id: i64, ids: &[i64]) -> Result<u64, AppError> {
    let mut restored = 0u64;
    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    for &id in ids {
        let item = recycle_repo::get_item_by_id(&mut *tx, id, user_id).await?;

        if let Some(ref metadata_json) = item.metadata_json {
            restore_to_source(&mut *tx, &item.item_type, metadata_json, user_id).await?;
        }

        recycle_repo::delete_item(&mut *tx, id, user_id).await?;
        restored += 1;
    }

    tx.commit().await.map_err(AppError::Database)?;

    tracing::info!(count = restored, "批量还原回收站项目");

    Ok(restored)
}

pub async fn delete_permanently_batch(
    pool: &SqlitePool,
    user_id: i64,
    ids: &[i64],
) -> Result<u64, AppError> {
    let count = recycle_repo::delete_items_batch(pool, user_id, ids).await?;
    tracing::info!(count = count, "批量彻底删除回收站项目");
    Ok(count)
}

pub async fn cleanup_expired(pool: &SqlitePool) -> Result<u64, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let count = recycle_repo::cleanup_expired(pool, now).await?;
    if count > 0 {
        tracing::info!(count = count, "已清理过期回收站项目");
    }
    Ok(count)
}

pub async fn clear_all(pool: &SqlitePool, user_id: i64) -> Result<(), AppError> {
    recycle_repo::clear_all(pool, user_id).await?;
    tracing::info!(user_id = user_id, "回收站已清空（当前用户）");
    Ok(())
}

pub async fn get_stats(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<crate::models::recycle::RecycleStats, AppError> {
    recycle_repo::get_stats(pool, user_id).await
}

async fn fetch_source_row(
    executor: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    item_type: &str,
    id: i64,
    user_id: i64,
) -> Result<(Option<String>, Option<String>, Option<i64>), AppError> {
    match item_type {
        "todo" => {
            // 多用户隔离补全：todos 已有 user_id 字段（migration 22 后某次 ALTER），按 user_id 过滤
            let row = sqlx::query("SELECT title, description, priority, due_date, date, completed, created_at, updated_at FROM todos WHERE id = ? AND user_id = ?")
                .bind(id)
                .bind(user_id)
                .fetch_optional(executor)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::NotFound)?;

            let title: String = row.get("title");
            let metadata = serde_json::json!({
                "title": row.get::<String, _>("title"),
                "description": row.get::<Option<String>, _>("description"),
                "priority": row.get::<String, _>("priority"),
                "due_date": row.get::<Option<String>, _>("due_date"),
                "date": row.get::<String, _>("date"),
                "completed": row.get::<i32, _>("completed"),
                "created_at": row.get::<i64, _>("created_at"),
                "updated_at": row.get::<i64, _>("updated_at"),
            });

            Ok((Some(title), Some(metadata.to_string()), None))
        }
        "journal" => {
            // 多用户隔离批次 5：journals 已加 user_id（migration 0120），按 user_id 过滤
            let row = sqlx::query("SELECT date, content, created_at, updated_at FROM journals WHERE id = ? AND user_id = ?")
                .bind(id)
                .bind(user_id)
                .fetch_optional(executor)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::NotFound)?;

            let date: String = row.get("date");
            let metadata = serde_json::json!({
                "date": row.get::<String, _>("date"),
                "content": row.get::<Option<String>, _>("content"),
                "created_at": row.get::<i64, _>("created_at"),
                "updated_at": row.get::<i64, _>("updated_at"),
            });

            Ok((Some(format!("日志 {}", date)), Some(metadata.to_string()), None))
        }
        "kb_entry" => {
            // 多用户隔离批次 3 补全：kb_entries 已加 user_id（migration 0118），按 user_id 过滤
            let row = sqlx::query("SELECT category_id, name, path_url, entry_type, created_at, updated_at FROM kb_entries WHERE id = ? AND user_id = ?")
                .bind(id)
                .bind(user_id)
                .fetch_optional(executor)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::NotFound)?;

            let name: String = row.get("name");
            let metadata = serde_json::json!({
                "category_id": row.get::<i64, _>("category_id"),
                "name": row.get::<String, _>("name"),
                "path_url": row.get::<String, _>("path_url"),
                "entry_type": row.get::<String, _>("entry_type"),
                "created_at": row.get::<i64, _>("created_at"),
                "updated_at": row.get::<i64, _>("updated_at"),
            });

            Ok((Some(name), Some(metadata.to_string()), None))
        }
        "conversation" => {
            // 多用户隔离批次 2：按 user_id 过滤，防止跨用户读取会话元数据
            let row = sqlx::query("SELECT title, type, is_temp, dissolve_at, token_budget, created_at, updated_at FROM conversations WHERE id = ? AND user_id = ?")
                .bind(id)
                .bind(user_id)
                .fetch_optional(executor)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::NotFound)?;

            let title: Option<String> = row.get("title");
            let display_title = title.clone().unwrap_or_else(|| format!("会话 #{}", id));
            let metadata = serde_json::json!({
                "title": title,
                "type": row.get::<String, _>("type"),
                "is_temp": row.get::<i32, _>("is_temp"),
                "dissolve_at": row.get::<Option<i64>, _>("dissolve_at"),
                "token_budget": row.get::<i64, _>("token_budget"),
                "created_at": row.get::<i64, _>("created_at"),
                "updated_at": row.get::<i64, _>("updated_at"),
                "user_id": user_id,
            });

            Ok((Some(display_title), Some(metadata.to_string()), None))
        }
        "timer" => {
            // 多用户隔离批次 5：timers 已加 user_id（migration 0120），按 user_id 过滤
            let row = sqlx::query("SELECT name, type, target_time, is_running, elapsed, created_at, updated_at FROM timers WHERE id = ? AND user_id = ?")
                .bind(id)
                .bind(user_id)
                .fetch_optional(executor)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::NotFound)?;

            let name: Option<String> = row.get("name");
            let display_name = name.clone().unwrap_or_else(|| format!("计时器 #{}", id));
            let metadata = serde_json::json!({
                "name": name,
                "type": row.get::<String, _>("type"),
                "target_time": row.get::<Option<i64>, _>("target_time"),
                "is_running": row.get::<i32, _>("is_running"),
                "elapsed": row.get::<i64, _>("elapsed"),
                "created_at": row.get::<i64, _>("created_at"),
                "updated_at": row.get::<i64, _>("updated_at"),
            });

            Ok((Some(display_name), Some(metadata.to_string()), None))
        }
        "kb_category" => {
            // 多用户隔离批次 3 补全：kb_categories 已加 user_id（migration 0118），按 user_id 过滤
            let row = sqlx::query("SELECT name, parent_id, sort_order, created_at FROM kb_categories WHERE id = ? AND user_id = ?")
                .bind(id)
                .bind(user_id)
                .fetch_optional(executor)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::NotFound)?;

            let name: String = row.get("name");
            let metadata = serde_json::json!({
                "name": row.get::<String, _>("name"),
                "parent_id": row.get::<Option<i64>, _>("parent_id"),
                "sort_order": row.get::<i32, _>("sort_order"),
                "created_at": row.get::<i64, _>("created_at"),
            });

            Ok((Some(name), Some(metadata.to_string()), None))
        }
        _ => Err(AppError::Validation(format!(
            "不支持的项目类型: {}",
            item_type
        ))),
    }
}

async fn delete_from_source(
    executor: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    item_type: &str,
    id: i64,
    user_id: i64,
) -> Result<(), AppError> {
    // 多用户隔离补全：对已加 user_id 字段的表（todos/kb_entries/kb_categories/conversations/journals/timers）
    // 一律按 user_id 过滤，防止跨用户删除
    match item_type {
        "conversation" => {
            // ON DELETE CASCADE 会级联删除 messages 和 conversation_participants
            sqlx::query("DELETE FROM conversations WHERE id = ? AND user_id = ?")
                .bind(id)
                .bind(user_id)
                .execute(executor)
                .await
                .map_err(AppError::Database)?;
        }
        "todo" | "kb_entry" | "kb_category" | "journal" | "timer" => {
            // 多用户隔离批次 5：journals/timers 已加 user_id（migration 0120），统一按 user_id 过滤
            let table = source_table(item_type)?;
            let sql = format!("DELETE FROM {} WHERE id = ? AND user_id = ?", table);
            sqlx::query(&sql)
                .bind(id)
                .bind(user_id)
                .execute(executor)
                .await
                .map_err(AppError::Database)?;
        }
        _ => {
            // 其他暂未加 user_id 字段的表（如有）
            let table = source_table(item_type)?;
            let sql = format!("DELETE FROM {} WHERE id = ?", table);
            sqlx::query(&sql)
                .bind(id)
                .execute(executor)
                .await
                .map_err(AppError::Database)?;
        }
    }
    Ok(())
}

async fn restore_to_source(
    executor: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    item_type: &str,
    metadata_json: &str,
    user_id: i64,
) -> Result<(), AppError> {
    let data: serde_json::Value =
        serde_json::from_str(metadata_json).map_err(|e| AppError::Internal(e.to_string()))?;

    let now = chrono::Utc::now().timestamp_millis();

    match item_type {
        "todo" => {
            // 多用户隔离补全：todos 已有 user_id 字段，恢复时写入原 user_id 防止归属错乱
            sqlx::query(
                "INSERT INTO todos (user_id, title, description, priority, due_date, date, completed, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(user_id)
            .bind(data["title"].as_str().unwrap_or(""))
            .bind(data["description"].as_str())
            .bind(data["priority"].as_str().unwrap_or("medium"))
            .bind(data["due_date"].as_str())
            .bind(data["date"].as_str().unwrap_or(""))
            .bind(data["completed"].as_i64().unwrap_or(0) as i32)
            .bind(data["created_at"].as_i64().unwrap_or(now))
            .bind(now)
            .execute(executor)
            .await
            .map_err(AppError::Database)?;
        }
        "journal" => {
            // 多用户隔离批次 5：journals 已加 user_id（migration 0120），恢复时写入原 user_id
            sqlx::query(
                "INSERT INTO journals (user_id, date, content, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(user_id)
            .bind(data["date"].as_str().unwrap_or(""))
            .bind(data["content"].as_str())
            .bind(data["created_at"].as_i64().unwrap_or(now))
            .bind(now)
            .execute(executor)
            .await
            .map_err(AppError::Database)?;
        }
        "kb_entry" => {
            // 多用户隔离批次 3 补全：kb_entries 已加 user_id（migration 0118），恢复时写入原 user_id
            sqlx::query(
                "INSERT INTO kb_entries (user_id, category_id, name, path_url, entry_type, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(user_id)
            .bind(data["category_id"].as_i64().unwrap_or(0))
            .bind(data["name"].as_str().unwrap_or(""))
            .bind(data["path_url"].as_str().unwrap_or(""))
            .bind(data["entry_type"].as_str().unwrap_or("file"))
            .bind(data["created_at"].as_i64().unwrap_or(now))
            .bind(now)
            .execute(executor)
            .await
            .map_err(AppError::Database)?;
        }
        "conversation" => {
            // 多用户隔离批次 2：恢复时携带 user_id，确保还原后会话仍属原用户
            sqlx::query(
                "INSERT INTO conversations (user_id, title, type, is_temp, dissolve_at, token_budget, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(user_id)
            .bind(data["title"].as_str())
            .bind(data["type"].as_str().unwrap_or("single"))
            .bind(data["is_temp"].as_i64().unwrap_or(0) as i32)
            .bind(data["dissolve_at"].as_i64())
            .bind(data["token_budget"].as_i64().unwrap_or(50000))
            .bind(data["created_at"].as_i64().unwrap_or(now))
            .bind(now)
            .execute(executor)
            .await
            .map_err(AppError::Database)?;
        }
        "timer" => {
            // 多用户隔离批次 5：timers 已加 user_id（migration 0120），恢复时写入原 user_id
            sqlx::query(
                "INSERT INTO timers (user_id, name, type, target_time, is_running, elapsed, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(user_id)
            .bind(data["name"].as_str())
            .bind(data["type"].as_str().unwrap_or("countdown"))
            .bind(data["target_time"].as_i64())
            .bind(data["is_running"].as_i64().unwrap_or(0) as i32)
            .bind(data["elapsed"].as_i64().unwrap_or(0))
            .bind(data["created_at"].as_i64().unwrap_or(now))
            .bind(now)
            .execute(executor)
            .await
            .map_err(AppError::Database)?;
        }
        "kb_category" => {
            // 多用户隔离批次 3 补全：kb_categories 已加 user_id（migration 0118），恢复时写入原 user_id
            sqlx::query(
                "INSERT INTO kb_categories (user_id, name, parent_id, sort_order, created_at)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(user_id)
            .bind(data["name"].as_str().unwrap_or(""))
            .bind(data["parent_id"].as_i64())
            .bind(data["sort_order"].as_i64().unwrap_or(0) as i32)
            .bind(data["created_at"].as_i64().unwrap_or(now))
            .execute(executor)
            .await
            .map_err(AppError::Database)?;
        }
        _ => {
            return Err(AppError::Validation(format!(
                "不支持还原的项目类型: {}",
                item_type
            )))
        }
    }

    Ok(())
}

fn source_table(item_type: &str) -> Result<&str, AppError> {
    match item_type {
        "todo" => Ok("todos"),
        "journal" => Ok("journals"),
        "kb_entry" => Ok("kb_entries"),
        "kb_category" => Ok("kb_categories"),
        "conversation" => Ok("conversations"),
        "timer" => Ok("timers"),
        _ => Err(AppError::Validation(format!(
            "不支持的项目类型: {}",
            item_type
        ))),
    }
}