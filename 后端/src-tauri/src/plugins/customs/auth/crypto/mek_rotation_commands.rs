//! customs.auth 私有 MEK 轮换命令（阶段3 批次1a-2a：自内核 `src/commands/` 收编）。
//!
//! 契约依据：`02_核心机制设计/02_后端插件架构设计.md` §七
//! 「`crypto/`（用户密钥）→ auth 插件私有 + kernel 仅留加密基础设施接口」。
//!
//! 提供 3 条命令（批次1a-2a 起经 `au:plugin:<旧名>` alias 派发，同时保留旧
//! Tauri transport 供 S6 双轨回放比对）：
//! 1. `mek_rotation_status`   — 查询当前 MEK 版本 + 距上次轮换天数
//! 2. `mek_rotation_rotate_now` — 立即触发轮换（manual / emergency）
//! 3. `mek_rotation_history`  — 查询轮换历史
//!
//! 批次1a-2a 修正的两点（S1 裁定「绕过路径必修 + 越权一并加固」）：
//! - **不再自建 `MekManager` 实例**：轮换走 auth 私有实现，结果经内核门面写回
//!   `AppState.mek_manager` 的会话缓存，库中活跃版本与内存缓存保持一致；
//! - **越权加固**：`user_id` 入参必须等于当前会话用户（原实现任意已登录用户
//!   可查/轮换他人 MEK）。

use base64::Engine;
use tauri::State;

use super::mek_manager::{self, MekRotationLog, MekRotationResult, MekStatus, RotationReason};
use crate::db::connection::AppState;
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;

/// 越权防护：MEK 为用户私密密钥材料，只允许操作**当前会话用户**自身。
fn ensure_self(command: &str, current_user_id: i64, user_id: i64) -> Result<(), String> {
    if current_user_id == user_id {
        return Ok(());
    }
    Err(AppError::Permission {
        resource: format!("{command}(user_id={user_id})"),
        action: format!("当前会话用户（user_id={current_user_id}）"),
    }
    .to_string())
}

/// 查询当前用户的 MEK 状态
///
/// 返回：
/// - current_version: 当前版本号（None 表示尚未初始化）
/// - created_at: 创建时间（Unix 秒）
/// - days_since_creation: 距上次轮换天数
/// - needs_rotation: 是否需要轮换（超过 90 天）
#[tauri::command]
pub async fn mek_rotation_status(
    state: State<'_, AppState>,
    user_id: i64,
) -> Result<ApiResponse<MekStatus>, String> {
    let current = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    ensure_self("mek_rotation_status", current, user_id)?;
    match mek_manager::get_status(&state.pool, user_id).await {
        Ok(status) => Ok(ApiResponse::success(status)),
        Err(e) => Err(e.into()),
    }
}

/// 立即触发 MEK 轮换
///
/// 参数：
/// - user_id: 用户 ID
/// - kek_base64: 用户的 KEK（Base64 编码，从密码派生）
/// - reason: 触发原因（"manual" 或 "emergency"）
#[tauri::command]
pub async fn mek_rotation_rotate_now(
    state: State<'_, AppState>,
    user_id: i64,
    kek_base64: String,
    reason: String,
) -> Result<ApiResponse<MekRotationResult>, String> {
    let current = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    ensure_self("mek_rotation_rotate_now", current, user_id)?;

    // 解析 KEK
    let kek_bytes = base64::engine::general_purpose::STANDARD
        .decode(&kek_base64)
        .map_err(|e| {
            AppError::Validation(format!("KEK Base64 解码失败: {}", e))
                .to_string()
        })?;

    if kek_bytes.len() != 32 {
        return Err(AppError::Validation(format!(
            "KEK 长度错误: 期望 32 字节, 实际 {} 字节",
            kek_bytes.len()
        ))
        .to_string());
    }

    let mut kek_arr = [0u8; 32];
    kek_arr.copy_from_slice(&kek_bytes);

    // 解析触发原因
    let rotation_reason = match reason.as_str() {
        "manual" => RotationReason::Manual,
        "emergency" => RotationReason::Emergency,
        "scheduled" => RotationReason::Scheduled,
        _ => {
            return Err(AppError::Validation(format!(
                "无效的轮换原因: {}（应为 manual/emergency/scheduled）",
                reason
            ))
            .to_string())
        }
    };

    let (result, new_mek) =
        mek_manager::rotate_mek(&state.pool, user_id, &kek_arr, rotation_reason).await?;

    // 轮换结果写回会话缓存：库中活跃版本与内存缓存必须一致
    state.mek_manager.write().await.cache_mek(user_id, new_mek);

    Ok(ApiResponse::success(result))
}

/// 查询 MEK 轮换历史
///
/// 参数：
/// - user_id: 用户 ID
/// - limit: 返回记录数上限（默认 10）
#[tauri::command]
pub async fn mek_rotation_history(
    state: State<'_, AppState>,
    user_id: i64,
    limit: Option<i64>,
) -> Result<ApiResponse<Vec<MekRotationLog>>, String> {
    let current = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    ensure_self("mek_rotation_history", current, user_id)?;
    let limit = limit.unwrap_or(10);
    match mek_manager::get_rotation_history(&state.pool, user_id, limit).await {
        Ok(logs) => Ok(ApiResponse::success(logs)),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    /// 测试 KEK Base64 编解码
    #[test]
    fn test_kek_base64_roundtrip() {
        let kek = [42u8; 32];
        let encoded = base64::engine::general_purpose::STANDARD.encode(kek);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .unwrap();
        assert_eq!(decoded.len(), 32);
        assert_eq!(&decoded[..], &kek[..]);
    }

    /// 测试 KEK 长度校验逻辑（命令层不做校验，由 Tauri 命令的 kek_bytes.len() != 32 处理）
    #[test]
    fn test_kek_length_validation() {
        let short_kek = base64::engine::general_purpose::STANDARD.encode([0u8; 16]);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&short_kek)
            .unwrap();
        assert_ne!(decoded.len(), 32, "16 字节 KEK 应被拒绝");
    }

    /// 越权防护：仅允许操作当前会话用户
    #[test]
    fn test_ensure_self_rejects_other_user() {
        assert!(ensure_self("mek_rotation_status", 1, 1).is_ok());
        let denied = ensure_self("mek_rotation_status", 1, 2);
        assert!(denied.is_err(), "查询他人 MEK 必须被拒绝");
        assert!(denied.unwrap_err().contains("权限不足"));
    }
}
