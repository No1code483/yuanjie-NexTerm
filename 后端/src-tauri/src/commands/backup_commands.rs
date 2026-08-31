//! T2.8 自动备份系统 - Tauri 命令
//!
//! 实现规范：`功能展望/平台级增强/02_数据完整性_迁移与备份.md` §2.3.10
//!
//! 暴露的命令：
//! - `backup_create_now`：手动创建备份（指定类型和可选标签）
//! - `backup_list`：列出备份记录（可选类型过滤 + limit）
//! - `backup_delete`：删除指定备份（路径安全校验）
//! - `backup_stats`：获取备份统计信息
//! - `backup_verify`：重新校验指定备份文件
//!
//! 不在 T2.8 范围内（T2.9 实现）：
//! - `backup_restore`：从备份恢复数据
//! - `backup_export`：导出备份为 zip

use tauri::State;

use crate::db::connection::AppState;
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;
use crate::models::backup::{BackupMetadata, BackupRecord, BackupResult, BackupStats, BackupType};
use crate::services::backup_service;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

/// 手动创建备份
///
/// 参数：
/// - `backup_type`：备份类型字符串（"hourly" | "daily" | "pre_op" | "emergency"）
/// - `label`：可选标签（用于 pre_op 和 emergency 标识触发原因）
#[tauri::command]
pub async fn backup_create_now(
    state: State<'_, AppState>,
    backup_type: String,
    label: Option<String>,
) -> Result<ApiResponse<BackupResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let bt = BackupType::from_str(&backup_type).ok_or_else(|| {
        let err: AppError = AppError::Validation(format!(
            "无效的备份类型: {}（应为 hourly/daily/pre_op/emergency）",
            backup_type
        ));
        let s: String = err.into();
        s
    })?;

    match backup_service::create_backup(&state.pool, &state.data_dir, bt, label.as_deref()).await {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Err(e.into()),
    }
}

/// 列出备份记录
///
/// 参数：
/// - `backup_type`：可选类型过滤（None 表示所有类型）
/// - `limit`：最大返回数量（默认 50）
#[tauri::command]
pub async fn backup_list(
    state: State<'_, AppState>,
    backup_type: Option<String>,
    limit: Option<usize>,
) -> Result<ApiResponse<Vec<BackupRecord>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let limit = limit.unwrap_or(50);
    match backup_service::list_backups(&state.data_dir, backup_type.as_deref(), limit).await {
        Ok(records) => Ok(ApiResponse::success(records)),
        Err(e) => Err(e.into()),
    }
}

/// 删除指定备份
///
/// 安全：路径必须位于 `<data_dir>/backups/` 目录内，否则拒绝
#[tauri::command]
pub async fn backup_delete(
    state: State<'_, AppState>,
    backup_path: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match backup_service::delete_backup(&state.data_dir, &backup_path).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

/// 获取备份统计信息
#[tauri::command]
pub async fn backup_stats(
    state: State<'_, AppState>,
) -> Result<ApiResponse<BackupStats>, String> {
    crate::commands::common::require_auth(&state).await?;
    match backup_service::get_backup_stats(&state.data_dir).await {
        Ok(stats) => Ok(ApiResponse::success(stats)),
        Err(e) => Err(e.into()),
    }
}

/// 重新校验指定备份文件
///
/// 手动触发校验，更新元数据中的 integrity_check 和 checksum
#[tauri::command]
pub async fn backup_verify(
    state: State<'_, AppState>,
    backup_path: String,
) -> Result<ApiResponse<BackupMetadata>, String> {
    crate::commands::common::require_auth(&state).await?;
    match backup_service::verify_backup_now(&state.data_dir, &backup_path).await {
        Ok(metadata) => Ok(ApiResponse::success(metadata)),
        Err(e) => Err(e.into()),
    }
}
