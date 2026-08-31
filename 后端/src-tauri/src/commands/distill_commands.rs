//! 恐龙双脑 Phase 0 IPC 命令
//!
//! 设计依据：
//!   - 功能展望/v2_核心战略/01_恐龙双脑智能架构_实施路线图_补充.md §2.1 Phase 0
//!   - .trae/rules/项目核心设计意图.md §七（蒸馏教师模型走云端 API）§8.1（底层智能走本地模型）
//!
//! 命令清单：
//!   - distill_extract_samples      从数据源抽取待改写样本
//!   - distill_generate_dataset     完整生成管线（抽取 → 教师改写 → 入库）
//!   - distill_list_samples         列出蒸馏样本
//!   - distill_approve_sample       审核通过样本
//!   - distill_reject_sample        驳回样本
//!   - distill_export_jsonl         Phase0.1 导出样本为 JSONL（供训练使用）
//!   - distill_sample_stats         Phase0.1 统计样本数量（按状态分组）
//!   - distill_local_infer          本地推理 PoC（走 ollama，非云端）

use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::services::ai_model_service::AiModelService;
use crate::services::distill_dataset_service::{
    self, DistillSample, DistillSampleRecord, GenerateDatasetRequest, GenerateDatasetResult,
    ListSamplesRequest,
};
use crate::services::local_inference_service::LocalInferenceService;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

/// 从指定数据源抽取待改写样本（不入库，仅预览）
#[tauri::command]
pub async fn distill_extract_samples(
    source: String,
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<DistillSample>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let pool = state.pool.clone();
    let limit = limit.unwrap_or(50).min(200);
    let samples = match source.as_str() {
        "activity" => distill_dataset_service::extract_from_activity_logs(&pool, limit).await,
        "chat" => distill_dataset_service::extract_from_chat_messages(&pool, user_id, limit).await,
        "kb" => distill_dataset_service::extract_from_kb_entries(&pool, user_id, limit).await,
        _ => Err(crate::error::app_error::AppError::Validation(format!(
            "未知数据源: {}（支持: activity / chat / kb）",
            source
        ))),
    }
    .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(samples))
}

/// 完整生成管线：抽取 → 教师模型改写（云端 API）→ 入库
#[tauri::command]
pub async fn distill_generate_dataset(
    request: GenerateDatasetRequest,
    state: State<'_, AppState>,
) -> Result<ApiResponse<GenerateDatasetResult>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let pool = state.pool.clone();
    let mek_manager = state.mek_manager.clone();
    let ai_service = AiModelService::new();
    distill_dataset_service::generate_dataset(&pool, user_id, &mek_manager, &ai_service, request)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 列出蒸馏样本（支持按 status / source_type 筛选 + 分页）
#[tauri::command]
pub async fn distill_list_samples(
    request: ListSamplesRequest,
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<DistillSampleRecord>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let pool = state.pool.clone();
    distill_dataset_service::list_samples(&pool, request)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 审核通过样本（可附加质量评分 0.0~1.0）
#[tauri::command]
pub async fn distill_approve_sample(
    id: i64,
    quality_score: Option<f64>,
    state: State<'_, AppState>,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    let pool = state.pool.clone();
    distill_dataset_service::approve_sample(&pool, id, quality_score)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 驳回样本
#[tauri::command]
pub async fn distill_reject_sample(
    id: i64,
    state: State<'_, AppState>,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    let pool = state.pool.clone();
    distill_dataset_service::reject_sample(&pool, id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 本地推理 PoC（走 ollama，遵循项目核心设计意图 §8.1）
///
/// Phase 0 验证推理管线可用性。V5 将替换为恐龙双脑定制模型。
#[tauri::command]
pub async fn distill_local_infer(
    state: State<'_, AppState>,
    prompt: String,
    system_prompt: Option<String>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let service = LocalInferenceService::with_defaults();
    service
        .infer(&prompt, system_prompt.as_deref())
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// Phase0.1 导出样本为 JSONL 文件（供训练使用）
///
/// 将 distill_dataset 表中指定状态的样本导出为 JSONL 格式文件。
/// 每行一个 JSON 对象：`{"instruction": "...", "output": "...", "source": "...", "metadata": {...}}`
///
/// 验收标准（开发计划 §2.2.5）：可导出训练样本
#[tauri::command]
pub async fn distill_export_jsonl(
    output_path: String,
    status_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<ApiResponse<usize>, String> {
    crate::commands::common::require_auth(&state).await?;
    let pool = state.pool.clone();
    distill_dataset_service::export_samples_jsonl(
        &pool,
        &output_path,
        status_filter.as_deref(),
    )
    .await
    .map(ApiResponse::success)
    .map_err(|e| e.to_string())
}

/// Phase0.1 统计样本数量（按状态分组）
///
/// 返回 JSON：`{"approved": 150, "pending": 30, "rejected": 5}`
#[tauri::command]
pub async fn distill_sample_stats(
    state: State<'_, AppState>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let pool = state.pool.clone();
    distill_dataset_service::count_samples_by_status(&pool)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}
