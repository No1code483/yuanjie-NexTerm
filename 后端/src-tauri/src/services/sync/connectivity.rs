//! A5 离线与同步机制 - Phase 3 Task 1 网络状态检测层
//!
//! 设计文档：功能展望/平台级增强/04_离线与同步机制.md §2.4.1（网络恢复触发）
//!
//! ## 职责
//!
//! - 周期性（30s 间隔）探测网络可达性，缓存 is_online + last_checked
//! - 提供 `check_now()` 立即触发探测
//! - 状态变更时通过 Tauri 事件 `network-status-changed` 广播给前端
//! - 后台任务支持优雅退出（AtomicBool 停止标志）
//!
//! ## 探测策略
//!
//! - 端点：`https://www.baidu.com`（国内可达，避免 google 204 端点在 CN 网络误判为离线）
//! - 方法：HEAD（减少流量；百度对 HEAD 返回 200）
//! - 超时：3 秒
//! - HTTP 2xx/3xx 均视为在线（部分 CDN 会返回 30x 重定向）
//!
//! ## 与同步调度器的关系
//!
//! 本模块仅负责"网络可达性"判定，不直接驱动同步。
//! 前端监听 `network-status-changed` 事件后，可在 `is_online: false → true` 时
//! 主动调用 `sync_run_once` 命令触发同步（符合 scheduler.rs §2.4.1 设计）。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

use crate::error::app_error::AppError;

/// 网络探测端点（国内可达，避免 google 204 端点的 CN 网络误判）
pub const PROBE_URL: &str = "https://www.baidu.com";

/// 单次探测超时（秒）
pub const PROBE_TIMEOUT_SECS: u64 = 3;

/// 周期检测间隔（秒）
pub const CHECK_INTERVAL_SECS: u64 = 30;

/// 网络状态变更事件名（前端通过 `listen('network-status-changed', ...)` 订阅）
pub const NETWORK_STATUS_EVENT: &str = "network-status-changed";

/// 网络状态快照
///
/// 通过 IPC 命令 `sync_get_network_status` / `sync_check_network_now` 返回前端，
/// 同时作为 `network-status-changed` 事件的 payload。
#[derive(Debug, Clone, Serialize)]
pub struct NetworkStatus {
    /// 当前是否在线
    pub is_online: bool,
    /// 最近一次检测的 Unix 时间戳（秒）
    pub last_checked: i64,
}

/// 网络连通性检测器
///
/// 状态以 `Arc<RwLock<...>>` 形式持有，可通过 `Arc::clone` 在多个任务间共享。
/// 后台周期检测任务由 [`start_background_task`] 启动，由 [`stop`] 优雅退出。
///
/// Phase 3 Task 5：可选持有 `SqlitePool`，在检测到 `is_online: false → true` 时
/// 自动触发 `config_sync_service::flush_config_queue`，将 failed 的配置条目重置为 pending。
/// 池为 `None` 时（如单元测试）跳过 flush，不影响检测逻辑。
#[derive(Clone)]
pub struct ConnectivityChecker {
    is_online: Arc<RwLock<bool>>,
    last_checked: Arc<RwLock<i64>>,
    stop_flag: Arc<AtomicBool>,
    /// Phase 3 Task 5：网络恢复时自动 flush 配置同步队列（可选）
    pool: Option<SqlitePool>,
}

impl ConnectivityChecker {
    /// 创建检测器实例（初始状态：离线，未检测过）
    ///
    /// 不持有 DbPool，网络恢复时不会自动 flush 配置队列。
    /// 适用于单元测试或不需配置同步降级的场景。
    pub fn new() -> Self {
        Self {
            is_online: Arc::new(RwLock::new(false)),
            last_checked: Arc::new(RwLock::new(0)),
            stop_flag: Arc::new(AtomicBool::new(false)),
            pool: None,
        }
    }

    /// 创建带 DbPool 的检测器实例
    ///
    /// 网络恢复（`is_online: false → true`）时自动 spawn 后台任务调用
    /// `config_sync_service::flush_config_queue`，将 failed 的配置条目重置为 pending。
    /// 生产环境（AppState::new）应使用此构造函数。
    pub fn with_pool(pool: SqlitePool) -> Self {
        Self {
            is_online: Arc::new(RwLock::new(false)),
            last_checked: Arc::new(RwLock::new(0)),
            stop_flag: Arc::new(AtomicBool::new(false)),
            pool: Some(pool),
        }
    }

    /// 获取当前缓存的网络状态（不触发探测）
    pub async fn get_status(&self) -> NetworkStatus {
        NetworkStatus {
            is_online: *self.is_online.read().await,
            last_checked: *self.last_checked.read().await,
        }
    }

    /// 执行一次 HTTP HEAD 探测，返回是否在线
    ///
    /// - 2xx / 3xx 响应视为在线
    /// - 任何网络错误或 4xx/5xx 视为离线
    async fn probe_once() -> bool {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(PROBE_TIMEOUT_SECS))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("[connectivity] reqwest client 构建失败: {}", e);
                return false;
            }
        };

        match client.head(PROBE_URL).send().await {
            Ok(resp) => {
                let status = resp.status();
                status.is_success() || status.is_redirection()
            }
            Err(e) => {
                tracing::debug!("[connectivity] 探测失败: {}", e);
                false
            }
        }
    }

    /// 内部：更新缓存状态，若状态有变化则返回 Some(NetworkStatus) 供调用方广播
    async fn update_status(&self, is_online: bool) -> Option<NetworkStatus> {
        let mut current = self.is_online.write().await;
        let changed = *current != is_online;
        if changed {
            *current = is_online;
        }
        drop(current);

        let now = chrono::Utc::now().timestamp();
        *self.last_checked.write().await = now;

        if changed {
            Some(NetworkStatus {
                is_online,
                last_checked: now,
            })
        } else {
            None
        }
    }

    /// 立即触发一次探测，并在状态变更时通过 Tauri 事件广播
    ///
    /// 此方法同时被 [`start_background_task`] 周期调用与 IPC 命令
    /// `sync_check_network_now` 复用，保证两条路径的事件广播逻辑一致。
    ///
    /// Phase 3 Task 5：当 `is_online` 从 `false → true` 时，若检测器持有 DbPool，
    /// 自动 spawn 后台任务调用 `config_sync_service::flush_config_queue`，
    /// 将 failed 的配置条目重置为 pending，等待 scheduler 同步。
    pub async fn check_now(&self, app_handle: &AppHandle) -> NetworkStatus {
        let is_online = Self::probe_once().await;

        if let Some(status) = self.update_status(is_online).await {
            if is_online {
                tracing::info!("[connectivity] 网络已恢复在线");
                // Phase 3 Task 5：网络恢复时自动 flush 配置同步队列
                if let Some(pool) = &self.pool {
                    let pool = pool.clone();
                    tokio::spawn(async move {
                        match crate::services::config_sync_service::flush_config_queue(&pool)
                            .await
                        {
                            Ok(n) => {
                                if n > 0 {
                                    tracing::info!(
                                        "[connectivity] 已 flush {} 条 failed 配置变更到 pending",
                                        n
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "[connectivity] flush 配置队列失败: {}",
                                    e
                                );
                            }
                        }
                    });
                }
            } else {
                tracing::warn!("[connectivity] 网络已断开");
            }
            let _ = app_handle.emit(NETWORK_STATUS_EVENT, status.clone());
        }

        self.get_status().await
    }

    /// 启动后台周期检测任务（立即返回，任务在 Tauri 全局异步运行时中运行）
    ///
    /// - 启动后立即执行首次探测（避免首次检测需等待一个 interval）
    /// - 之后每 `CHECK_INTERVAL_SECS` 秒探测一次
    /// - 调用 [`stop`] 后任务在下个 sleep 周期退出
    ///
    /// ## 为何使用 `tauri::async_runtime::spawn` 而非 `tokio::spawn`
    ///
    /// 本方法在 `main.rs` 的 `setup` 闭包中**同步**调用（不在任何 tokio 运行时上下文里），
    /// 直接 `tokio::spawn` 会 panic：`there is no reactor running, must be called from
    /// the context of a Tokio 1.x runtime`。`tauri::async_runtime::spawn` 使用 Tauri 管理
    /// 的全局运行时，可在同步上下文中安全调用，与 `backup_scheduler` / `mek_rotation_scheduler`
    /// 的模式保持一致。Bug 修复：v1.52.19.1 之前的 `tokio::spawn` 会导致 Tauri 启动 panic。
    pub fn start_background_task(&self, app_handle: AppHandle) {
        let checker = self.clone();
        let stop_flag = self.stop_flag.clone();

        tauri::async_runtime::spawn(async move {
            tracing::info!(
                "[connectivity] 后台检测任务启动（间隔 {}s，端点 {}）",
                CHECK_INTERVAL_SECS,
                PROBE_URL
            );

            // 启动后立即探测一次
            checker.check_now(&app_handle).await;

            loop {
                if stop_flag.load(Ordering::SeqCst) {
                    tracing::info!("[connectivity] 后台检测任务已停止");
                    break;
                }

                // 分段 sleep 以便及时响应停止信号（每秒检查一次）
                let mut slept = 0u64;
                while slept < CHECK_INTERVAL_SECS {
                    if stop_flag.load(Ordering::SeqCst) {
                        break;
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    slept += 1;
                }

                if stop_flag.load(Ordering::SeqCst) {
                    tracing::info!("[connectivity] 后台检测任务已停止");
                    break;
                }

                checker.check_now(&app_handle).await;
            }
        });
    }

    /// 请求后台检测任务停止（异步：任务在下个 sleep 周期退出）
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }
}

impl Default for ConnectivityChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// 构建带超时的 reqwest 客户端（公共工具，供测试或扩展使用）
#[allow(dead_code)]
fn build_probe_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(PROBE_TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::Internal(format!("reqwest client 构建失败: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_checker_starts_offline() {
        let checker = ConnectivityChecker::new();
        let status = checker.get_status().await;
        assert!(!status.is_online);
        assert_eq!(status.last_checked, 0);
    }

    #[tokio::test]
    async fn test_update_status_detects_change() {
        let checker = ConnectivityChecker::new();

        // false → true：应触发变更
        let changed = checker.update_status(true).await;
        assert!(changed.is_some());
        assert_eq!(changed.unwrap().is_online, true);

        // true → true：不应触发变更
        let changed = checker.update_status(true).await;
        assert!(changed.is_none());

        // true → false：应触发变更
        let changed = checker.update_status(false).await;
        assert!(changed.is_some());
        assert_eq!(changed.unwrap().is_online, false);

        // last_checked 应被更新（>0）
        let status = checker.get_status().await;
        assert!(status.last_checked > 0);
    }

    #[tokio::test]
    async fn test_stop_flag_set() {
        let checker = ConnectivityChecker::new();
        assert!(!checker.stop_flag.load(Ordering::SeqCst));
        checker.stop();
        assert!(checker.stop_flag.load(Ordering::SeqCst));
    }

    #[test]
    fn test_network_status_serialization() {
        let status = NetworkStatus {
            is_online: true,
            last_checked: 1_700_000_000,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"is_online\":true"));
        assert!(json.contains("\"last_checked\":1700000000"));
    }
}
