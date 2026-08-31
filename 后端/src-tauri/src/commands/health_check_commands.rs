//! T2.10 异常检测与自愈 - Tauri 命令
//!
//! 实现规范：`功能展望/平台级增强/02_数据完整性_迁移与备份.md` §2.6
//!
//! 暴露的命令：
//! - `health_check_run`：手动触发健康检查（4 项检查：integrity / FK / table_counts / indexes）
//! - `health_check_repair`：手动触发自动修复（孤儿记录 → _orphaned 表 + 索引重建）
//!
//! 自动调度：
//! - 应用启动后 60 秒自动执行首次健康检查（由 `backup_scheduler::start_health_check_scheduler` 调度）
//! - 每日定时执行健康检查
//! - 检测到 Critical 状态时自动触发紧急备份
//! - 此处仅暴露手动触发的命令，供前端"设置 > 数据完整性"页面使用

use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::health_check::{AutoRepairResult, HealthCheckResult};
use crate::services::health_check_service;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

/// 手动触发健康检查
///
/// 执行 4 项检查：
/// 1. PRAGMA integrity_check（数据库文件物理完整性）
/// 2. PRAGMA foreign_key_check（外键约束违规）
/// 3. 关键表行数对比（与最近一次备份对比）
/// 4. 索引覆盖率（关键索引是否存在）
///
/// 返回：
/// - `HealthCheckResult`：包含整体状态、4 项检查详情、修复建议
///
/// 前端可根据 `status` 字段决定后续操作：
/// - `healthy`：显示绿色状态
/// - `warning`：显示黄色状态 + 修复建议
/// - `critical`：显示红色状态 + 紧急提示 + 建议立即修复或恢复备份
#[tauri::command]
pub async fn health_check_run(
    state: State<'_, AppState>,
) -> Result<ApiResponse<HealthCheckResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    match health_check_service::run_health_check(&state.pool, &state.data_dir).await {
        Ok(result) => {
            tracing::info!(
                "手动健康检查完成: status={}, fk_violations={}",
                result.status.as_str(),
                result.foreign_keys.violation_count
            );
            Ok(ApiResponse::success(result))
        }
        Err(e) => Err(e.into()),
    }
}

/// 手动触发自动修复
///
/// 修复策略：
/// 1. FK 违规 → 将孤儿记录从原表移动到 `<table>_orphaned` 表
/// 2. 索引损坏 → 执行 `REINDEX` 重建已存在的索引
/// 3. 索引缺失 → 不自动创建（需业务层提供索引定义）
/// 4. 表行数偏差 → 不修复（业务数据问题）
/// 5. integrity 失败 → 不修复（需恢复备份）
///
/// 修复后自动重新执行健康检查，返回修复后的状态
///
/// 流程：
/// 1. 执行健康检查获取当前状态
/// 2. 若状态为 Healthy → 直接返回（无需修复）
/// 3. 若状态为 Warning/Critical → 执行自动修复
/// 4. 修复后重新检查状态
#[tauri::command]
pub async fn health_check_repair(
    state: State<'_, AppState>,
) -> Result<ApiResponse<AutoRepairResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    // 1. 执行健康检查获取当前状态
    let check_result = health_check_service::run_health_check(&state.pool, &state.data_dir)
        .await
        .map_err(|e| -> String { e.into() })?;

    // 2. 若状态为 Healthy，直接返回无需修复
    if !check_result.status.needs_repair() {
        return Ok(ApiResponse::success(AutoRepairResult {
            success: false,
            repaired_items: 0,
            orphaned_records_moved: 0,
            indexes_rebuilt: 0,
            details: vec!["数据库状态健康，无需修复".to_string()],
            post_repair_status: Some(check_result.status),
        }));
    }

    // 3. 执行自动修复
    match health_check_service::auto_repair(&state.pool, &check_result, &state.data_dir).await {
        Ok(result) => {
            tracing::info!(
                "自动修复完成: success={}, repaired_items={}, orphaned_moved={}, indexes_rebuilt={}",
                result.success,
                result.repaired_items,
                result.orphaned_records_moved,
                result.indexes_rebuilt
            );
            Ok(ApiResponse::success(result))
        }
        Err(e) => Err(e.into()),
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use crate::error::app_error::AppError;
    use crate::models::health_check::HealthStatus;

    #[test]
    fn test_health_status_str() {
        assert_eq!(HealthStatus::Healthy.as_str(), "healthy");
        assert_eq!(HealthStatus::Warning.as_str(), "warning");
        assert_eq!(HealthStatus::Critical.as_str(), "critical");
    }

    #[test]
    fn test_app_error_backup_code() {
        let err = AppError::Backup("test".into());
        assert_eq!(err.error_code(), 7001);
    }
}
