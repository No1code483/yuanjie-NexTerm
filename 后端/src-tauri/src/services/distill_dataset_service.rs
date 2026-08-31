//! 恐龙双脑 Phase 0：蒸馏数据集服务
//!
//! 设计依据：
//!   - 功能展望/v2_核心战略/01_恐龙双脑智能架构_实施路线图_补充.md §2.1 Phase 0
//!   - .trae/rules/项目核心设计意图.md §七（教师模型走云端 API，非底层智能模型）
//!
//! Phase 0 任务：
//!   1. 数据源对接：从 activity_logs / messages / kb_entries 抽取输入样本
//!   2. Schema 落地：distill_dataset 表（v96 迁移）
//!   3. 生成管线 PoC：调用教师模型（云端 API）改写样本并入库
//!
//! 注意：本服务不涉及学生模型训练（Phase 1+），仅做数据准备。
//! 教师模型通过 AiModelService 调用云端 API，遵循项目核心设计意图 §七模型接入总览。

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tokio::sync::RwLock;

use crate::crypto::mek_manager::MekManager;
use crate::error::app_error::AppError;
use crate::services::ai_model_service::AiModelService;

/// 抽取的待改写样本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistillSample {
    pub source_type: String,
    pub source_id: String,
    pub input_text: String,
    pub metadata: serde_json::Value,
}

/// 生成管线请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateDatasetRequest {
    /// 数据源：`activity` | `chat` | `kb` | `all`
    pub source: String,
    /// 抽取上限（默认 50，最大 200）
    pub limit: Option<i64>,
    /// 教师模型 ID（从 ai_models 表选择，走云端 API）
    pub teacher_model_id: i64,
    /// 用户 ID（用于解密 API Key）
    pub user_id: i64,
}

/// 生成管线结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateDatasetResult {
    pub extracted: usize,
    pub generated: usize,
    pub failed: usize,
    pub sample_ids: Vec<i64>,
}

/// 列表查询请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSamplesRequest {
    pub status: Option<String>,
    pub source_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 样本记录（查询返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistillSampleRecord {
    pub id: i64,
    pub source_type: String,
    pub source_id: Option<String>,
    pub input_text: String,
    pub teacher_output: String,
    pub metadata: serde_json::Value,
    pub quality_score: f64,
    pub status: String,
    pub created_at: i64,
    pub reviewed_at: Option<i64>,
}

/// 教师模型系统提示词：将原始用户活动改写为干净训练样本
const TEACHER_SYSTEM_PROMPT: &str = r#"You are a teacher model assisting in distilling a student model for the NexTerm·元界 desktop app.
Given the following user activity/input text, generate a clean, high-quality training sample:
1. Rewrite the input into a clear instruction or context.
2. Provide a concise, accurate, and helpful response that a local student model should learn to produce.
3. Keep the response focused and under 300 tokens.
4. Respond in the same language as the input.

Output format (strict):
REWRITTEN_INPUT: <rewritten instruction/context>
TEACHER_RESPONSE: <your response>"#;

// ===== 数据源对接 =====

/// 从 activity_logs 抽取样本
pub async fn extract_from_activity_logs(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<DistillSample>, AppError> {
    let rows = sqlx::query(
        "SELECT id, module, operation, detail, remark FROM activity_logs
         WHERE detail IS NOT NULL AND detail != ''
         ORDER BY id DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut samples = Vec::new();
    for row in rows {
        let id: i64 = row.try_get("id").unwrap_or(0);
        let module: String = row.try_get("module").unwrap_or_default();
        let operation: String = row.try_get("operation").unwrap_or_default();
        let detail: String = row.try_get("detail").unwrap_or_default();
        let remark: Option<String> = row.try_get("remark").ok().flatten();

        let input_text = format!(
            "[{}] {} {}{}",
            module,
            operation,
            detail,
            remark
                .filter(|r| !r.is_empty())
                .map(|r| format!(" | 备注: {}", r))
                .unwrap_or_default()
        );

        samples.push(DistillSample {
            source_type: "activity".into(),
            source_id: id.to_string(),
            input_text,
            metadata: serde_json::json!({
                "module": module,
                "operation": operation,
            }),
        });
    }
    Ok(samples)
}

/// 从 messages（聊天）抽取样本（多用户隔离批次 2：按 user_id 过滤）
pub async fn extract_from_chat_messages(
    pool: &SqlitePool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<DistillSample>, AppError> {
    let rows = sqlx::query(
        "SELECT id, content, conversation_id, sender_type FROM messages
         WHERE user_id = ? AND content IS NOT NULL AND length(content) > 10
         ORDER BY id DESC LIMIT ?",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut samples = Vec::new();
    for row in rows {
        let id: i64 = row.try_get("id").unwrap_or(0);
        let content: String = row.try_get("content").unwrap_or_default();
        let conversation_id: i64 = row.try_get("conversation_id").unwrap_or(0);
        let sender_type: String = row.try_get("sender_type").unwrap_or_default();

        samples.push(DistillSample {
            source_type: "chat".into(),
            source_id: id.to_string(),
            input_text: content,
            metadata: serde_json::json!({
                "conversation_id": conversation_id,
                "sender_type": sender_type,
            }),
        });
    }
    Ok(samples)
}

/// 从 kb_entries 抽取样本（多用户隔离：按 user_id 过滤，与 extract_from_chat_messages 保持一致）
pub async fn extract_from_kb_entries(
    pool: &SqlitePool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<DistillSample>, AppError> {
    let rows = sqlx::query(
        "SELECT id, name, content, entry_type FROM kb_entries
         WHERE user_id = ? AND content IS NOT NULL AND content != ''
         ORDER BY id DESC LIMIT ?",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut samples = Vec::new();
    for row in rows {
        let id: i64 = row.try_get("id").unwrap_or(0);
        let name: String = row.try_get("name").unwrap_or_default();
        let content: String = row.try_get("content").unwrap_or_default();
        let entry_type: Option<String> = row.try_get("entry_type").ok().flatten();

        let input_text = format!("标题: {}\n内容: {}", name, content);

        samples.push(DistillSample {
            source_type: "kb".into(),
            source_id: id.to_string(),
            input_text,
            metadata: serde_json::json!({
                "name": name,
                "entry_type": entry_type,
            }),
        });
    }
    Ok(samples)
}

// ===== 生成管线 PoC =====

/// 调用教师模型生成改写后的训练样本
///
/// 返回 (rewritten_input, teacher_response)
pub async fn generate_teacher_output(
    pool: &SqlitePool,
    mek_manager: &Arc<RwLock<MekManager>>,
    ai_service: &AiModelService,
    user_id: i64,
    teacher_model_id: i64,
    input_text: &str,
) -> Result<(String, String), AppError> {
    let prompt = format!(
        "Input text:\n{}\n\nPlease generate the training sample.",
        input_text
    );
    let raw = ai_service
        .call_model(
            pool,
            mek_manager,
            user_id,
            teacher_model_id,
            &prompt,
            Some(TEACHER_SYSTEM_PROMPT),
        )
        .await?;

    Ok(parse_teacher_response(&raw))
}

fn parse_teacher_response(raw: &str) -> (String, String) {
    let mut rewritten = String::new();
    let mut response = String::new();
    let mut current_section: Option<&str> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("REWRITTEN_INPUT:") {
            rewritten.push_str(rest.trim());
            current_section = Some("rewritten");
        } else if let Some(rest) = trimmed.strip_prefix("TEACHER_RESPONSE:") {
            response.push_str(rest.trim());
            current_section = Some("response");
        } else if let Some(section) = current_section {
            match section {
                "rewritten" => {
                    rewritten.push('\n');
                    rewritten.push_str(trimmed);
                }
                "response" => {
                    response.push('\n');
                    response.push_str(trimmed);
                }
                _ => {}
            }
        }
    }

    // 若模型未按协议响应，把整段作为 response，rewritten 留空
    if rewritten.is_empty() && response.is_empty() {
        response.push_str(raw);
    }
    (rewritten, response)
}

/// 插入样本到 distill_dataset 表
pub async fn insert_sample(
    pool: &SqlitePool,
    source_type: &str,
    source_id: &str,
    input_text: &str,
    teacher_output: &str,
    metadata: &serde_json::Value,
) -> Result<i64, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let result = sqlx::query(
        "INSERT INTO distill_dataset
            (source_type, source_id, input_text, teacher_output, metadata, quality_score, status, created_at)
         VALUES (?, ?, ?, ?, ?, 0.0, 'pending', ?)",
    )
    .bind(source_type)
    .bind(source_id)
    .bind(input_text)
    .bind(teacher_output)
    .bind(metadata.to_string())
    .bind(now)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// 完整生成管线：抽取 → 教师改写 → 入库（多用户隔离批次 2：按 user_id 过滤聊天样本）
pub async fn generate_dataset(
    pool: &SqlitePool,
    user_id: i64,
    mek_manager: &Arc<RwLock<MekManager>>,
    ai_service: &AiModelService,
    req: GenerateDatasetRequest,
) -> Result<GenerateDatasetResult, AppError> {
    let limit = req.limit.unwrap_or(50).min(200);
    let mut samples = Vec::new();

    if matches!(req.source.as_str(), "activity" | "all") {
        samples.extend(extract_from_activity_logs(pool, limit).await?);
    }
    if matches!(req.source.as_str(), "chat" | "all") {
        samples.extend(extract_from_chat_messages(pool, user_id, limit).await?);
    }
    if matches!(req.source.as_str(), "kb" | "all") {
        samples.extend(extract_from_kb_entries(pool, user_id, limit).await?);
    }

    // 限制总数
    if samples.len() > limit as usize {
        samples.truncate(limit as usize);
    }

    let extracted = samples.len();
    let mut generated = 0usize;
    let mut failed = 0usize;
    let mut sample_ids = Vec::new();

    for sample in samples {
        match generate_teacher_output(
            pool,
            mek_manager,
            ai_service,
            req.user_id,
            req.teacher_model_id,
            &sample.input_text,
        )
        .await
        {
            Ok((rewritten, response)) => {
                // 改写后的 input 优先，为空则回退原 input
                let final_input = if rewritten.is_empty() {
                    sample.input_text.clone()
                } else {
                    rewritten
                };
                let mut meta = sample.metadata.clone();
                if let serde_json::Value::Object(ref mut m) = meta {
                    m.insert(
                        "source_module".into(),
                        serde_json::json!(sample.source_type),
                    );
                }
                match insert_sample(
                    pool,
                    &sample.source_type,
                    &sample.source_id,
                    &final_input,
                    &response,
                    &meta,
                )
                .await
                {
                    Ok(id) => {
                        sample_ids.push(id);
                        generated += 1;
                    }
                    Err(e) => {
                        tracing::warn!("[distill] 入库失败 (source_id={}): {}", sample.source_id, e);
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "[distill] 教师模型改写失败 (source_id={}): {}",
                    sample.source_id,
                    e
                );
                failed += 1;
            }
        }
    }

    tracing::info!(
        "[D2-Phase0] 蒸馏数据集生成完成: 抽取 {} / 成功 {} / 失败 {}",
        extracted,
        generated,
        failed
    );

    Ok(GenerateDatasetResult {
        extracted,
        generated,
        failed,
        sample_ids,
    })
}

// ===== 查询与审核 =====

/// 列出样本（支持按 status / source_type 筛选 + 分页）
pub async fn list_samples(
    pool: &SqlitePool,
    req: ListSamplesRequest,
) -> Result<Vec<DistillSampleRecord>, AppError> {
    let limit = req.limit.unwrap_or(50).min(500);
    let offset = req.offset.unwrap_or(0);

    let mut sql = String::from(
        "SELECT id, source_type, source_id, input_text, teacher_output, metadata,
                quality_score, status, created_at, reviewed_at
         FROM distill_dataset WHERE 1=1",
    );
    if req.status.is_some() {
        sql.push_str(" AND status = ?");
    }
    if req.source_type.is_some() {
        sql.push_str(" AND source_type = ?");
    }
    sql.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");

    let mut q = sqlx::query(&sql);
    if let Some(ref s) = req.status {
        q = q.bind(s);
    }
    if let Some(ref s) = req.source_type {
        q = q.bind(s);
    }
    q = q.bind(limit).bind(offset);

    let rows = q.fetch_all(pool).await?;
    let mut records = Vec::new();
    for row in rows {
        let metadata_str: String = row.try_get("metadata").unwrap_or_else(|_| "{}".into());
        let metadata: serde_json::Value =
            serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({}));
        records.push(DistillSampleRecord {
            id: row.try_get("id").unwrap_or(0),
            source_type: row.try_get("source_type").unwrap_or_default(),
            source_id: row.try_get("source_id").ok().flatten(),
            input_text: row.try_get("input_text").unwrap_or_default(),
            teacher_output: row.try_get("teacher_output").unwrap_or_default(),
            metadata,
            quality_score: row.try_get("quality_score").unwrap_or(0.0),
            status: row.try_get("status").unwrap_or_default(),
            created_at: row.try_get("created_at").unwrap_or(0),
            reviewed_at: row.try_get("reviewed_at").ok().flatten(),
        });
    }
    Ok(records)
}

/// 审核通过样本（可附加质量评分）
pub async fn approve_sample(
    pool: &SqlitePool,
    id: i64,
    quality_score: Option<f64>,
) -> Result<bool, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let result = if let Some(score) = quality_score {
        sqlx::query(
            "UPDATE distill_dataset SET status = 'approved', quality_score = ?, reviewed_at = ? WHERE id = ?",
        )
        .bind(score.clamp(0.0, 1.0))
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?
    } else {
        sqlx::query(
            "UPDATE distill_dataset SET status = 'approved', reviewed_at = ? WHERE id = ?",
        )
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?
    };
    Ok(result.rows_affected() > 0)
}

/// 驳回样本
pub async fn reject_sample(pool: &SqlitePool, id: i64) -> Result<bool, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let result = sqlx::query(
        "UPDATE distill_dataset SET status = 'rejected', reviewed_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

// ===== Phase0.1 导出 =====

/// Phase0.1 数据源对接：导出样本为 JSONL 文件（供训练使用）
///
/// 将 distill_dataset 表中指定状态的样本导出为 JSONL 格式文件。
/// 每行一个 JSON 对象：`{"instruction": "...", "output": "..."}`
///
/// 验收标准（开发计划 §2.2.5）：可导出训练样本
/// 默认导出 approved 状态的样本，也可导出 pending 用于预览。
pub async fn export_samples_jsonl(
    pool: &SqlitePool,
    output_path: &str,
    status_filter: Option<&str>,
) -> Result<usize, AppError> {
    let status = status_filter.unwrap_or("approved");

    let rows = sqlx::query(
        "SELECT input_text, teacher_output, source_type, metadata
         FROM distill_dataset
         WHERE status = ?
         ORDER BY id ASC",
    )
    .bind(status)
    .fetch_all(pool)
    .await?;

    use std::io::Write;
    let mut file = std::fs::File::create(output_path).map_err(|e| {
        AppError::Internal(format!("无法创建导出文件 {}: {}", output_path, e))
    })?;

    let mut count = 0usize;
    for row in rows {
        let input_text: String = row.try_get("input_text").unwrap_or_default();
        let teacher_output: String = row.try_get("teacher_output").unwrap_or_default();
        let source_type: String = row.try_get("source_type").unwrap_or_default();
        let metadata_str: String = row.try_get("metadata").unwrap_or_else(|_| "{}".into());
        let metadata: serde_json::Value =
            serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({}));

        // JSONL 格式：每行一个 JSON 对象（兼容 Qwen/Llama 等 SFT 训练格式）
        let record = serde_json::json!({
            "instruction": input_text,
            "output": teacher_output,
            "source": source_type,
            "metadata": metadata,
        });

        let line = serde_json::to_string(&record)
            .map_err(|e| AppError::Internal(format!("JSON 序列化失败: {}", e)))?;

        writeln!(file, "{}", line).map_err(|e| {
            AppError::Internal(format!("写入文件失败: {}", e))
        })?;

        count += 1;
    }

    tracing::info!(
        "[Phase0.1] 导出 {} 条样本（status={}）到 {}",
        count,
        status,
        output_path
    );

    Ok(count)
}

/// Phase0.1 统计样本数量（按状态分组）
pub async fn count_samples_by_status(
    pool: &SqlitePool,
) -> Result<serde_json::Value, AppError> {
    let rows = sqlx::query(
        "SELECT status, COUNT(*) as count FROM distill_dataset GROUP BY status",
    )
    .fetch_all(pool)
    .await?;

    use sqlx::Row;
    let mut stats = serde_json::json!({});
    for row in rows {
        let status: String = row.try_get("status").unwrap_or_default();
        let count: i64 = row.try_get("count").unwrap_or(0);
        stats[status] = serde_json::json!(count);
    }

    Ok(stats)
}

