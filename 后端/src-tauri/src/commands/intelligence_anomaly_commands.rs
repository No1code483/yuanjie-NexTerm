use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::intelligence::{
    AnomalyDetectionResult, AutonomousDecisionResult, KnowledgeOrganizationResult,
    ScheduledTask, SystemResourceSnapshot,
};
use crate::services::intelligence_anomaly_service::{
    AnomalyDetector, AutonomousDecisionEngine, DataIntelligence, KnowledgeEntry,
};

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn intelligence_v3_detect_anomaly(
    state: State<'_, AppState>,
    window_secs: Option<u64>,
) -> Result<ApiResponse<AnomalyDetectionResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let activities = state.intelligence_service.get_recent_activities().await;
    let window = window_secs.unwrap_or(3600);
    let result = AnomalyDetector::detect(&activities, window);
    Ok(ApiResponse::success(result))
}

#[tauri::command]
pub async fn intelligence_v3_snapshot_resources(
    state: State<'_, AppState>,
) -> Result<ApiResponse<SystemResourceSnapshot>, String> {
    crate::commands::common::require_auth(&state).await?;
    let snapshot = AnomalyDetector::snapshot_resources();
    Ok(ApiResponse::success(snapshot))
}

#[tauri::command]
pub async fn intelligence_v3_organize_knowledge(
    state: State<'_, AppState>,
    entries_json: String,
    categories_json: String,
) -> Result<ApiResponse<KnowledgeOrganizationResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let entries: Vec<KnowledgeEntry> = serde_json::from_str(&entries_json)
        .map_err(|e| format!("解析知识条目数据失败: {}", e))?;
    let categories: Vec<String> = serde_json::from_str(&categories_json)
        .map_err(|e| format!("解析分类数据失败: {}", e))?;
    let result = DataIntelligence::organize(&entries, &categories);
    Ok(ApiResponse::success(result))
}

#[tauri::command]
pub async fn intelligence_v3_list_scheduled_tasks(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<ScheduledTask>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let engine = AutonomousDecisionEngine::new();
    Ok(ApiResponse::success(engine.list_tasks()))
}

#[tauri::command]
pub async fn intelligence_v3_execute_scheduled_tasks(
    state: State<'_, AppState>,
) -> Result<ApiResponse<AutonomousDecisionResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut engine = AutonomousDecisionEngine::new();
    let data_dir = state.data_dir.clone();
    let result = engine.execute_due_tasks(&data_dir);
    Ok(ApiResponse::success(result))
}