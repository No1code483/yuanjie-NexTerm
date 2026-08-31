use std::sync::Arc;

use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::skill::{
    AutoDiscoverRequest, DetectImplicitRequest, SkillLoadOutcome, SkillMatchResult,
    SkillMetadata, SkillRegistration, SkillRenderConfig, SkillScope, SkillTriggerRequest,
};
use crate::services::ai_model_service::AiModelService;
use crate::services::cloud_api_router::CloudApiRouter;
use crate::services::skill_market_service::SkillMarketEntry;
use crate::services::skill_service::SkillService;
use crate::skills::builtin::{BuiltinSkillRegistry, SkillInput, SkillOutput};

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn yuan_skill_add_root(
    state: State<'_, AppState>,
    path: String,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service.add_scan_root(&path).await;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn yuan_skill_set_project_files(
    state: State<'_, AppState>,
    files: Vec<String>,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service.set_project_files(files).await;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn yuan_skill_load_all(
    state: State<'_, AppState>,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<SkillLoadOutcome>, String> {
    crate::commands::common::require_auth(&state).await?;
    let outcome = service.load_skills().await;
    Ok(ApiResponse::success(outcome))
}

#[tauri::command]
pub async fn yuan_skill_get(
    state: State<'_, AppState>,
    name: String,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<Option<SkillMetadata>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let skill = service.find_skill(&name).await;
    Ok(ApiResponse::success(skill))
}

#[tauri::command]
pub async fn yuan_skill_list(
    state: State<'_, AppState>,
    scope: Option<String>,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<Vec<SkillMetadata>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let scope_enum = scope.map(|s| SkillScope::from_str(&s));
    let skills = service.list_skills(scope_enum).await;
    Ok(ApiResponse::success(skills))
}

#[tauri::command]
pub async fn yuan_skill_detect_implicit(
    state: State<'_, AppState>,
    request: DetectImplicitRequest,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<Vec<SkillMetadata>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let matches = service.detect_implicit(request).await;
    Ok(ApiResponse::success(matches))
}

#[tauri::command]
pub async fn yuan_skill_register(
    state: State<'_, AppState>,
    registration: SkillRegistration,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .register_skill(registration)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_skill_unregister(
    state: State<'_, AppState>,
    name: String,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .unregister_skill(&name)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_skill_trigger(
    state: State<'_, AppState>,
    request: SkillTriggerRequest,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<Option<SkillMetadata>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let result = service.trigger_skill(request).await;
    Ok(ApiResponse::success(result))
}

#[tauri::command]
pub async fn yuan_skill_auto_discover(
    state: State<'_, AppState>,
    request: AutoDiscoverRequest,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let recommendation = service.auto_discover(request).await;
    Ok(ApiResponse::success(format!(
        "{} (置信度: {:.0}%)",
        recommendation.reason, recommendation.confidence * 100.0
    )))
}

#[tauri::command]
pub async fn yuan_skill_match(
    state: State<'_, AppState>,
    query: String,
    limit: usize,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<Vec<SkillMatchResult>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let results = service.match_skills(&query, limit).await;
    Ok(ApiResponse::success(results))
}

#[tauri::command]
pub async fn yuan_skill_render_context(
    state: State<'_, AppState>,
    config: Option<SkillRenderConfig>,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let cfg = config.unwrap_or_default();
    let text = service.render_context(&cfg).await;
    Ok(ApiResponse::success(text))
}

#[tauri::command]
pub async fn yuan_skill_stats(
    state: State<'_, AppState>,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let total = service.skill_count().await;
    let enabled = service.enabled_count().await;
    let invocations = service.invocation_count().await;
    Ok(ApiResponse::success(serde_json::json!({
        "total": total,
        "enabled": enabled,
        "invocations": invocations,
    })))
}

#[tauri::command]
pub async fn yuan_skill_export(
    state: State<'_, AppState>,
    data: String,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let export_path = std::env::temp_dir().join("yuan_skills_export.json");
    std::fs::write(&export_path, &data)
        .map_err(|e| format!("导出失败: {}", e))?;
    Ok(ApiResponse::success(export_path.to_string_lossy().to_string()))
}

// ===== D1.8 Skill 市场：浏览/搜索/安装/卸载 =====

#[tauri::command]
pub async fn yuan_skill_market_list(
    state: State<'_, AppState>,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<Vec<SkillMarketEntry>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(service.list_market().await))
}

#[tauri::command]
pub async fn yuan_skill_market_search(
    state: State<'_, AppState>,
    query: String,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<Vec<SkillMarketEntry>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(service.search_market(&query).await))
}

#[tauri::command]
pub async fn yuan_skill_market_get(
    state: State<'_, AppState>,
    name: String,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<Option<SkillMarketEntry>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(service.get_market_entry(&name).await))
}

#[tauri::command]
pub async fn yuan_skill_market_install(
    state: State<'_, AppState>,
    name: String,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<SkillMetadata>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .install_market_skill(&name)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_skill_market_uninstall(
    state: State<'_, AppState>,
    name: String,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .uninstall_market_skill(&name)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

// ===== D1 v3.2 Task 3.3.3: Skill 安装/卸载（本地注册表管理）=====
//
// 与 yuan_skill_market_install/uninstall 的关系：
//   market_install/uninstall 是"市场浏览 → 安装"流程的命令；
//   skill_install/uninstall 是"本地注册表管理"的语义化别名，委托同一底层实现。
//   两者均通过 SkillService 管理运行时 SkillsManager.loaded_skills 注册表。

#[tauri::command]
pub async fn yuan_skill_install(
    state: State<'_, AppState>,
    name: String,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<SkillMetadata>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .install_market_skill(&name)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_skill_uninstall(
    state: State<'_, AppState>,
    name: String,
    service: State<'_, SkillService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .uninstall_market_skill(&name)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

// ===== D1 v3.2 Task 3.3.1: 执行内置 Skill（调用云端 API）=====
//
// 强制规则（项目核心设计意图 §三 / §八 8.1.1）：
//   - 所有 Skill 的 AI 调用必须走云端 API（OpenAI/Claude/...）
//   - 由 CloudApiRouter::validate_cloud_only() 守卫，拒绝本地底层智能模型（ollama 等）
//   - provider 必须为 CLOUD_API_PROVIDERS 之一

/// 构造 CloudApiRouter（无状态，每次构造；与 agent/specialized.rs 模式一致）
fn build_cloud_router() -> Arc<CloudApiRouter> {
    Arc::new(CloudApiRouter::new(Arc::new(AiModelService::new())))
}

/// 从 AppState 读取当前登录用户 ID
async fn get_user_id(state: &State<'_, AppState>) -> Result<i64, String> {
    let user = state.current_user.read().await;
    user.ok_or_else(|| String::from("未登录"))
}

#[tauri::command]
pub async fn yuan_skill_execute(
    skill_name: String,
    input: SkillInput,
    provider: String,
    model_name: String,
    state: State<'_, AppState>,
) -> Result<ApiResponse<SkillOutput>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;

    // A5 Phase 3 Task 2: 云端 AI 离线守卫
    //
    // 设计依据：.trae/rules/项目核心设计意图.md §三（Yuan Code Skill 走云端 API）
    // 离线时直接返回 Offline 错误，不进入 Skill 执行（避免触发 CloudApiRouter 调用）。
    crate::services::ai_model_service::AiModelService::check_online_before_call(&state)
        .await
        .map_err(|e| e.to_string())?;

    // 校验 provider 必须为云端 API（强制规则）
    let provider_lower = provider.to_lowercase();
    if !crate::models::api_key::CLOUD_API_PROVIDERS.contains(&provider_lower.as_str()) {
        return Err(format!(
            "Yuan Code Skill 必须走云端 API provider（{}），\
             禁止使用本地底层智能模型 [项目核心设计意图 §八 8.1.1]",
            crate::models::api_key::CLOUD_API_PROVIDERS.join(", ")
        ));
    }

    let router = build_cloud_router();
    let ctx = crate::skills::builtin::SkillExecutionContext {
        router,
        pool: state.pool.clone(),
        mek_manager: state.mek_manager.clone(),
        user_id,
        provider: provider_lower,
        model_name,
    };

    let registry = BuiltinSkillRegistry::new();
    let output = registry
        .execute(&skill_name, input, &ctx)
        .await
        .map_err(|e| e.to_string())?;
    tracing::info!(
        "[skill_execute] skill={} provider={} model={} tokens={:?}",
        output.skill_name,
        output.provider,
        output.model_used,
        output.tokens_used
    );
    Ok(ApiResponse::success(output))
}

/// 列出全部内置 Skill 元数据（市场展示 + execute 路由用）
#[tauri::command]
pub async fn yuan_skill_builtin_list(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<crate::skills::builtin::BuiltinSkillInfo>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let registry = BuiltinSkillRegistry::new();
    Ok(ApiResponse::success(registry.list_metadata()))
}

// ===== D1 v3.2 Task 3.3.4: Skill 评分/评论系统 =====
//
// 表：skill_ratings（migration v111）
// - UNIQUE(skill_name, user_id)：一人一评，更新走 UPSERT
// - rating: 1-5 星
// - review: 可选评论文本

/// 单条评分/评论记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillRatingEntry {
    pub user_id: i64,
    pub rating: i64,
    pub review: Option<String>,
    pub updated_at: i64,
}

/// Skill 评分汇总（市场卡片展示用）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillRatingSummary {
    pub skill_name: String,
    /// 平均评分（0 表示暂无评分）
    pub avg_rating: f64,
    /// 评分总数
    pub total_count: i64,
    /// 评分分布：[1星数, 2星数, 3星数, 4星数, 5星数]
    pub distribution: [i64; 5],
    /// 最近评论（最多 10 条）
    pub recent_reviews: Vec<SkillRatingEntry>,
}

/// 评分/评论（UPSERT：同一 user 对同一 skill 只保留最新一条）
#[tauri::command]
pub async fn yuan_skill_rate(
    skill_name: String,
    rating: i64,
    review: Option<String>,
    state: State<'_, AppState>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    if !(1..=5).contains(&rating) {
        return Err(format!("rating 必须在 1-5 之间，收到 {}", rating));
    }
    let user_id = get_user_id(&state).await?;
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO skill_ratings (skill_name, user_id, rating, review, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?) \
         ON CONFLICT(skill_name, user_id) DO UPDATE SET \
         rating = excluded.rating, review = excluded.review, updated_at = excluded.updated_at",
    )
    .bind(&skill_name)
    .bind(user_id)
    .bind(rating)
    .bind(&review)
    .bind(now)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(|e| format!("评分写入失败: {}", e))?;

    tracing::info!(
        "[skill_rate] skill={} user={} rating={} has_review={}",
        skill_name,
        user_id,
        rating,
        review.is_some()
    );
    Ok(ApiResponse::success(()))
}

/// 更新评论（保留已有评分，仅更新 review 文本）
#[tauri::command]
pub async fn yuan_skill_review(
    skill_name: String,
    review: String,
    state: State<'_, AppState>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    let user_id = get_user_id(&state).await?;
    let now = chrono::Utc::now().timestamp();

    let result = sqlx::query(
        "UPDATE skill_ratings SET review = ?, updated_at = ? \
         WHERE skill_name = ? AND user_id = ?",
    )
    .bind(&review)
    .bind(now)
    .bind(&skill_name)
    .bind(user_id)
    .execute(&state.pool)
    .await
    .map_err(|e| format!("评论更新失败: {}", e))?;

    if result.rows_affected() == 0 {
        return Err(format!(
            "未找到 skill '{}' 的评分记录，请先调用 yuan_skill_rate 评分",
            skill_name
        ));
    }
    Ok(ApiResponse::success(()))
}

/// 获取 Skill 评分汇总（平均分 + 分布 + 最近评论）
#[tauri::command]
pub async fn yuan_skill_ratings_get(
    skill_name: String,
    state: State<'_, AppState>,
) -> Result<ApiResponse<SkillRatingSummary>, String> {
    crate::commands::common::require_auth(&state).await?;
    use sqlx::Row;

    let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT user_id, rating, review, updated_at FROM skill_ratings \
         WHERE skill_name = ? ORDER BY updated_at DESC",
    )
    .bind(&skill_name)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| format!("评分查询失败: {}", e))?;

    let total_count = rows.len() as i64;
    let mut distribution = [0i64; 5];
    let mut sum: i64 = 0;
    let mut recent_reviews: Vec<SkillRatingEntry> = Vec::new();

    for row in rows.iter().take(10) {
        let rating: i64 = row.get("rating");
        if (1..=5).contains(&rating) {
            distribution[(rating - 1) as usize] += 1;
        }
        sum += rating;
        recent_reviews.push(SkillRatingEntry {
            user_id: row.get("user_id"),
            rating,
            review: row.get("review"),
            updated_at: row.get("updated_at"),
        });
    }

    let avg_rating = if total_count > 0 {
        sum as f64 / total_count as f64
    } else {
        0.0
    };

    Ok(ApiResponse::success(SkillRatingSummary {
        skill_name,
        avg_rating,
        total_count,
        distribution,
        recent_reviews,
    }))
}