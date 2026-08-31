//! T2.9 数据恢复与导出 - Tauri 命令
//!
//! 实现规范：`功能展望/平台级增强/02_数据完整性_迁移与备份.md` §2.4 + §2.5
//!
//! 暴露的命令：
//! - `restore_from_backup`：从备份文件恢复数据（7 步流程 + 自动回滚）
//! - `export_to_zip`：全量导出数据库到 zip 文件
//! - `check_import_compatibility`：检查导入文件兼容性（不实际导入）
//! - `import_from_zip`：从 zip 文件导入数据
//!
//! 重要约束：
//! - `restore_from_backup` 和 `import_from_zip` 都会关闭当前连接池
//! - 调用这两个命令后，前端必须重启应用（基于 `need_restart: true` 标志）
//! - 前端可通过 `tauri::app::restart()` 或提示用户手动重启

use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::restore::{
    ExportResult, ImportCompatibility, ImportResult, RestoreResult,
};
use crate::services::{export_service, restore_service};

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

/// 从备份文件恢复数据
///
/// 完整 7 步流程：验证 → 备份当前 → 关闭连接池 → 覆盖文件 → 清理 WAL → 一致性检查 → 失败回滚
///
/// 参数：
/// - `backup_path`：目标备份文件路径（必须位于 `<data_dir>/backups/` 目录内）
///
/// 返回：
/// - `RestoreResult`：包含恢复状态、回滚备份路径、schema_version 等
/// - 错误：若恢复失败且回滚成功，返回错误描述；若回滚也失败，错误消息中包含手动恢复建议
#[tauri::command]
pub async fn restore_from_backup(
    state: State<'_, AppState>,
    backup_path: String,
) -> Result<ApiResponse<RestoreResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    match restore_service::restore_from_backup(&state.pool, &state.data_dir, &backup_path).await {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Err(e.into()),
    }
}

/// 全量导出数据库到 zip 文件
///
/// 导出格式：
/// - `data.db`：SQLite 数据库副本（VACUUM INTO）
/// - `manifest.json`：导出元信息（schema_version、表行数等）
/// - `checksum.sha256`：data.db 的 SHA256 校验和
///
/// 参数：
/// - `exported_by`：可选导出者用户名
///
/// 返回：
/// - `ExportResult`：包含导出文件路径、大小、校验和等
#[tauri::command]
pub async fn export_to_zip(
    state: State<'_, AppState>,
    exported_by: Option<String>,
) -> Result<ApiResponse<ExportResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    match export_service::export_to_zip(&state.pool, &state.data_dir, exported_by.as_deref()).await {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Err(e.into()),
    }
}

/// 检查导入文件兼容性
///
/// 在实际导入前调用，检查 zip 文件的 schema_version 是否与当前应用兼容：
/// - 若导入文件版本 > 当前版本 → 拒绝（不兼容）
/// - 若导入文件版本 < 当前版本 → 兼容，但需重启后自动迁移
/// - 若版本相同 → 兼容，无需迁移
///
/// 参数：
/// - `zip_path`：导入 zip 文件路径
#[tauri::command]
pub async fn check_import_compatibility(
    state: State<'_, AppState>,
    zip_path: String,
) -> Result<ApiResponse<ImportCompatibility>, String> {
    crate::commands::common::require_auth(&state).await?;
    // 获取当前 schema_version
    let current_schema_version: i64 = sqlx::query_scalar(
        "SELECT MAX(version) FROM schema_migrations WHERE version IS NOT NULL;",
    )
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    match export_service::check_import_compatibility(&zip_path, current_schema_version).await {
        Ok(compat) => Ok(ApiResponse::success(compat)),
        Err(e) => Err(e.into()),
    }
}

/// 从 zip 文件导入数据
///
/// 流程：
/// 1. 检查兼容性（高版本拒绝）
/// 2. 解压 data.db 到临时文件
/// 3. 校验 SHA256
/// 4. 备份当前数据库（回滚点）
/// 5. 关闭连接池
/// 6. 复制 data.db 到数据库位置
/// 7. 清理 WAL/SHM
///
/// 返回：
/// - `ImportResult`：包含导入状态、schema_version、是否需要迁移等
#[tauri::command]
pub async fn import_from_zip(
    state: State<'_, AppState>,
    zip_path: String,
) -> Result<ApiResponse<ImportResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    match export_service::import_from_zip(&state.pool, &state.data_dir, &zip_path).await {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Err(e.into()),
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::app_error::AppError;

    #[test]
    fn test_app_error_backup_variant() {
        // 验证 AppError::Backup 错误码正确
        let err = AppError::Backup("test".into());
        assert_eq!(err.error_code(), 7001);
        assert_eq!(err.category(), "backup");
    }
}
