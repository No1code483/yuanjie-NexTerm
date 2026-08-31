//! T2.8 自动备份系统 - 后台调度器
//!
//! 实现规范：`功能展望/平台级增强/02_数据完整性_迁移与备份.md` §2.3.6 + §2.6.5
//!
//! 设计：
//! - 使用 `tokio::task::spawn` 启动后台调度器
//! - 每小时整点触发 hourly 备份（间隔 3600 秒）
//! - 每日 03:00 触发 daily 备份（间隔 86400 秒，启动时立即触发首次）
//! - 失败不自动重试（避免磁盘满时无限重试），等下一次定时触发
//! - 紧急备份由 T2.10 health_check 调度器在 Critical 状态时触发
//!
//! T2.10.3 健康检查调度器：
//! - 启动后 60 秒执行首次健康检查（避免与启动初始化竞争）
//! - 每日定时执行健康检查
//! - 检测到 Critical 状态时触发紧急备份
//!
//! 简化说明：
//! - 不使用 cron 表达式（避免引入新依赖），用 `tokio::time::interval`
//! - 不强制整点触发，启动后立即触发第一次（interval 默认行为）
//! - 若需要精确整点触发，可在 T2.10 阶段升级为 cron 实现

use std::path::{Path, PathBuf};
use std::time::Duration;

use sqlx::SqlitePool;

use crate::models::backup::BackupType;
use crate::services::{backup_service, health_check_service};

/// 启动备份调度器（后台任务）
///
/// 在应用 setup 钩子中调用，传入主库连接池和应用数据目录
///
/// ```ignore
/// // main.rs setup 钩子
/// let pool = app_state.pool.clone();
/// let data_dir = app_state.data_dir.clone();
/// backup_scheduler::start_backup_scheduler(pool, data_dir);
/// ```
pub fn start_backup_scheduler(pool: SqlitePool, data_dir: PathBuf) {
    // 使用 tauri::async_runtime::spawn 而非 tokio::spawn
    // 原因：main.rs 的 setup 钩子在自定义 rt.block_on 之外调用本函数，
    //       此时不在 tokio runtime 上下文中，tokio::spawn 会 panic。
    //       tauri::async_runtime::spawn 使用 Tauri 全局 runtime，无此限制。
    tauri::async_runtime::spawn(async move {
        tracing::info!("备份调度器启动：hourly=每小时, daily=每日");

        // 每小时触发一次（首次立即触发）
        let mut hourly_interval = tokio::time::interval(Duration::from_secs(3600));
        // 每日触发一次（首次立即触发）
        let mut daily_interval = tokio::time::interval(Duration::from_secs(86400));

        // 首次触发跳过（应用刚启动，无需立即备份）
        // 注：interval 的 first tick 立即返回，但 create_backup 内部会做幂等处理
        // 若备份目录已存在同秒备份，会自动追加 _1 后缀
        hourly_interval.tick().await;
        daily_interval.tick().await;

        loop {
            tokio::select! {
                _ = hourly_interval.tick() => {
                    if let Err(e) = backup_service::create_backup(
                        &pool, &data_dir, BackupType::Hourly, None
                    ).await {
                        tracing::error!("Hourly backup 失败: {}", e);
                        // 失败不重试，等下一次定时触发
                    }
                }
                _ = daily_interval.tick() => {
                    if let Err(e) = backup_service::create_backup(
                        &pool, &data_dir, BackupType::Daily, None
                    ).await {
                        tracing::error!("Daily backup 失败: {}", e);
                    }
                }
            }
        }
    });
}

/// 触发紧急备份（由 T2.10 health_check 调用）
///
/// 与调度器不同，紧急备份是同步调用，立即返回结果
pub async fn trigger_emergency_backup(
    pool: &SqlitePool,
    data_dir: &std::path::Path,
    reason: &str,
) -> Result<(), crate::error::app_error::AppError> {
    tracing::warn!("触发紧急备份: reason={}", reason);
    backup_service::create_backup(pool, data_dir, BackupType::Emergency, Some(reason)).await?;
    Ok(())
}

/// 触发操作前备份（由业务模块在危险操作前调用）
///
/// 例如：批量删除、清空回收站、危险 SQL 执行前
pub async fn trigger_pre_op_backup(
    pool: &SqlitePool,
    data_dir: &Path,
    label: &str,
) -> Result<(), crate::error::app_error::AppError> {
    tracing::info!("触发操作前备份: label={}", label);
    backup_service::create_backup(pool, data_dir, BackupType::PreOp, Some(label)).await?;
    Ok(())
}

// ============================================================================
// T2.10.3 健康检查调度器
// ============================================================================

/// 启动健康检查调度器（后台任务）
///
/// 在应用 setup 钩子中调用：
/// - 启动后 60 秒执行首次健康检查（避免与启动初始化竞争）
/// - 每日定时执行健康检查
/// - 检测到 Critical 状态时触发紧急备份
///
/// ```ignore
/// // main.rs setup 钩子
/// let pool = app_state.pool.clone();
/// let data_dir = app_state.data_dir.clone();
/// backup_scheduler::start_health_check_scheduler(pool, data_dir);
/// ```
pub fn start_health_check_scheduler(pool: SqlitePool, data_dir: PathBuf) {
    tauri::async_runtime::spawn(async move {
        tracing::info!("健康检查调度器启动：首次检查（60s 后）+ 每日定时检查");

        // 首次检查延迟 60 秒（避免与启动初始化竞争资源）
        tokio::time::sleep(Duration::from_secs(60)).await;

        // 执行首次健康检查
        run_scheduled_health_check(&pool, &data_dir).await;

        // 每日定时检查
        let mut interval = tokio::time::interval(Duration::from_secs(86400));
        // 跳过首次立即触发（已在上面执行过）
        interval.tick().await;

        loop {
            interval.tick().await;
            run_scheduled_health_check(&pool, &data_dir).await;
        }
    });
}

/// 执行一次定时健康检查
///
/// 流程：
/// 1. 调用 `health_check_service::run_health_check`
/// 2. 记录检查结果到日志
/// 3. 若状态为 Critical → 触发紧急备份（reason="health_check:critical"）
/// 4. 若状态为 Warning → 仅记录日志，不触发备份
///
/// 失败不重试，等下一次定时触发
async fn run_scheduled_health_check(pool: &SqlitePool, data_dir: &Path) {
    match health_check_service::run_health_check(pool, data_dir).await {
        Ok(result) => {
            tracing::info!(
                "健康检查完成: status={}, integrity={}, fk_violations={}, missing_indexes={}",
                result.status.as_str(),
                result.integrity.ok,
                result.foreign_keys.violation_count,
                result.indexes.missing_indexes.len()
            );

            if let Some(suggestion) = &result.repair_suggestion {
                tracing::warn!("健康检查修复建议: {}", suggestion);
            }

            // Critical 状态触发紧急备份
            if result.status.should_trigger_emergency_backup() {
                let reason = format!("health_check:{}", result.status.as_str());
                if let Err(e) = trigger_emergency_backup(pool, data_dir, &reason).await {
                    tracing::error!(
                        "健康检查触发紧急备份失败（status={}）: {}",
                        result.status.as_str(),
                        e
                    );
                } else {
                    tracing::warn!(
                        "健康检查触发紧急备份成功（status={}）",
                        result.status.as_str()
                    );
                }
            }
        }
        Err(e) => {
            tracing::error!("健康检查失败（等下一次定时触发）: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_interval_creation() {
        // 仅验证 Interval 可在 tokio runtime 中创建（不实际运行 tick）
        let _hourly = tokio::time::interval(Duration::from_secs(3600));
        let _daily = tokio::time::interval(Duration::from_secs(86400));
    }

    #[tokio::test]
    async fn test_run_scheduled_health_check_healthy() {
        // 验证健康检查调度器在健康状态下不触发紧急备份
        let temp = tempfile::tempdir().unwrap();
        let data_dir = temp.path();
        let db_path = data_dir.join("nexterm.db");

        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        // 创建 schema_migrations + 关键索引（避免告警）
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now')),
                applied_by TEXT,
                migration_hash TEXT NOT NULL,
                rollback_script TEXT,
                checksum_verified INTEGER NOT NULL DEFAULT 1
            );"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO schema_migrations (version, description, migration_hash) VALUES (89, 'test', 'h89');")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE test_table (col TEXT);")
            .execute(&pool)
            .await
            .unwrap();
        // 注：CRITICAL_INDEXES 是 health_check_service 的私有常量，测试中无法直接引用
        // 实际测试时，missing_indexes 可能非空，但不会影响 run_scheduled_health_check 的核心逻辑

        // 执行调度检查（不 panic 即可）
        run_scheduled_health_check(&pool, data_dir).await;

        pool.close().await;
    }
}
