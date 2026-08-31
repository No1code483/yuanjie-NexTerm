//! spec ai-chat-enhancement Phase 1 §1.2: 模型健康检测后台服务
//!
//! 设计文档：.trae/specs/ai-chat-enhancement/spec.md
//!
//! ## 职责
//!
//! - 周期性（默认 15 分钟）批量检测所有 `ai_models` 的连通性
//! - 调用 `chat_service::check_model_health` 复用单模型检测逻辑（含 DB 写回）
//! - 通过 Tauri 事件 `ai-model-health-changed` 广播状态变化给前端
//! - 内存维护连续失败计数器，连续 3 次失败将状态升级为 `error`
//! - 支持运行时调整检测间隔（`set_interval`）与优雅停止（`stop`）
//!
//! ## 与 connectivity.rs 的差异
//!
//! - connectivity 探测网络（一个端点）；本模块探测多个 AI 模型（并发限制 5）
//!- connectivity 状态用 bool；本模块用 5 种状态字符串（online/offline/checking/error/unknown）
//! - connectivity 失败立即标记离线；本模块连续 3 次失败才升级到 error（避免抖动）
//!
//! ## 为何使用 `tauri::async_runtime::spawn` 而非 `tokio::spawn`
//!
//! 与 `services/sync/connectivity.rs::start_background_task` 同原因：
//! 本方法在 `main.rs` 的 `setup` 闭包中**同步**调用（不在 tokio 运行时上下文中），
//! 直接 `tokio::spawn` 会 panic。`tauri::async_runtime::spawn` 走 Tauri 全局运行时，
//! 可在同步上下文中安全调用，与 `backup_scheduler` / `mek_rotation_scheduler` /
//! `connectivity_checker` 模式保持一致。

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::RwLock;

use crate::crypto::mek_manager::MekManager;
use crate::db::connection::AppState;
use crate::db::repositories::ai_repo;
use crate::models::chat::ModelHealthStatus;
use crate::services::chat_service;

/// 后台检测周期默认值（秒）= 15 分钟
pub const DEFAULT_INTERVAL_SECS: u64 = 900;

/// 连续失败多少次后升级状态为 `error`
pub const FAILURE_THRESHOLD: u8 = 3;

/// 并发检测上限（避免对 API 服务商造成突发压力）
pub const MAX_CONCURRENCY: usize = 5;

/// 前端监听的事件名（`listen('ai-model-health-changed', ...)`）
pub const HEALTH_CHANGED_EVENT: &str = "ai-model-health-changed";

/// 系统级 user_id（用于未登录场景下解密 API Key 的 MEK 查询）
///
/// 注：本应用当前为单用户模式，user_id = 1 为系统默认用户。
/// 与 `chat_commands::send_message` 中 `state.current_user.read().await` 不同，
/// 后台任务启动时用户可能尚未登录，因此使用固定系统用户 ID 兜底。
pub const SYSTEM_USER_ID: i64 = 1;

/// 模型健康检测后台服务
///
/// 状态字段均以 `Arc<RwLock<...>>` / `Arc<AtomicBool>` 持有，可通过 `Arc::clone`
/// 在后台任务与命令处理器之间共享。结构体本身 `Clone` 廉价（仅克隆 Arc 指针）。
///
/// 生命周期：
/// - `main.rs` setup 闭包中 `AppState::new` 创建实例
/// - `start_background_task` 在 Tauri 全局运行时启动后台循环
/// - 应用退出时 `stop` 请求任务在下个 sleep 周期退出
#[derive(Clone)]
pub struct ModelHealthMonitor {
    stop_flag: Arc<AtomicBool>,
    interval_secs: Arc<RwLock<u64>>,
    /// model_id → 连续失败次数（内存态，重启清零）
    failure_counts: Arc<RwLock<HashMap<i64, u8>>>,
}

impl ModelHealthMonitor {
    pub fn new() -> Self {
        Self {
            stop_flag: Arc::new(AtomicBool::new(false)),
            interval_secs: Arc::new(RwLock::new(DEFAULT_INTERVAL_SECS)),
            failure_counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 启动后台周期检测任务（立即返回，任务在 Tauri 全局异步运行时中运行）
    ///
    /// - 启动后立即执行首次检测（避免首次需等待一个 interval）
    /// - 之后每 `interval_secs` 秒检测一次
    /// - 调用 `stop` 后任务在下个 sleep 周期退出
    /// - sleep 期间每秒检查 stop_flag，确保停止信号响应延迟 ≤ 1s
    pub fn start_background_task(
        &self,
        app_handle: AppHandle,
        pool: SqlitePool,
        mek_manager: Arc<RwLock<MekManager>>,
        user_id: i64,
    ) {
        let stop_flag = self.stop_flag.clone();
        let interval_secs = self.interval_secs.clone();
        let failure_counts = self.failure_counts.clone();

        tauri::async_runtime::spawn(async move {
            tracing::info!(
                "[model_health] 后台检测任务启动（默认间隔 {}s，并发上限 {}）",
                DEFAULT_INTERVAL_SECS, MAX_CONCURRENCY
            );

            // 启动后立即检测一次
            Self::run_check_once(
                &app_handle, &pool, &mek_manager, user_id, &failure_counts,
            ).await;

            loop {
                if stop_flag.load(Ordering::SeqCst) {
                    tracing::info!("[model_health] 后台检测任务已停止");
                    break;
                }

                // 分段 sleep 以便及时响应停止信号（每秒检查一次）
                let interval = *interval_secs.read().await;
                let mut slept = 0u64;
                while slept < interval {
                    if stop_flag.load(Ordering::SeqCst) {
                        break;
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    slept += 1;
                }

                if stop_flag.load(Ordering::SeqCst) {
                    tracing::info!("[model_health] 后台检测任务已停止");
                    break;
                }

                // 用户可能已登录，优先用 current_user；未登录则继续用启动时传入的 user_id
                let effective_uid = match app_handle.try_state::<AppState>() {
                    Some(state) => {
                        let user = state.current_user.read().await;
                        user.unwrap_or(user_id)
                    }
                    None => user_id,
                };

                Self::run_check_once(
                    &app_handle, &pool, &mek_manager, effective_uid, &failure_counts,
                ).await;
            }
        });
    }

    /// 执行一次全量检测：查询所有模型 → 并发检测（buffer_unordered 限制 5）→ 写 DB → emit 事件
    ///
    /// 设计要点：
    /// - `chat_service::check_model_health` 已在检测前标记 `status='checking'`、检测后写
    ///   `online`/`offline` + `latency_ms`。本函数仅做"批量调度 + 失败升级 + 事件广播"。
    /// - 失败计数器在内存维护：成功重置为 0，失败累加；达到阈值（3）时把 DB 状态升级为 `error`。
    /// - 并发用 `futures::stream::iter(...).buffer_unordered(5)`，避免对 API 服务商突发压力。
    pub async fn run_check_once(
        app_handle: &AppHandle,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        failure_counts: &Arc<RwLock<HashMap<i64, u8>>>,
    ) {
        let models = match ai_repo::get_all_models(pool, user_id).await {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("[model_health] 查询所有模型失败: {}", e);
                return;
            }
        };

        if models.is_empty() {
            return;
        }

        // 并发检测，限制并发数 = MAX_CONCURRENCY
        use futures::stream::{iter, StreamExt};
        let results: Vec<(i64, Result<ModelHealthStatus, crate::error::app_error::AppError>)> =
            iter(models.into_iter().map(|m| {
                let pool = pool.clone();
                let mek = mek_manager.clone();
                async move {
                    let result = chat_service::check_model_health(&pool, m.id, user_id, &mek).await;
                    (m.id, result)
                }
            }))
            .buffer_unordered(MAX_CONCURRENCY)
            .collect()
            .await;

        // 处理结果：失败计数 + 状态升级 + 事件广播
        let mut failures = failure_counts.write().await;
        let now = chrono::Utc::now().timestamp_millis();

        for (model_id, result) in results {
            let (status_str, latency_ms) = match &result {
                Ok(hs) if hs.is_online => {
                    // 检测成功：重置失败计数
                    failures.insert(model_id, 0);
                    ("online", hs.latency_ms)
                }
                Ok(_) => {
                    // 检测失败（is_online=false）：累加失败计数
                    let count = failures.entry(model_id).and_modify(|c| *c += 1).or_insert(1);
                    if *count >= FAILURE_THRESHOLD {
                        // 连续失败达阈值：升级 DB 状态为 error（覆盖 check_model_health 写的 offline）
                        let _ = ai_repo::update_model_health(
                            pool, model_id, "error", None, now,
                        ).await;
                        ("error", None)
                    } else {
                        ("offline", None)
                    }
                }
                Err(_) => {
                    // 检测异常：同样累加失败计数
                    let count = failures.entry(model_id).and_modify(|c| *c += 1).or_insert(1);
                    if *count >= FAILURE_THRESHOLD {
                        let _ = ai_repo::update_model_health(
                            pool, model_id, "error", None, now,
                        ).await;
                        ("error", None)
                    } else {
                        ("offline", None)
                    }
                }
            };

            // 广播状态变化给前端（不区分前后值是否变化，前端按 model_id 更新对应项即可）
            let _ = app_handle.emit(
                HEALTH_CHANGED_EVENT,
                serde_json::json!({
                    "model_id": model_id,
                    "status": status_str,
                    "latency_ms": latency_ms,
                    "last_health_check": now,
                }),
            );
        }
    }

    /// 设置检测间隔（分钟），下一次 sleep 周期生效
    pub async fn set_interval(&self, minutes: u64) {
        let secs = minutes.saturating_mul(60);
        *self.interval_secs.write().await = secs;
        tracing::info!("[model_health] 检测间隔已更新为 {} 分钟（{}s）", minutes, secs);
    }

    /// 请求后台检测任务停止（异步：任务在下个 sleep 周期退出）
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }
}

impl Default for ModelHealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}
