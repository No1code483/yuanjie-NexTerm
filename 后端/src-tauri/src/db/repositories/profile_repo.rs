use sqlx::Row;
use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::profile::{Quote, QuoteDuplicateResult, Resume, UserProfile};

pub async fn get_profile(pool: &SqlitePool, key: &str) -> Result<Option<UserProfile>, AppError> {
    sqlx::query_as::<_, UserProfile>("SELECT * FROM user_profiles WHERE field_key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn set_profile(
    pool: &SqlitePool,
    key: &str,
    value: &str,
    now: i64,
) -> Result<UserProfile, AppError> {
    sqlx::query_as::<_, UserProfile>(
        "INSERT INTO user_profiles (field_key, field_value, updated_at)
         VALUES (?, ?, ?)
         ON CONFLICT(field_key) DO UPDATE SET field_value = ?, updated_at = ?
         RETURNING *",
    )
    .bind(key)
    .bind(value)
    .bind(now)
    .bind(value)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_resumes(pool: &SqlitePool, user_id: i64) -> Result<Vec<Resume>, AppError> {
    sqlx::query_as::<_, Resume>("SELECT * FROM resumes WHERE user_id = ? ORDER BY updated_at DESC")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn add_resume(
    pool: &SqlitePool,
    user_id: i64,
    title: &str,
    content: &str,
    now: i64,
) -> Result<Resume, AppError> {
    sqlx::query_as::<_, Resume>(
        "INSERT INTO resumes (user_id, title, content, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(user_id)
    .bind(title)
    .bind(content)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn update_resume(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    title: &str,
    content: &str,
    now: i64,
) -> Result<Resume, AppError> {
    sqlx::query_as::<_, Resume>(
        "UPDATE resumes SET title = ?, content = ?, updated_at = ? WHERE id = ? AND user_id = ? RETURNING *",
    )
    .bind(title)
    .bind(content)
    .bind(now)
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn delete_resume(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM resumes WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn get_random_quote(pool: &SqlitePool, user_id: i64) -> Result<Option<Quote>, AppError> {
    sqlx::query_as::<_, Quote>("SELECT * FROM quotes WHERE user_id = ? ORDER BY RANDOM() LIMIT 1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn add_quote(
    pool: &SqlitePool,
    user_id: i64,
    content: &str,
    source: Option<&str>,
    quote_type: &str,
) -> Result<Quote, AppError> {
    sqlx::query_as::<_, Quote>(
        "INSERT INTO quotes (user_id, content, source, type) VALUES (?, ?, ?, ?) RETURNING *",
    )
    .bind(user_id)
    .bind(content)
    .bind(source)
    .bind(quote_type)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

pub async fn get_all_quotes(pool: &SqlitePool, user_id: i64) -> Result<Vec<Quote>, AppError> {
    sqlx::query_as::<_, Quote>("SELECT * FROM quotes WHERE user_id = ? ORDER BY id ASC")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
}

pub async fn batch_add_quotes(
    pool: &SqlitePool,
    user_id: i64,
    quotes: &[(String, Option<String>, String)],
) -> Result<Vec<Quote>, AppError> {
    let mut results = Vec::with_capacity(quotes.len());
    for (content, source, quote_type) in quotes {
        let quote = sqlx::query_as::<_, Quote>(
            "INSERT INTO quotes (user_id, content, source, type) VALUES (?, ?, ?, ?) RETURNING *",
        )
        .bind(user_id)
        .bind(content)
        .bind(source)
        .bind(quote_type)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;
        results.push(quote);
    }
    Ok(results)
}

pub async fn delete_all_quotes(pool: &SqlitePool, user_id: i64) -> Result<u64, AppError> {
    let result = sqlx::query("DELETE FROM quotes WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(result.rows_affected())
}

pub async fn delete_quote_by_id(pool: &SqlitePool, user_id: i64, id: i64) -> Result<u64, AppError> {
    let result = sqlx::query("DELETE FROM quotes WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(result.rows_affected())
}

pub async fn get_personal_info(pool: &SqlitePool) -> Result<Option<String>, AppError> {
    let row = sqlx::query("SELECT field_value FROM user_profiles WHERE field_key = 'personal_info'")
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(row.and_then(|r| {
        let val: Option<String> = r.get(0);
        val
    }))
}

pub async fn save_personal_info(pool: &SqlitePool, json: &str, now: i64) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO user_profiles (field_key, field_value, updated_at)
         VALUES ('personal_info', ?, ?)
         ON CONFLICT(field_key) DO UPDATE SET field_value = ?, updated_at = ?",
    )
    .bind(json)
    .bind(now)
    .bind(json)
    .bind(now)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

fn char_bigram_jaccard(a: &str, b: &str) -> f64 {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    if a_chars.len() < 2 && b_chars.len() < 2 {
        return if a == b { 1.0 } else { 0.0 };
    }
    if a_chars.len() < 2 || b_chars.len() < 2 {
        let longer = if a_chars.len() > b_chars.len() { a } else { b };
        let shorter = if a_chars.len() > b_chars.len() { b } else { a };
        return if longer.contains(shorter) { 1.0 } else { 0.0 };
    }

    let a_bigrams: std::collections::HashSet<(char, char)> =
        a_chars.windows(2).map(|w| (w[0], w[1])).collect();
    let b_bigrams: std::collections::HashSet<(char, char)> =
        b_chars.windows(2).map(|w| (w[0], w[1])).collect();

    let intersection = a_bigrams.intersection(&b_bigrams).count();
    let union = a_bigrams.union(&b_bigrams).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

fn split_sentences(text: &str) -> Vec<String> {
    let delimiters = ['。', '！', '？', '；', '，', '.', '!', '?', '\n'];
    let mut sentences: Vec<String> = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if delimiters.contains(&ch) {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                sentences.push(trimmed);
            }
            current = String::new();
        } else {
            current.push(ch);
        }
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        sentences.push(trimmed);
    }

    sentences
}

fn sentence_overlap_ratio(new_text: &str, existing_text: &str) -> (f64, f64) {
    let new_sentences = split_sentences(new_text);
    let existing_sentences = split_sentences(existing_text);

    if new_sentences.is_empty() || existing_sentences.is_empty() {
        return (0.0, 0.0);
    }

    let new_in_existing = new_sentences
        .iter()
        .filter(|s| existing_sentences.iter().any(|e| e.contains(s.as_str()) || s.contains(e.as_str())))
        .count() as f64
        / new_sentences.len() as f64;

    let existing_in_new = existing_sentences
        .iter()
        .filter(|e| new_sentences.iter().any(|s| s.contains(e.as_str()) || e.contains(s.as_str())))
        .count() as f64
        / existing_sentences.len() as f64;

    (new_in_existing, existing_in_new)
}

fn is_cjk_punctuation(c: char) -> bool {
    matches!(
        c,
        '\u{3002}'   // 。
            | '\u{FF01}' // ！
            | '\u{FF1F}' // ？
            | '\u{FF1B}' // ；
            | '\u{FF0C}' // ，
            | '\u{FF1A}' // ：
            | '\u{201C}' // "
            | '\u{201D}' // "
            | '\u{2018}' // '
            | '\u{2019}' // '
            | '\u{300A}' // 《
            | '\u{300B}' // 》
            | '\u{300C}' // 「
            | '\u{300D}' // 」
            | '\u{300E}' // 『
            | '\u{300F}' // 』
            | '\u{3010}' // 【
            | '\u{3011}' // 】
            | '\u{FF08}' // （
            | '\u{FF09}' // ）
            | '\u{2026}' // …
            | '\u{2014}' // —
            | '\u{FF5E}' // ～
    )
}

fn strip_punctuation(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_ascii_punctuation() && !is_cjk_punctuation(*c))
        .collect()
}

pub async fn find_similar_quotes(
    pool: &SqlitePool,
    user_id: i64,
    content: &str,
) -> Result<Vec<QuoteDuplicateResult>, AppError> {
    let existing = get_all_quotes(pool, user_id).await?;
    let new_trimmed = content.trim().to_lowercase();

    if new_trimmed.is_empty() {
        return Ok(vec![]);
    }

    let new_stripped = strip_punctuation(&new_trimmed);

    let mut results: Vec<QuoteDuplicateResult> = Vec::new();

    for quote in &existing {
        let existing_trimmed = quote.content.trim().to_lowercase();
        let existing_stripped = strip_punctuation(&existing_trimmed);

        if existing_trimmed == new_trimmed {
            results.push(QuoteDuplicateResult {
                existing: quote.clone(),
                match_type: "exact".into(),
                match_detail: "内容完全相同".into(),
            });
            continue;
        }

        if existing_trimmed.contains(&new_trimmed) {
            results.push(QuoteDuplicateResult {
                existing: quote.clone(),
                match_type: "partial".into(),
                match_detail: "新语录是已有语录的子内容".into(),
            });
            continue;
        }

        if new_trimmed.contains(&existing_trimmed) {
            results.push(QuoteDuplicateResult {
                existing: quote.clone(),
                match_type: "partial".into(),
                match_detail: "已有语录是新语录的子内容".into(),
            });
            continue;
        }

        let jaccard = char_bigram_jaccard(&new_stripped, &existing_stripped);
        if jaccard >= 0.85 {
            results.push(QuoteDuplicateResult {
                existing: quote.clone(),
                match_type: "fuzzy".into(),
                match_detail: "内容高度相似（仅个别字符差异）".into(),
            });
            continue;
        }

        let (new_in_existing, existing_in_new) =
            sentence_overlap_ratio(&new_trimmed, &existing_trimmed);
        if new_in_existing > 0.5 {
            results.push(QuoteDuplicateResult {
                existing: quote.clone(),
                match_type: "fuzzy".into(),
                match_detail: "新语录的大部分句子已存在于已有语录中".into(),
            });
            continue;
        }
        if existing_in_new > 0.5 {
            results.push(QuoteDuplicateResult {
                existing: quote.clone(),
                match_type: "fuzzy".into(),
                match_detail: "已有语录的大部分句子存在于新语录中".into(),
            });
            continue;
        }
    }

    Ok(results)
}
