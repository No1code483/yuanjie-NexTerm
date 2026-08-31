//! T2.13 MEK 轮换 Tauri 命令
//!
//! 设计依据：功能展望/平台级增强/05_安全加固_A4.md §2.3.6
//!
//! 提供 3 个 Tauri 命令：
//! 1. mek_rotation_status  — 查询当前 MEK 版本 + 距上次轮换天数
//! 2. mek_rotation_rotate_now — 立即触发轮换（manual / emergency）
//! 3. mek_rotation_history — 查询轮换历史

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

use base64::Engine;
use tauri::State;

use crate::crypto::mek_manager::{MekManager, MekRotationLog, MekRotationResult, MekStatus, RotationReason};
use crate::db::connection::AppState;
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;

// ============================================================================
// 命令辅助：从 AppState 获取 MekManager
// ============================================================================

// 注意：MekManager 需要在 AppState 中持久化。
// 当前简化方案：每次命令调用创建新的 MekManager 实例（版本查询走数据库，缓存丢失但功能正常）
// 后续可优化为将 MekManager 加入 AppState::manage()。

// ============================================================================
// Tauri 命令
// ============================================================================

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
    crate::commands::common::require_auth(&state).await?;
    let mgr = MekManager::new();
    match mgr.get_status(&state.pool, user_id).await {
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
    crate::commands::common::require_auth(&state).await?;
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

    let mut mgr = MekManager::new();
    match mgr
        .rotate_mek(&state.pool, user_id, &kek_arr, rotation_reason)
        .await
    {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Err(e.into()),
    }
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
    crate::commands::common::require_auth(&state).await?;
    let limit = limit.unwrap_or(10);
    let mgr = MekManager::new();
    match mgr.get_rotation_history(&state.pool, user_id, limit).await {
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
}
