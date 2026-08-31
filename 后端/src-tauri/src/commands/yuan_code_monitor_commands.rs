//! Yuan Code v3.2 Task 3.4.2 — Yuan Code 底层智能监测 IPC 命令层
//!
//! 命令清单：
//! - yuan_code_monitor_record_event: 非侵入式记录 Yuan Code 事件到底层智能 activity_logs（关闭时 no-op）
//! - yuan_code_monitor_status: 查询底层智能监测钩子状态（启用与否 + 近期事件数）
//! - yuan_code_monitor_summary: 查询监测数据汇总（按 operation 分组 + 每日趋势）
//!
//! 边界（项目核心设计意图 §二、§三、§8.2）：
//! - 此钩子仅"喂数据给底层智能监测"，不替代 Yuan Code 的云端 API 调用
//! - 底层智能关闭时，record_event 立即 no-op，但 Yuan Code 核心功能仍可用
//! - 非侵入式：Yuan Code 流程不依赖此钩子；前端可选地调用 record_event 上报事件

use tauri::State;

use crate::commands::common::require_auth;
use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::services::yuan_code_monitor::{
    self, YuanCodeEventType, YuanCodeMonitorStatus, YuanCodeMonitorSummary,
};

/// 记录一条 Yuan Code 事件供底层智能监测消费（非侵入式、可关闭）
///
/// 前端在发生以下事件后可选调用本命令上报：
/// - route_resolved：路由规则解析完成
/// - cloud_api_call：云端 API 调用完成
/// - agent_execute：Agent 执行完成
/// - user_feedback：用户反馈（采纳/拒绝/编辑）
/// - completion：代码补全
/// - analysis：代码分析
///
/// 底层智能关闭时立即 no-op；写入失败不影响 Yuan Code 流程。
#[tauri::command]
pub async fn yuan_code_monitor_record_event(
    state: State<'_, AppState>,
    event_type: String,
    detail: Option<String>,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;

    let parsed = match event_type.as_str() {
        "route_resolved" => YuanCodeEventType::RouteResolved,
        "cloud_api_call" => YuanCodeEventType::CloudApiCall,
        "agent_execute" => YuanCodeEventType::AgentExecute,
        "user_feedback" => YuanCodeEventType::UserFeedback,
        "completion" => YuanCodeEventType::Completion,
        "analysis" => YuanCodeEventType::Analysis,
        other => YuanCodeEventType::Custom(other.to_string()),
    };

    match yuan_code_monitor::record_event(
        &state.pool,
        &state.intelligence_service,
        user_id,
        parsed,
        detail.as_deref(),
    )
    .await
    {
        Ok(()) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

/// 查询底层智能监测钩子状态（启用与否 + 近期事件数）
#[tauri::command]
pub async fn yuan_code_monitor_status(
    state: State<'_, AppState>,
) -> Result<ApiResponse<YuanCodeMonitorStatus>, String> {
    let user_id = require_auth(&state).await?;
    match yuan_code_monitor::get_status(&state.pool, &state.intelligence_service, user_id).await {
        Ok(status) => Ok(ApiResponse::success(status)),
        Err(e) => {
            // 状态查询失败不应阻塞前端展示，返回一个关闭态默认值
            tracing::warn!("[D1-v3.2-3.4.2-hook] 钩子状态查询失败: {}", e);
            Ok(ApiResponse::success(YuanCodeMonitorStatus {
                enabled: false,
                recent_event_count: 0,
                recent_cloud_api_calls: 0,
                recent_agent_executions: 0,
            }))
        }
    }
}

/// 查询监测数据汇总（UI 展示用：按 operation 分组 + 每日趋势）
#[tauri::command]
pub async fn yuan_code_monitor_summary(
    state: State<'_, AppState>,
) -> Result<ApiResponse<YuanCodeMonitorSummary>, String> {
    let user_id = require_auth(&state).await?;
    match yuan_code_monitor::get_summary(&state.pool, &state.intelligence_service, user_id).await {
        Ok(summary) => Ok(ApiResponse::success(summary)),
        Err(e) => {
            tracing::warn!("[D1-v3.2-3.4.2-hook] 钩子汇总查询失败: {}", e);
            Ok(ApiResponse::success(YuanCodeMonitorSummary {
                enabled: false,
                total_events: 0,
                by_operation: Vec::new(),
                daily_counts: Vec::new(),
            }))
        }
    }
}
