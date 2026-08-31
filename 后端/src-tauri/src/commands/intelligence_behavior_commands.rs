/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::intelligence::{
    BehaviorAnalysisResult, CognitiveLoadSnapshot, ProjectTechStack,
    SmartNotification, UsageDashboard, WorkflowRecommendation,
};
use crate::services::intelligence_behavior_service::{
    BehaviorAnalyzer, CognitiveLoadTracker, DashboardBuilder, SmartNotifier,
    TechStackDetector, WorkflowRecommender,
};

#[tauri::command]
pub async fn intelligence_v2_analyze_behavior(
    state: State<'_, AppState>,
) -> Result<ApiResponse<BehaviorAnalysisResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let activities = state.intelligence_service.get_recent_activities().await;
    BehaviorAnalyzer::analyze(&activities)
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v2_recommend_workflows(
    state: State<'_, AppState>,
    current_context: Option<String>,
) -> Result<ApiResponse<Vec<WorkflowRecommendation>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let activities = state.intelligence_service.get_recent_activities().await;
    let ctx = current_context.unwrap_or_default();
    let recs = WorkflowRecommender::recommend(&activities, &ctx);
    Ok(ApiResponse::success(recs))
}

#[tauri::command]
pub async fn intelligence_v2_detect_tech_stack(
    state: State<'_, AppState>,
    project_dir: String,
) -> Result<ApiResponse<ProjectTechStack>, String> {
    crate::commands::common::require_auth(&state).await?;
    TechStackDetector::detect(&project_dir)
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn intelligence_v2_cognitive_load(
    state: State<'_, AppState>,
    window_secs: Option<u64>,
) -> Result<ApiResponse<CognitiveLoadSnapshot>, String> {
    crate::commands::common::require_auth(&state).await?;
    let activities = state.intelligence_service.get_recent_activities().await;
    let snapshot = CognitiveLoadTracker::assess(&activities, window_secs.unwrap_or(3600));
    Ok(ApiResponse::success(snapshot))
}

#[tauri::command]
pub async fn intelligence_v2_notifications(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<SmartNotification>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let activities = state.intelligence_service.get_recent_activities().await;
    let cognitive = CognitiveLoadTracker::assess(&activities, 3600);
    let notifications = SmartNotifier::generate(&activities, &cognitive);
    Ok(ApiResponse::success(notifications))
}

#[tauri::command]
pub async fn intelligence_v2_dashboard(
    state: State<'_, AppState>,
    period: Option<String>,
) -> Result<ApiResponse<UsageDashboard>, String> {
    crate::commands::common::require_auth(&state).await?;
    let activities = state.intelligence_service.get_recent_activities().await;
    let period = period.unwrap_or_else(|| "day".into());
    let dashboard = DashboardBuilder::build(&activities, &period);
    Ok(ApiResponse::success(dashboard))
}