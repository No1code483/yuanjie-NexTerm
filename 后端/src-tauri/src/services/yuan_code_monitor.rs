//! Yuan Code v3.2 Task 3.4.2 — 底层智能（恐龙双脑）非侵入式监测接入
//!
//! 设计依据：
//! - .trae/rules/项目核心设计意图.md §二（底层智能渗透所有模块，非侵入式，可关闭）
//! - .trae/rules/项目核心设计意图.md §8.2（底层智能必须支持一键开关，关闭后各模块核心功能仍可用）
//! - .trae/rules/项目核心设计意图.md §三（Yuan Code 编程 AI 必须走云端 API；底层智能仅监测，不替代编程 AI）
//!
//! 核心能力：
//! - record_event()：将 Yuan Code 使用事件写入底层智能 activity_logs 表（module='yuan_code'），
//!   供底层智能的行为分析/建议引擎消费。非侵入式：Yuan Code 核心流程不依赖此钩子。
//! - get_status()：查询钩子是否启用 + 近期 Yuan Code 事件数。
//! - get_summary()：汇总监测数据（调用频率 / 成功率 / 用户反馈），供 UI 展示。
//!
//! 边界（严格遵守项目核心设计意图 §二、§三、§8.2）：
//! - 此钩子仅"喂数据给底层智能监测"，不替代 Yuan Code 的云端 API 调用
//! - 底层智能关闭（is_enabled() == false）时，钩子完全 no-op，但 Yuan Code 核心功能仍可用
//! - 非侵入式：Yuan Code 服务（model_routing_service / cloud_api_router 等）不依赖此钩子即可工作；
//!   钩子失败不影响 Yuan Code 流程（调用方忽略错误）
//!
//! 监测的事件类型：
//! - YuanCodeEventType::RouteResolved：路由规则解析（task_type → provider+model）
//! - YuanCodeEventType::CloudApiCall：云端 API 调用（成功/失败/延迟）
//! - YuanCodeEventType::AgentExecute：Agent 执行（7 种类型）
//! - YuanCodeEventType::UserFeedback：用户反馈（采纳/拒绝/编辑）

use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::services::intelligence_service::IntelligenceService;

/// Yuan Code 使用事件类型（写入 activity_logs.operation）
#[derive(Debug, Clone)]
pub enum YuanCodeEventType {
    /// 路由规则解析（task_type → provider+model）
    RouteResolved,
    /// 云端 API 调用（成功/失败/延迟）
    CloudApiCall,
    /// Agent 执行（7 种类型：coding/refactor/test/documentation/debug/migration/review）
    AgentExecute,
    /// 用户反馈（采纳/拒绝/编辑）
    UserFeedback,
    /// 代码补全调用
    Completion,
    /// 代码分析调用
    Analysis,
    Custom(String),
}

impl YuanCodeEventType {
    fn as_str(&self) -> &str {
        match self {
            YuanCodeEventType::RouteResolved => "yuan_code_route_resolved",
            YuanCodeEventType::CloudApiCall => "yuan_code_cloud_api_call",
            YuanCodeEventType::AgentExecute => "yuan_code_agent_execute",
            YuanCodeEventType::UserFeedback => "yuan_code_user_feedback",
            YuanCodeEventType::Completion => "yuan_code_completion",
            YuanCodeEventType::Analysis => "yuan_code_analysis",
            YuanCodeEventType::Custom(s) => s.as_str(),
        }
    }
}

/// Yuan Code 事件监测状态
#[derive(Debug, Clone, serde::Serialize)]
pub struct YuanCodeMonitorStatus {
    /// 底层智能全局开关是否启用（关闭时本钩子完全 no-op）
    pub enabled: bool,
    /// 近 7 天 Yuan Code 事件数（module='yuan_code'）
    pub recent_event_count: i64,
    /// 近 7 天云端 API 调用次数（operation='yuan_code_cloud_api_call'）
    pub recent_cloud_api_calls: i64,
    /// 近 7 天 Agent 执行次数（operation='yuan_code_agent_execute'）
    pub recent_agent_executions: i64,
}

/// Yuan Code 监测数据汇总（UI 展示用）
#[derive(Debug, Clone, serde::Serialize)]
pub struct YuanCodeMonitorSummary {
    pub enabled: bool,
    /// 总事件数
    pub total_events: i64,
    /// 按 operation 分组的事件数
    pub by_operation: Vec<OperationCount>,
    /// 最近 7 天每日事件数（按日期字符串聚合）
    pub daily_counts: Vec<DailyCount>,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct OperationCount {
    pub operation: String,
    pub count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DailyCount {
    pub date: String,
    pub count: i64,
}

/// 记录一条 Yuan Code 事件供底层智能监测消费（非侵入式 + 可关闭）
///
/// 非侵入式 + 可关闭：
/// - 底层智能关闭时立即返回 Ok(())，不写入任何数据
/// - 写入失败仅记录日志，不向上抛错（Yuan Code 流程不应受监测钩子影响）
///
/// 典型调用场景：
/// - model_routing_service 解析路由后调用 record_event(RouteResolved, ...)
/// - cloud_api_router 调用云端 API 后调用 record_event(CloudApiCall, ...)
/// - yuan_agent_commands 执行 Agent 后调用 record_event(AgentExecute, ...)
pub async fn record_event(
    pool: &SqlitePool,
    intelligence: &IntelligenceService,
    user_id: i64,
    event_type: YuanCodeEventType,
    detail: Option<&str>,
) -> Result<(), AppError> {
    // 可关闭性：底层智能关闭时，钩子完全 no-op
    if !intelligence.is_enabled().await {
        return Ok(());
    }

    let user_id_str = user_id.to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();

    let res = sqlx::query(
        r#"INSERT INTO activity_logs (user_id, timestamp, module, operation, detail, duration_secs)
           VALUES (?, ?, 'yuan_code', ?, ?, 0)"#,
    )
    .bind(&user_id_str)
    .bind(&timestamp)
    .bind(event_type.as_str())
    .bind(detail)
    .execute(pool)
    .await;

    if let Err(e) = res {
        // 非侵入式：监测写入失败不影响 Yuan Code 流程，仅记录告警
        tracing::warn!(
            "[D1-v3.2-3.4.2-hook] Yuan Code 事件写入底层智能 activity_logs 失败（已忽略，不影响 Yuan Code）: {}",
            e
        );
    }

    Ok(())
}

/// 查询钩子状态（启用与否 + 近期事件数）
pub async fn get_status(
    pool: &SqlitePool,
    intelligence: &IntelligenceService,
    user_id: i64,
) -> Result<YuanCodeMonitorStatus, AppError> {
    let enabled = intelligence.is_enabled().await;

    let user_id_str = user_id.to_string();
    let cutoff = {
        let now = chrono::Utc::now();
        let seven_days_ago = now - chrono::Duration::days(7);
        seven_days_ago.to_rfc3339()
    };

    let recent_event_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM activity_logs
           WHERE user_id = ? AND module = 'yuan_code' AND timestamp >= ?"#,
    )
    .bind(&user_id_str)
    .bind(&cutoff)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let recent_cloud_api_calls: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM activity_logs
           WHERE user_id = ? AND module = 'yuan_code' AND operation = 'yuan_code_cloud_api_call' AND timestamp >= ?"#,
    )
    .bind(&user_id_str)
    .bind(&cutoff)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let recent_agent_executions: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM activity_logs
           WHERE user_id = ? AND module = 'yuan_code' AND operation = 'yuan_code_agent_execute' AND timestamp >= ?"#,
    )
    .bind(&user_id_str)
    .bind(&cutoff)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    Ok(YuanCodeMonitorStatus {
        enabled,
        recent_event_count,
        recent_cloud_api_calls,
        recent_agent_executions,
    })
}

/// 查询监测数据汇总（UI 展示用）
pub async fn get_summary(
    pool: &SqlitePool,
    intelligence: &IntelligenceService,
    user_id: i64,
) -> Result<YuanCodeMonitorSummary, AppError> {
    let enabled = intelligence.is_enabled().await;
    let user_id_str = user_id.to_string();
    let cutoff = {
        let now = chrono::Utc::now();
        let seven_days_ago = now - chrono::Duration::days(7);
        seven_days_ago.to_rfc3339()
    };

    let total_events: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM activity_logs
           WHERE user_id = ? AND module = 'yuan_code' AND timestamp >= ?"#,
    )
    .bind(&user_id_str)
    .bind(&cutoff)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 按 operation 分组
    let by_operation: Vec<OperationCount> = sqlx::query_as(
        r#"SELECT operation, COUNT(*) as count FROM activity_logs
           WHERE user_id = ? AND module = 'yuan_code' AND timestamp >= ?
           GROUP BY operation ORDER BY count DESC"#,
    )
    .bind(&user_id_str)
    .bind(&cutoff)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|_| Vec::new());

    // 按日期分组（取 date 部分）
    let daily_rows: Vec<(String, i64)> = sqlx::query_as(
        r#"SELECT DATE(timestamp) as date, COUNT(*) as count FROM activity_logs
           WHERE user_id = ? AND module = 'yuan_code' AND timestamp >= ?
           GROUP BY DATE(timestamp) ORDER BY date ASC"#,
    )
    .bind(&user_id_str)
    .bind(&cutoff)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|_| Vec::new());

    let daily_counts = daily_rows
        .into_iter()
        .map(|(date, count)| DailyCount { date, count })
        .collect();

    Ok(YuanCodeMonitorSummary {
        enabled,
        total_events,
        by_operation,
        daily_counts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_strings() {
        assert_eq!(YuanCodeEventType::RouteResolved.as_str(), "yuan_code_route_resolved");
        assert_eq!(YuanCodeEventType::CloudApiCall.as_str(), "yuan_code_cloud_api_call");
        assert_eq!(YuanCodeEventType::AgentExecute.as_str(), "yuan_code_agent_execute");
        assert_eq!(YuanCodeEventType::UserFeedback.as_str(), "yuan_code_user_feedback");
        assert_eq!(YuanCodeEventType::Completion.as_str(), "yuan_code_completion");
        assert_eq!(YuanCodeEventType::Analysis.as_str(), "yuan_code_analysis");
        assert_eq!(
            YuanCodeEventType::Custom("yuan_code_custom".into()).as_str(),
            "yuan_code_custom"
        );
    }
}
