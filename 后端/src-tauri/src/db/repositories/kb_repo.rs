// 多用户数据隔离批次 3：所有 KB 表操作均按 user_id 过滤
// 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
// 说明: kb_categories / kb_entries / kb_tags / kb_entry_tags / kb_recent_access /
//       kb_tracked_paths / kb_references / kb_snapshots / kb_templates 均已添加 user_id 字段。
//       所有 INSERT 携带 user_id，所有 SELECT/UPDATE/DELETE 携带 WHERE user_id = ? 过滤。

use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::knowledge::{KbCategory, KbEntry, KbTag, KbTrackedPath, KbTemplate, KbSnapshot, EntryTagsResult, TagStats};

pub async fn get_categories(pool: &SqlitePool, user_id: i64, library: Option<&str>) -> Result<Vec<KbCategory>, AppError> {
    if let Some(lib) = library {
        sqlx::query_as::<_, KbCategory>("SELECT * FROM kb_categories WHERE user_id = ? AND library = ? ORDER BY sort_order ASC")
            .bind(user_id)
            .bind(lib)
            .fetch_all(pool)
            .await
            .map_err(AppError::Database)
    } else {
        sqlx::query_as::<_, KbCategory>("SELECT * FROM kb_categories WHERE user_id = ? ORDER BY sort_order ASC")
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(AppError::Database)
    }
}

pub async fn add_category(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    parent_id: Option<i64>,
    library: &str,
    sort_order: i32,
    now: i64,
) -> Result<KbCategory, AppError> {
    sqlx::query_as::<_, KbCategory>(
        "INSERT INTO kb_categories (user_id, name, parent_id, library, sort_order, created_at)
         VALUES (?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(user_id)
    .bind(name)
    .bind(parent_id)
    .bind(library)
    .bind(sort_order)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_category(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM kb_categories WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn create_snapshot(pool: &SqlitePool, user_id: i64, entry_id: i64, content: &str, now: i64) -> Result<KbSnapshot, AppError> {
    let snapshot: KbSnapshot = sqlx::query_as::<_, KbSnapshot>(
        "INSERT INTO kb_snapshots (user_id, entry_id, content, created_at) VALUES (?, ?, ?, ?) RETURNING *",
    )
    .bind(user_id)
    .bind(entry_id)
    .bind(content)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM kb_snapshots WHERE entry_id = ? AND user_id = ?")
        .bind(entry_id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;
    if count.0 > 50 {
        sqlx::query(
            "DELETE FROM kb_snapshots WHERE id IN (
                SELECT id FROM kb_snapshots WHERE entry_id = ? AND user_id = ? ORDER BY created_at ASC LIMIT ?
            )",
        )
        .bind(entry_id)
        .bind(user_id)
        .bind(count.0 - 50)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    }

    Ok(snapshot)
}

pub async fn get_snapshots(pool: &SqlitePool, user_id: i64, entry_id: i64) -> Result<Vec<KbSnapshot>, AppError> {
    sqlx::query_as::<_, KbSnapshot>(
        "SELECT * FROM kb_snapshots WHERE entry_id = ? AND user_id = ? ORDER BY created_at DESC",
    )
    .bind(entry_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_snapshot(pool: &SqlitePool, user_id: i64, snapshot_id: i64) -> Result<Option<KbSnapshot>, AppError> {
    sqlx::query_as::<_, KbSnapshot>("SELECT * FROM kb_snapshots WHERE id = ? AND user_id = ?")
        .bind(snapshot_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn delete_snapshots_by_entry(pool: &SqlitePool, user_id: i64, entry_id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM kb_snapshots WHERE entry_id = ? AND user_id = ?")
        .bind(entry_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn rebuild_references(pool: &SqlitePool, user_id: i64, source_entry_id: i64, target_names: &[String]) -> Result<(), AppError> {
    sqlx::query("DELETE FROM kb_references WHERE source_entry_id = ? AND user_id = ?")
        .bind(source_entry_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    if target_names.is_empty() {
        return Ok(());
    }
    let now = chrono::Utc::now().timestamp_millis();
    for name in target_names {
        let target: Option<KbEntry> = sqlx::query_as::<_, KbEntry>(
            "SELECT * FROM kb_entries WHERE name = ? AND user_id = ? LIMIT 1",
        )
        .bind(name)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;
        if let Some(t) = target {
            if t.id != source_entry_id {
                sqlx::query(
                    "INSERT OR IGNORE INTO kb_references (user_id, source_entry_id, target_entry_id, created_at) VALUES (?, ?, ?, ?)",
                )
                .bind(user_id)
                .bind(source_entry_id)
                .bind(t.id)
                .bind(now)
                .execute(pool)
                .await
                .map_err(AppError::Database)?;
            }
        }
    }
    Ok(())
}

pub async fn get_backlinks(pool: &SqlitePool, user_id: i64, entry_id: i64) -> Result<Vec<(KbEntry, String)>, AppError> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT source_entry_id, target_entry_id FROM kb_references WHERE target_entry_id = ? AND user_id = ? ORDER BY created_at DESC",
    )
    .bind(entry_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;
    let mut results = Vec::new();
    for (source_id, _) in rows {
        if let Ok(source) = sqlx::query_as::<_, KbEntry>("SELECT * FROM kb_entries WHERE id = ? AND user_id = ?")
            .bind(source_id)
            .bind(user_id)
            .fetch_one(pool)
            .await
        {
            let snippet = source
                .content
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(200)
                .collect::<String>();
            results.push((source, snippet));
        }
    }
    Ok(results)
}

pub async fn get_outgoing_links(pool: &SqlitePool, user_id: i64, entry_id: i64) -> Result<Vec<KbEntry>, AppError> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT target_entry_id FROM kb_references WHERE source_entry_id = ? AND user_id = ? ORDER BY created_at DESC",
    )
    .bind(entry_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;
    let mut results = Vec::new();
    for (target_id,) in rows {
        if let Ok(entry) = sqlx::query_as::<_, KbEntry>("SELECT * FROM kb_entries WHERE id = ? AND user_id = ?")
            .bind(target_id)
            .bind(user_id)
            .fetch_one(pool)
            .await
        {
            results.push(entry);
        }
    }
    Ok(results)
}

pub async fn update_category(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    name: &str,
) -> Result<KbCategory, AppError> {
    sqlx::query_as::<_, KbCategory>(
        "UPDATE kb_categories SET name = ? WHERE id = ? AND user_id = ? RETURNING *",
    )
    .bind(name)
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_entries_by_category(
    pool: &SqlitePool,
    user_id: i64,
    category_id: i64,
) -> Result<Vec<KbEntry>, AppError> {
    sqlx::query_as::<_, KbEntry>("SELECT * FROM kb_entries WHERE category_id = ? AND user_id = ? ORDER BY created_at DESC")
        .bind(category_id)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn add_entry(
    pool: &SqlitePool,
    user_id: i64,
    category_id: i64,
    name: &str,
    path_url: &str,
    entry_type: &str,
    source_path: Option<&str>,
    now: i64,
) -> Result<KbEntry, AppError> {
    sqlx::query_as::<_, KbEntry>(
        "INSERT INTO kb_entries (user_id, category_id, name, path_url, entry_type, source_path, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(user_id)
    .bind(category_id)
    .bind(name)
    .bind(path_url)
    .bind(entry_type)
    .bind(source_path)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_entry(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM kb_entries WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn update_entry(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    name: Option<&str>,
    path_url: Option<&str>,
    entry_type: Option<&str>,
    category_id: Option<i64>,
    content: Option<&str>,
) -> Result<KbEntry, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let existing: KbEntry = sqlx::query_as::<_, KbEntry>("SELECT * FROM kb_entries WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

    let final_name = name.unwrap_or(&existing.name);
    let final_path = path_url.unwrap_or(&existing.path_url);
    let final_type = entry_type.unwrap_or(&existing.entry_type);
    let final_cat_id = category_id.unwrap_or(existing.category_id);
    let final_content = content.or(existing.content.as_deref());

    sqlx::query_as::<_, KbEntry>(
        "UPDATE kb_entries SET name = ?, path_url = ?, entry_type = ?, category_id = ?, content = ?, updated_at = ? WHERE id = ? AND user_id = ? RETURNING *",
    )
    .bind(final_name)
    .bind(final_path)
    .bind(final_type)
    .bind(final_cat_id)
    .bind(final_content)
    .bind(now)
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn search_entries(pool: &SqlitePool, user_id: i64, query: &str) -> Result<Vec<KbEntry>, AppError> {
    let pattern = format!("%{}%", query);
    sqlx::query_as::<_, KbEntry>(
        "SELECT * FROM kb_entries WHERE user_id = ? AND (name LIKE ? OR path_url LIKE ?) ORDER BY created_at DESC",
    )
    .bind(user_id)
    .bind(&pattern)
    .bind(&pattern)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn add_entries_batch(
    pool: &SqlitePool,
    user_id: i64,
    entries: Vec<(i64, &str, &str, &str, Option<&str>, i64)>,
) -> Result<Vec<KbEntry>, AppError> {
    let mut results = Vec::new();
    for (category_id, name, path_url, entry_type, source_path, now) in entries {
        let entry: KbEntry = sqlx::query_as::<_, KbEntry>(
            "INSERT INTO kb_entries (user_id, category_id, name, path_url, entry_type, source_path, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING *",
        )
        .bind(user_id)
        .bind(category_id)
        .bind(name)
        .bind(path_url)
        .bind(entry_type)
        .bind(source_path)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;
        results.push(entry);
    }
    Ok(results)
}

pub async fn move_category(pool: &SqlitePool, user_id: i64, id: i64, target_parent_id: Option<i64>, target_library: Option<String>) -> Result<KbCategory, AppError> {
    if let Some(lib) = target_library {
        sqlx::query_as::<_, KbCategory>(
            "UPDATE kb_categories SET parent_id = ?, library = ? WHERE id = ? AND user_id = ? RETURNING *",
        )
        .bind(target_parent_id)
        .bind(lib)
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
    } else {
        sqlx::query_as::<_, KbCategory>(
            "UPDATE kb_categories SET parent_id = ? WHERE id = ? AND user_id = ? RETURNING *",
        )
        .bind(target_parent_id)
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
    }
}

pub async fn move_entry(pool: &SqlitePool, user_id: i64, id: i64, target_category_id: i64) -> Result<KbEntry, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query_as::<_, KbEntry>(
        "UPDATE kb_entries SET category_id = ?, updated_at = ? WHERE id = ? AND user_id = ? RETURNING *",
    )
    .bind(target_category_id)
    .bind(now)
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_all_descendant_category_ids(pool: &SqlitePool, user_id: i64, root_id: i64) -> Result<Vec<i64>, AppError> {
    let mut all_ids = vec![root_id];
    let mut i = 0;
    while i < all_ids.len() {
        let current = all_ids[i];
        let children: Vec<(i64,)> = sqlx::query_as("SELECT id FROM kb_categories WHERE parent_id = ? AND user_id = ?")
            .bind(current)
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(AppError::Database)?;
        for (child_id,) in children {
            all_ids.push(child_id);
        }
        i += 1;
    }
    Ok(all_ids)
}

pub async fn get_entry_by_id(pool: &SqlitePool, user_id: i64, id: i64) -> Result<KbEntry, AppError> {
    sqlx::query_as::<_, KbEntry>("SELECT * FROM kb_entries WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn get_category_by_id(pool: &SqlitePool, user_id: i64, id: i64) -> Result<KbCategory, AppError> {
    sqlx::query_as::<_, KbCategory>("SELECT * FROM kb_categories WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn get_all_entries(pool: &SqlitePool, user_id: i64) -> Result<Vec<KbEntry>, AppError> {
    sqlx::query_as::<_, KbEntry>("SELECT * FROM kb_entries WHERE user_id = ? ORDER BY updated_at DESC")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn get_entries_count_by_category(pool: &SqlitePool, user_id: i64) -> Result<Vec<(i64, i64)>, AppError> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT category_id, COUNT(*) as cnt FROM kb_entries WHERE user_id = ? GROUP BY category_id"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(rows)
}

pub async fn get_child_category_ids(pool: &SqlitePool, user_id: i64, parent_id: i64) -> Result<Vec<i64>, AppError> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM kb_categories WHERE parent_id = ? AND user_id = ?"
    )
    .bind(parent_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

pub async fn delete_entries_by_category(pool: &SqlitePool, user_id: i64, category_id: i64) -> Result<u64, AppError> {
    let result = sqlx::query("DELETE FROM kb_entries WHERE category_id = ? AND user_id = ?")
        .bind(category_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(result.rows_affected())
}

pub async fn recursive_delete_category(
    pool: &SqlitePool,
    user_id: i64,
    root_id: i64,
) -> Result<(i64, i64), AppError> {
    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    let mut all_category_ids = vec![root_id];
    let mut i = 0;
    while i < all_category_ids.len() {
        let current = all_category_ids[i];
        let children: Vec<(i64,)> = sqlx::query_as(
            "SELECT id FROM kb_categories WHERE parent_id = ? AND user_id = ?"
        )
        .bind(current)
        .bind(user_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(AppError::Database)?;
        for (child_id,) in children {
            all_category_ids.push(child_id);
        }
        i += 1;
    }

    let mut total_entries = 0i64;
    for &cat_id in &all_category_ids {
        let result = sqlx::query("DELETE FROM kb_entries WHERE category_id = ? AND user_id = ?")
            .bind(cat_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        total_entries += result.rows_affected() as i64;
    }

    for &cat_id in all_category_ids.iter().rev() {
        sqlx::query("DELETE FROM kb_categories WHERE id = ? AND user_id = ?")
            .bind(cat_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
    }

    tx.commit().await.map_err(AppError::Database)?;
    Ok((all_category_ids.len() as i64 - 1, total_entries))
}

pub async fn get_tags(pool: &SqlitePool, user_id: i64) -> Result<Vec<KbTag>, AppError> {
    sqlx::query_as::<_, KbTag>("SELECT * FROM kb_tags WHERE user_id = ? ORDER BY name ASC")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn add_tag(pool: &SqlitePool, user_id: i64, name: &str, color: &str) -> Result<KbTag, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query_as::<_, KbTag>(
        "INSERT INTO kb_tags (user_id, name, color, created_at) VALUES (?, ?, ?, ?) RETURNING *"
    )
    .bind(user_id)
    .bind(name)
    .bind(color)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_tag(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM kb_tags WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn update_tag(pool: &SqlitePool, user_id: i64, id: i64, name: &str, color: &str) -> Result<KbTag, AppError> {
    sqlx::query_as::<_, KbTag>(
        "UPDATE kb_tags SET name = ?, color = ? WHERE id = ? AND user_id = ? RETURNING *"
    )
    .bind(name)
    .bind(color)
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_entry_tags(pool: &SqlitePool, user_id: i64, entry_id: i64) -> Result<Vec<KbTag>, AppError> {
    sqlx::query_as::<_, KbTag>(
        "SELECT t.* FROM kb_tags t
         INNER JOIN kb_entry_tags et ON t.id = et.tag_id
         WHERE et.entry_id = ? AND et.user_id = ? AND t.user_id = ?
         ORDER BY t.name ASC"
    )
    .bind(entry_id)
    .bind(user_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn set_entry_tags(pool: &SqlitePool, user_id: i64, entry_id: i64, tag_ids: &[i64]) -> Result<(), AppError> {
    let mut tx = pool.begin().await.map_err(AppError::Database)?;
    sqlx::query("DELETE FROM kb_entry_tags WHERE entry_id = ? AND user_id = ?")
        .bind(entry_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;
    for &tag_id in tag_ids {
        sqlx::query("INSERT OR IGNORE INTO kb_entry_tags (user_id, entry_id, tag_id) VALUES (?, ?, ?)")
            .bind(user_id)
            .bind(entry_id)
            .bind(tag_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
    }
    tx.commit().await.map_err(AppError::Database)?;
    Ok(())
}

pub async fn batch_add_tag_to_entries(pool: &SqlitePool, user_id: i64, entry_ids: &[i64], tag_id: i64) -> Result<u64, AppError> {
    let mut tx = pool.begin().await.map_err(AppError::Database)?;
    let mut count: u64 = 0;
    for &eid in entry_ids {
        let r = sqlx::query("INSERT OR IGNORE INTO kb_entry_tags (user_id, entry_id, tag_id) VALUES (?, ?, ?)")
            .bind(user_id)
            .bind(eid)
            .bind(tag_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        count += r.rows_affected();
    }
    tx.commit().await.map_err(AppError::Database)?;
    Ok(count)
}

pub async fn batch_remove_tag_from_entries(pool: &SqlitePool, user_id: i64, entry_ids: &[i64], tag_id: i64) -> Result<u64, AppError> {
    if entry_ids.is_empty() { return Ok(0); }
    let placeholders = entry_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("DELETE FROM kb_entry_tags WHERE user_id = ? AND entry_id IN ({}) AND tag_id = ?", placeholders);
    let mut query = sqlx::query(&sql);
    query = query.bind(user_id);
    for id in entry_ids { query = query.bind(id); }
    query = query.bind(tag_id);
    let result = query.execute(pool).await.map_err(AppError::Database)?;
    Ok(result.rows_affected())
}

pub async fn get_entries_by_tag(pool: &SqlitePool, user_id: i64, tag_id: i64) -> Result<Vec<KbEntry>, AppError> {
    sqlx::query_as::<_, KbEntry>(
        "SELECT e.* FROM kb_entries e
         INNER JOIN kb_entry_tags et ON e.id = et.entry_id
         WHERE et.tag_id = ? AND et.user_id = ? AND e.user_id = ?
         ORDER BY e.updated_at DESC"
    )
    .bind(tag_id)
    .bind(user_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn toggle_favorite(pool: &SqlitePool, user_id: i64, entry_id: i64) -> Result<bool, AppError> {
    let row: (i64,) = sqlx::query_as("SELECT is_favorited FROM kb_entries WHERE id = ? AND user_id = ?")
        .bind(entry_id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;
    let new_val = if row.0 == 1 { 0 } else { 1 };
    sqlx::query("UPDATE kb_entries SET is_favorited = ? WHERE id = ? AND user_id = ?")
        .bind(new_val)
        .bind(entry_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(new_val == 1)
}

pub async fn get_favorite_entries(pool: &SqlitePool, user_id: i64) -> Result<Vec<KbEntry>, AppError> {
    sqlx::query_as::<_, KbEntry>(
        "SELECT * FROM kb_entries WHERE is_favorited = 1 AND user_id = ? ORDER BY updated_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn record_access(pool: &SqlitePool, user_id: i64, entry_id: i64) -> Result<(), AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query("INSERT INTO kb_recent_access (user_id, entry_id, accessed_at) VALUES (?, ?, ?)")
        .bind(user_id)
        .bind(entry_id)
        .bind(now)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

    sqlx::query("DELETE FROM kb_recent_access WHERE id NOT IN (
        SELECT id FROM kb_recent_access WHERE entry_id = ? AND user_id = ? ORDER BY accessed_at DESC LIMIT 5
    ) AND entry_id = ? AND user_id = ?")
        .bind(entry_id)
        .bind(user_id)
        .bind(entry_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM kb_recent_access WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;
    if total.0 > 200 {
        sqlx::query("DELETE FROM kb_recent_access WHERE id IN (
            SELECT id FROM kb_recent_access WHERE user_id = ? ORDER BY accessed_at ASC LIMIT 50
        )")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    }

    Ok(())
}

pub async fn get_recent_entries(pool: &SqlitePool, user_id: i64, limit: i64) -> Result<Vec<KbEntry>, AppError> {
    sqlx::query_as::<_, KbEntry>(
        "SELECT DISTINCT e.* FROM kb_entries e
         INNER JOIN kb_recent_access ra ON e.id = ra.entry_id
         WHERE ra.user_id = ? AND e.user_id = ?
         ORDER BY ra.accessed_at DESC
         LIMIT ?"
    )
    .bind(user_id)
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn batch_delete_entries(pool: &SqlitePool, user_id: i64, ids: &[i64]) -> Result<u64, AppError> {
    if ids.is_empty() { return Ok(0); }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("DELETE FROM kb_entries WHERE user_id = ? AND id IN ({})", placeholders);
    let mut query = sqlx::query(&sql);
    query = query.bind(user_id);
    for id in ids { query = query.bind(id); }
    let result = query.execute(pool).await.map_err(AppError::Database)?;
    Ok(result.rows_affected())
}

pub async fn batch_move_entries(pool: &SqlitePool, user_id: i64, ids: &[i64], target_category_id: i64) -> Result<u64, AppError> {
    if ids.is_empty() { return Ok(0); }
    let now = chrono::Utc::now().timestamp_millis();
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("UPDATE kb_entries SET category_id = ?, updated_at = ? WHERE user_id = ? AND id IN ({})", placeholders);
    let mut query = sqlx::query(&sql);
    query = query.bind(target_category_id).bind(now).bind(user_id);
    for id in ids { query = query.bind(id); }
    let result = query.execute(pool).await.map_err(AppError::Database)?;
    Ok(result.rows_affected())
}

pub async fn get_all_entry_tags(pool: &SqlitePool, user_id: i64) -> Result<Vec<EntryTagsResult>, AppError> {
    let rows: Vec<(i64, i64, String, String, i64)> = sqlx::query_as(
        "SELECT et.entry_id, t.id, t.name, t.color, t.created_at FROM kb_entry_tags et
         INNER JOIN kb_tags t ON t.id = et.tag_id
         WHERE et.user_id = ? AND t.user_id = ?
         ORDER BY et.entry_id, t.name"
    )
    .bind(user_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let mut results: Vec<EntryTagsResult> = Vec::new();
    let mut current_entry_id: i64 = -1;
    let mut current_tags: Vec<KbTag> = Vec::new();

    for (entry_id, tag_id, tag_name, tag_color, tag_created_at) in rows {
        if entry_id != current_entry_id {
            if current_entry_id != -1 && !current_tags.is_empty() {
                results.push(EntryTagsResult {
                    entry_id: current_entry_id,
                    tags: std::mem::take(&mut current_tags),
                });
            }
            current_entry_id = entry_id;
        }
        current_tags.push(KbTag {
            id: tag_id,
            user_id,
            name: tag_name,
            color: tag_color,
            created_at: tag_created_at,
        });
    }

    if current_entry_id != -1 && !current_tags.is_empty() {
        results.push(EntryTagsResult {
            entry_id: current_entry_id,
            tags: current_tags,
        });
    }

    Ok(results)
}

pub async fn get_tag_stats(pool: &SqlitePool, user_id: i64) -> Result<Vec<TagStats>, AppError> {
    let rows: Vec<(i64, String, String, i64)> = sqlx::query_as(
        "SELECT t.id, t.name, t.color, COUNT(et.entry_id) as cnt
         FROM kb_tags t
         LEFT JOIN kb_entry_tags et ON t.id = et.tag_id AND et.user_id = ?
         WHERE t.user_id = ?
         GROUP BY t.id
         ORDER BY cnt DESC, t.name ASC"
    )
    .bind(user_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(rows.into_iter().map(|(id, name, color, count)| TagStats {
        tag_id: id,
        tag_name: name,
        tag_color: color,
        entry_count: count,
    }).collect())
}

pub async fn add_tracked_path(
    pool: &SqlitePool,
    user_id: i64,
    path: &str,
    category_id: i64,
    library: &str,
    now: i64,
) -> Result<KbTrackedPath, AppError> {
    sqlx::query_as::<_, KbTrackedPath>(
        "INSERT INTO kb_tracked_paths (user_id, path, category_id, library, last_imported_at, created_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(user_id, path) DO UPDATE SET category_id = ?, library = ?, last_imported_at = ?
         RETURNING *",
    )
    .bind(user_id)
    .bind(path)
    .bind(category_id)
    .bind(library)
    .bind(now)
    .bind(now)
    .bind(category_id)
    .bind(library)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_tracked_paths(
    pool: &SqlitePool,
    user_id: i64,
    library: Option<&str>,
) -> Result<Vec<KbTrackedPath>, AppError> {
    if let Some(lib) = library {
        sqlx::query_as::<_, KbTrackedPath>(
            "SELECT * FROM kb_tracked_paths WHERE user_id = ? AND library = ? ORDER BY last_imported_at DESC",
        )
        .bind(user_id)
        .bind(lib)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    } else {
        sqlx::query_as::<_, KbTrackedPath>(
            "SELECT * FROM kb_tracked_paths WHERE user_id = ? ORDER BY last_imported_at DESC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
    }
}

pub async fn remove_tracked_path(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM kb_tracked_paths WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn get_all_tracked_paths(pool: &SqlitePool, user_id: i64) -> Result<Vec<KbTrackedPath>, AppError> {
    sqlx::query_as::<_, KbTrackedPath>("SELECT * FROM kb_tracked_paths WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn get_external_entries(pool: &SqlitePool, user_id: i64) -> Result<Vec<KbEntry>, AppError> {
    sqlx::query_as::<_, KbEntry>(
        "SELECT * FROM kb_entries WHERE user_id = ? AND path_url NOT LIKE 'kb://%' AND path_url NOT LIKE 'http%'"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_templates(pool: &SqlitePool, user_id: i64) -> Result<Vec<KbTemplate>, AppError> {
    // 返回系统内置模板（user_id=0）+ 当前用户自定义模板
    sqlx::query_as::<_, KbTemplate>("SELECT * FROM kb_templates WHERE user_id = 0 OR user_id = ? ORDER BY created_at ASC")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn get_template(pool: &SqlitePool, user_id: i64, id: i64) -> Result<KbTemplate, AppError> {
    sqlx::query_as::<_, KbTemplate>("SELECT * FROM kb_templates WHERE id = ? AND (user_id = 0 OR user_id = ?)")
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn create_template(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    icon: &str,
    description: &str,
    entry_type: &str,
    content: &str,
    now: i64,
) -> Result<KbTemplate, AppError> {
    sqlx::query_as::<_, KbTemplate>(
        "INSERT INTO kb_templates (user_id, name, icon, description, entry_type, content, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(user_id)
    .bind(name)
    .bind(icon)
    .bind(description)
    .bind(entry_type)
    .bind(content)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn update_template(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    name: Option<&str>,
    icon: Option<&str>,
    description: Option<&str>,
    entry_type: Option<&str>,
    content: Option<&str>,
    now: i64,
) -> Result<KbTemplate, AppError> {
    let existing = get_template(pool, user_id, id).await?;
    let name = name.unwrap_or(&existing.name);
    let icon = icon.unwrap_or(&existing.icon);
    let description = description.unwrap_or(&existing.description);
    let entry_type = entry_type.unwrap_or(&existing.entry_type);
    let content = content.unwrap_or(&existing.content);
    sqlx::query_as::<_, KbTemplate>(
        "UPDATE kb_templates SET name = ?, icon = ?, description = ?, entry_type = ?, content = ?, updated_at = ? WHERE id = ? AND user_id = ? RETURNING *",
    )
    .bind(name)
    .bind(icon)
    .bind(description)
    .bind(entry_type)
    .bind(content)
    .bind(now)
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_template(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    // 仅允许删除用户自定义模板（user_id != 0），系统内置模板不可删
    sqlx::query("DELETE FROM kb_templates WHERE id = ? AND user_id = ? AND user_id != 0")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}
