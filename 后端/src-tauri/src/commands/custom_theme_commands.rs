//! 自定义主题 Tauri 命令（C1.5 / v1.51.7）
//!
//! 规范：`功能展望/体验深化/01_主题自定义系统_未来展望.md` §2.5
//!
//! 暴露 5 个 IPC 命令：
//! - `custom_theme_list`：列出所有自定义主题（按 updated_at 倒序）
//! - `custom_theme_get`：按 id 获取单个自定义主题
//! - `custom_theme_upsert`：新增或更新（按 name 唯一约束，UPSERT）
//! - `custom_theme_delete`：按 id 删除
//! - `custom_theme_migrate_local`：批量从 localStorage 迁移到 DB（一次性数据迁移）

use tauri::State;

use crate::db::connection::AppState;
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;
use crate::models::custom_theme::{CustomTheme, UpsertCustomThemeRequest};
use crate::services::custom_theme_service;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

/// 列出所有自定义主题
#[tauri::command]
pub async fn custom_theme_list(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<CustomTheme>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match custom_theme_service::list(&state.pool, user_id).await {
        Ok(themes) => Ok(ApiResponse::success(themes)),
        Err(e) => Err(e.into()),
    }
}

/// 按 id 查询单个自定义主题
#[tauri::command]
pub async fn custom_theme_get(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<Option<CustomTheme>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match custom_theme_service::get_by_id(&state.pool, user_id, id).await {
        Ok(theme) => Ok(ApiResponse::success(theme)),
        Err(e) => Err(e.into()),
    }
}

/// 新增或更新自定义主题（UPSERT 语义：name 唯一约束冲突时更新）
#[tauri::command]
pub async fn custom_theme_upsert(
    state: State<'_, AppState>,
    request: UpsertCustomThemeRequest,
) -> Result<ApiResponse<CustomTheme>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match custom_theme_service::upsert(
        &state.pool,
        user_id,
        &request.name,
        request.base_theme.as_deref(),
        &request.variables,
    )
    .await
    {
        Ok(theme) => Ok(ApiResponse::success(theme)),
        Err(e) => Err(e.into()),
    }
}

/// 按 id 删除自定义主题
#[tauri::command]
pub async fn custom_theme_delete(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<u64>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match custom_theme_service::delete_by_id(&state.pool, user_id, id).await {
        Ok(affected) => {
            if affected == 0 {
                Err(AppError::NotFound.to_string())
            } else {
                Ok(ApiResponse::success(affected))
            }
        }
        Err(e) => Err(e.into()),
    }
}

/// 批量迁移：从 localStorage JSON 字符串一次性导入多个自定义主题
///
/// 前端首次启动时检测到 localStorage 中存在 `nexterm-custom-themes` 键时调用。
/// 入参格式（旧 localStorage 中的数组）：
/// ```json
/// [
///   { "name": "我的暗色", "variables": { ... }, "createdAt": "2026-07-22T..." },
///   ...
/// ]
/// ```
/// 返回成功迁移的主题数量；失败的主题会被跳过并记录日志。
#[tauri::command]
pub async fn custom_theme_migrate_local(
    state: State<'_, AppState>,
    themes_json: String,
) -> Result<ApiResponse<usize>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    // 解析 JSON 数组
    let themes: Vec<serde_json::Value> = match serde_json::from_str(&themes_json) {
        Ok(arr) => arr,
        Err(e) => {
            return Err(AppError::Validation(format!("无效的 JSON: {}", e)).into());
        }
    };

    let mut migrated = 0usize;
    for theme in themes {
        // 提取字段
        let name = theme
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("未命名主题");
        let variables = if let Some(vars) = theme.get("variables") {
            serde_json::to_string(vars).unwrap_or_else(|_| "{}".to_string())
        } else {
            "{}".to_string()
        };

        // 复用 upsert，逐个迁移（失败则跳过并记录）
        match custom_theme_service::upsert(&state.pool, user_id, name, None, &variables).await {
            Ok(_) => migrated += 1,
            Err(e) => {
                tracing::warn!(
                    "[custom_theme_migrate] 跳过主题 '{}' 迁移失败: {}",
                    name,
                    e
                );
            }
        }
    }

    tracing::info!("[custom_theme_migrate] 迁移完成，共 {} 个主题", migrated);
    Ok(ApiResponse::success(migrated))
}
