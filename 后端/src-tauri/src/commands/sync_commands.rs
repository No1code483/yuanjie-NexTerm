//! A5 离线与同步机制 - IPC 命令
//!
//! 命令清单：
//! - sync_enqueue: 将变更记录入队
//! - sync_fetch_pending: 获取待同步队列
//! - sync_mark_synced: 标记为已同步
//! - sync_mark_failed: 标记为同步失败
//! - sync_get_queue_stats: 获取队列统计
//! - sync_register_device: 注册新设备
//! - sync_list_devices: 列出所有设备
//! - sync_unregister_device: 注销设备
//! - sync_test_transport: 测试传输后端连接（Phase 2 新增）
//! - sync_run_once: 执行一次同步（Phase 2 新增，触发退避重试）
//! - sync_get_status: 获取同步状态（Phase 2 新增）
//! - sync_list_conflicts: 获取冲突记录列表（Phase 2 新增）
//! - sync_resolve_conflict: 手动解决冲突（Phase 2 新增）
//! - sync_ecdh_generate_keypair: 生成 ECDH 密钥对（Phase 4 新增）
//! - sync_e2ee_encrypt: E2EE 加密（Phase 4 新增）
//! - sync_e2ee_decrypt: E2EE 解密（Phase 4 新增）
//! - sync_get_network_status: 获取当前网络状态（Phase 3 Task 1 新增）
//! - sync_check_network_now: 立即触发一次网络探测并返回（Phase 3 Task 1 新增）
//! - sync_record_config_change: 记录配置变更到 sync_queue（Phase 3 Task 5 新增）
//! - sync_flush_config_queue: 网络恢复时 flush 配置队列（Phase 3 Task 5 新增）
//! - sync_get_pending_config_count: 获取待发配置条数（Phase 3 Task 5 新增）

use tauri::{AppHandle, State};

use crate::crypto::ecdh::{EcdhKeyPair, SerializedKeyPair};
use crate::db::connection::AppState;
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;
use crate::services::config_sync_service;
use crate::services::sync::connectivity::NetworkStatus;
use crate::services::sync::e2ee;
use crate::services::sync::scheduler::SyncScheduler;
use crate::services::sync_service;
use crate::services::sync_service::SyncOperation;
use crate::services::sync_transport::TransportBackend;

// 安全审计修复（发现 13，MEDIUM）：原 sync_* 命令无认证，可被注入 IPC 删除设备、
// 操纵同步队列、泄露 E2EE 密钥材料、将 config_json 指向恶意 WebDAV/S3。
// 现每个命令入口强制调用 `require_auth(&state).await?`。
// 注：原本无 state 参数的命令（sync_test_transport / sync_ecdh_generate_keypair /
// sync_e2ee_encrypt / sync_e2ee_validate）新增 state 参数（Tauri 自动注入）。

/// 将变更记录入队
#[tauri::command]
pub async fn sync_enqueue(
    state: State<'_, AppState>,
    table_name: String,
    record_id: i64,
    operation: String,
    payload: String,
    device_id: Option<String>,
) -> Result<ApiResponse<i64>, String> {
    crate::commands::common::require_auth(&state).await?;
    let op = SyncOperation::from_str(&operation)
        .ok_or_else(|| format!("无效的 operation: {}", operation))?;

    let id = sync_service::enqueue(
        &state.pool,
        &table_name,
        record_id,
        op,
        &payload,
        device_id.as_deref(),
    )
    .await
    .map_err(|e: AppError| e.to_string())?;

    Ok(ApiResponse::success(id))
}

/// 获取待同步队列
#[tauri::command]
pub async fn sync_fetch_pending(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<ApiResponse<Vec<sync_service::SyncQueueItem>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let items = sync_service::fetch_pending(&state.pool, limit.unwrap_or(100))
        .await
        .map_err(|e: AppError| e.to_string())?;
    Ok(ApiResponse::success(items))
}

/// 标记为已同步
#[tauri::command]
pub async fn sync_mark_synced(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    sync_service::mark_synced(&state.pool, id)
        .await
        .map_err(|e: AppError| e.to_string())?;
    Ok(ApiResponse::success(()))
}

/// 标记为同步失败
#[tauri::command]
pub async fn sync_mark_failed(
    state: State<'_, AppState>,
    id: i64,
    error_msg: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    sync_service::mark_failed(&state.pool, id, &error_msg)
        .await
        .map_err(|e: AppError| e.to_string())?;
    Ok(ApiResponse::success(()))
}

/// 获取队列统计
#[tauri::command]
pub async fn sync_get_queue_stats(
    state: State<'_, AppState>,
) -> Result<ApiResponse<sync_service::SyncQueueStats>, String> {
    crate::commands::common::require_auth(&state).await?;
    let stats = sync_service::get_queue_stats(&state.pool)
        .await
        .map_err(|e: AppError| e.to_string())?;
    Ok(ApiResponse::success(stats))
}

/// 注册新设备
#[tauri::command]
pub async fn sync_register_device(
    state: State<'_, AppState>,
    id: String,
    device_name: String,
    device_type: String,
    public_key: String,
    is_current: bool,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    sync_service::register_device(
        &state.pool,
        &id,
        &device_name,
        &device_type,
        &public_key,
        is_current,
    )
    .await
    .map_err(|e: AppError| e.to_string())?;
    Ok(ApiResponse::success(()))
}

/// 列出所有已注册设备
#[tauri::command]
pub async fn sync_list_devices(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<sync_service::SyncDevice>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let devices = sync_service::list_devices(&state.pool)
        .await
        .map_err(|e: AppError| e.to_string())?;
    Ok(ApiResponse::success(devices))
}

/// 注销设备
#[tauri::command]
pub async fn sync_unregister_device(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    sync_service::unregister_device(&state.pool, &device_id)
        .await
        .map_err(|e: AppError| e.to_string())?;
    Ok(ApiResponse::success(()))
}

// ============================================================================
// Phase 2-4 新增命令
// ============================================================================

/// 测试传输后端连接（WebDAV/S3）
///
/// 参数：
/// - `backend_type`: "webdav" 或 "s3"
/// - `config_json`: 后端配置的 JSON 字符串
///
/// 返回：连接是否成功
#[tauri::command]
pub async fn sync_test_transport(
    state: State<'_, AppState>,
    backend_type: String,
    config_json: String,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    use crate::services::sync::s3::{S3Backend, S3Config};
    use crate::services::sync::webdav::{WebDavBackend, WebDavConfig};

    match backend_type.as_str() {
        "webdav" => {
            let config: WebDavConfig = serde_json::from_str(&config_json)
                .map_err(|e| format!("WebDAV 配置解析失败: {}", e))?;
            let backend = WebDavBackend::new(config)
                .map_err(|e: AppError| e.to_string())?;
            let result = backend
                .test_connection()
                .await
                .map_err(|e: AppError| e.to_string())?;
            Ok(ApiResponse::success(result))
        }
        "s3" => {
            let config: S3Config = serde_json::from_str(&config_json)
                .map_err(|e| format!("S3 配置解析失败: {}", e))?;
            let backend = S3Backend::new(config)
                .map_err(|e: AppError| e.to_string())?;
            let result = backend
                .test_connection()
                .await
                .map_err(|e: AppError| e.to_string())?;
            Ok(ApiResponse::success(result))
        }
        _ => Err(format!(
            "不支持的传输后端类型: {}（仅支持 webdav / s3）",
            backend_type
        )),
    }
}

/// 执行一次同步循环（触发退避重试调度器）
///
/// 流程：拉取 pending 队列 → 标记 syncing → 标记 synced/failed
/// 返回：本次执行的统计结果
#[tauri::command]
pub async fn sync_run_once(
    state: State<'_, AppState>,
) -> Result<ApiResponse<crate::services::sync::scheduler::SyncRunResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let scheduler = SyncScheduler::with_default_config();
    let result = scheduler
        .run_once(&state.pool)
        .await
        .map_err(|e: AppError| e.to_string())?;
    Ok(ApiResponse::success(result))
}

/// 获取同步状态（pending/syncing/synced/failed/conflict 计数）
#[tauri::command]
pub async fn sync_get_status(
    state: State<'_, AppState>,
) -> Result<ApiResponse<sync_service::SyncQueueStats>, String> {
    crate::commands::common::require_auth(&state).await?;
    let stats = sync_service::get_queue_stats(&state.pool)
        .await
        .map_err(|e: AppError| e.to_string())?;
    Ok(ApiResponse::success(stats))
}

/// 获取所有冲突状态的队列记录
///
/// 返回 sync_status = 'conflict' 的记录列表，供前端展示冲突解决 UI。
#[tauri::command]
pub async fn sync_list_conflicts(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<sync_service::SyncQueueItem>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let items = sqlx::query_as::<_, sync_service::SyncQueueItem>(
        "SELECT * FROM sync_queue WHERE sync_status = 'conflict' ORDER BY created_at ASC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| AppError::Database(e).to_string())?;
    Ok(ApiResponse::success(items))
}

/// 手动解决冲突
///
/// 参数：
/// - `id`: 冲突记录 ID
/// - `resolution`: 解决方案，"local"（保留本地） / "remote"（采用远端） / "merged"（合并）
/// - `resolved_payload`: 解决后的 payload（JSON 字符串，仅 "merged" 时需要）
#[tauri::command]
pub async fn sync_resolve_conflict(
    state: State<'_, AppState>,
    id: i64,
    resolution: String,
    resolved_payload: Option<String>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match resolution.as_str() {
        "local" | "remote" | "merged" => {
            // 更新 payload（如提供）并标记为 pending 重新同步
            if let Some(payload) = resolved_payload {
                sqlx::query(
                    "UPDATE sync_queue SET sync_status = 'pending', payload = ?, last_error = NULL, retry_count = 0 WHERE id = ?",
                )
                .bind(&payload)
                .bind(id)
                .execute(&state.pool)
                .await
                .map_err(|e| AppError::Database(e).to_string())?;
            } else {
                sqlx::query(
                    "UPDATE sync_queue SET sync_status = 'pending', last_error = NULL, retry_count = 0 WHERE id = ?",
                )
                .bind(id)
                .execute(&state.pool)
                .await
                .map_err(|e| AppError::Database(e).to_string())?;
            }
            tracing::info!("[sync] 冲突 {} 已解决（方案: {}）", id, resolution);
            Ok(ApiResponse::success(()))
        }
        _ => Err(format!(
            "无效的解决方案: {}（仅支持 local / remote / merged）",
            resolution
        )),
    }
}

/// 生成 ECDH 密钥对
///
/// 用于设备注册时生成本设备公钥/私钥。
/// 公钥上传到 sync_devices.public_key，私钥本地加密存储（TODO: Phase 5）。
#[tauri::command]
pub async fn sync_ecdh_generate_keypair(
    state: State<'_, AppState>,
) -> Result<ApiResponse<SerializedKeyPair>, String> {
    crate::commands::common::require_auth(&state).await?;
    let keypair = EcdhKeyPair::generate().map_err(|e: AppError| e.to_string())?;

    // 注：当前实现仅返回公钥（私钥因 EphemeralSecret 设计无法序列化）
    // 实际场景下应在内存中保留 encryptor 状态，每次启动重新生成密钥
    // TODO: Phase 3 改用 NonZeroScalar 实现长期密钥
    Ok(ApiResponse::success(SerializedKeyPair {
        private_key_b64: String::new(), // 私钥不返回（需在内存中保持）
        public_key_b64: keypair.public_key_b64,
    }))
}

/// E2EE 加密数据
///
/// 参数：
/// - `payload`: 明文数据（JSON 字符串）
/// - `peer_public_key_b64`: 目标设备的公钥
///
/// 返回：加密后的数据包（含临时公钥、nonce、密文）
#[tauri::command]
pub async fn sync_e2ee_encrypt(
    state: State<'_, AppState>,
    payload: String,
    peer_public_key_b64: String,
) -> Result<ApiResponse<crate::services::sync::e2ee::EncryptedPayload>, String> {
    crate::commands::common::require_auth(&state).await?;
    let encrypted = e2ee::encrypt_payload_once(&payload, &peer_public_key_b64)
        .map_err(|e: AppError| e.to_string())?;
    Ok(ApiResponse::success(encrypted))
}

/// E2EE 验证加密数据包格式
#[tauri::command]
pub async fn sync_e2ee_validate(
    state: State<'_, AppState>,
    encrypted: crate::services::sync::e2ee::EncryptedPayload,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    let result = e2ee::validate_encrypted_payload(&encrypted)
        .map_err(|e: AppError| e.to_string())?;
    Ok(ApiResponse::success(result))
}

// ============================================================================
// Phase 3 Task 1 新增命令：网络状态检测
// ============================================================================

/// 获取当前缓存的网络状态（不触发探测）
///
/// 返回 `NetworkStatus { is_online, last_checked }`：
/// - `is_online`: bool，最近一次探测结果
/// - `last_checked`: i64，Unix 时间戳（秒）
///
/// 前端可轮询此命令，或更推荐监听 `network-status-changed` 事件获取实时变更。
#[tauri::command]
pub async fn sync_get_network_status(
    state: State<'_, AppState>,
) -> Result<ApiResponse<NetworkStatus>, String> {
    crate::commands::common::require_auth(&state).await?;
    let status = state.connectivity_checker.get_status().await;
    Ok(ApiResponse::success(status))
}

/// 立即触发一次网络探测并返回最新状态
///
/// 与 `sync_get_network_status` 的差异：
/// - 此命令会主动发起一次 HTTP HEAD 探测（3 秒超时）
/// - 若探测结果与缓存状态不同，会通过 Tauri 事件 `network-status-changed` 广播
///
/// 适用场景：用户手动点击「检查网络」按钮，或前端在关键操作前主动确认网络可达性。
#[tauri::command]
pub async fn sync_check_network_now(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<ApiResponse<NetworkStatus>, String> {
    crate::commands::common::require_auth(&state).await?;
    let status = state.connectivity_checker.check_now(&app_handle).await;
    Ok(ApiResponse::success(status))
}

// ============================================================================
// Phase 3 Task 5 新增命令：配置同步降级
// ============================================================================

/// 记录配置变更到 sync_queue
///
/// 前端在配置 Store 的 setter 中调用此命令，将变更记录到待发队列。
/// 配置本地立即生效由前端处理，此命令仅负责入队。
///
/// 参数：
/// - `config_key`: 配置项标识（如 "theme" / "fontFamily" / "titleBarStyle"）
/// - `config_value`: 配置值（字符串，复杂结构可 JSON.stringify）
#[tauri::command]
pub async fn sync_record_config_change(
    state: State<'_, AppState>,
    config_key: String,
    config_value: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    config_sync_service::record_config_change(&state.pool, &config_key, &config_value)
        .await
        .map_err(|e: AppError| e.to_string())?;
    Ok(ApiResponse::success(()))
}

/// 网络恢复时 flush 配置队列
///
/// 将 `table_name = 'app_config'` 且 `sync_status = 'failed'` 的条目重置为 `pending`，
/// 让 scheduler 重新尝试同步。返回受影响的行数。
///
/// 通常由后端 `connectivity::check_now` 在检测到 `is_online: false → true` 时自动触发，
/// 前端也可在需要时手动调用（如用户点击「立即同步」按钮）。
#[tauri::command]
pub async fn sync_flush_config_queue(
    state: State<'_, AppState>,
) -> Result<ApiResponse<i64>, String> {
    crate::commands::common::require_auth(&state).await?;
    let count = config_sync_service::flush_config_queue(&state.pool)
        .await
        .map_err(|e: AppError| e.to_string())?;
    Ok(ApiResponse::success(count))
}

/// 获取待发配置条数
///
/// 返回 `table_name = 'app_config'` 且 `sync_status IN ('pending', 'failed')` 的条数。
/// 前端用于展示"有 N 条配置变更等待同步"。
#[tauri::command]
pub async fn sync_get_pending_config_count(
    state: State<'_, AppState>,
) -> Result<ApiResponse<i64>, String> {
    crate::commands::common::require_auth(&state).await?;
    let count = config_sync_service::get_pending_config_count(&state.pool)
        .await
        .map_err(|e: AppError| e.to_string())?;
    Ok(ApiResponse::success(count))
}
