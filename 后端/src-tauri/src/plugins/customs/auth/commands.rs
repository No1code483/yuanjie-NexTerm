//! customs.auth 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发。
//!
//! 与 boards.profile 同构：alias 只做旧名 → 逻辑名的映射登记；真实 handler 在
//! commands::dispatch_legacy —— 闭包仅捕获 AppHandle，运行时经
//! handle.state::<AppState>() 解析主应用真实 AppState（契约 06_Rust代码契约 §8.1），
//! 业务实现复用 auth_commands / profile_commands 原函数，返回值序列化与旧 IPC
//! 路径一致（V1 输入/输出快照等价）。
//!
//! 批次1a-1 边界：MEK 密钥设施仍留在主应用 AppState（crypto 收编属 1a-2），
//! 本插件不持有加密设施。

pub const PLUGIN_ID: &str = "customs.auth";
pub const SHORT_CODE: &str = "au";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcAlias {
    pub plugin_id: &'static str,
    pub short_code: &'static str,
    pub legacy_command: &'static str,
    pub new_command: &'static str,
}

macro_rules! alias {
    ($command:literal) => {
        IpcAlias {
            plugin_id: PLUGIN_ID,
            short_code: SHORT_CODE,
            legacy_command: $command,
            new_command: concat!("au:plugin:", $command),
        }
    };
}

/// 23 条 alias：17 条 auth 命令 + 3 条自 boards.profile 收归的账号口令命令
/// （profile_change_password / profile_change_username / profile_update_profile）
/// + 3 条批次1a-2a 收编的 MEK 轮换命令（mek_rotation_status / _rotate_now / _history）。
/// create_temp_account 原属 auth 但别名登记在 boards.profile，1a-1 一并收归。
pub const IPC_ALIASES: &[IpcAlias] = &[
    alias!("register"),
    alias!("login"),
    alias!("create_temp_account"),
    alias!("recover_by_phrase"),
    alias!("logout"),
    alias!("auth_verify_token"),
    alias!("auth_get_permissions"),
    alias!("auth_reset_password"),
    alias!("auth_restore_session"),
    alias!("session_list"),
    alias!("session_revoke"),
    alias!("auth_2fa_setup"),
    alias!("auth_2fa_verify"),
    alias!("auth_2fa_enable"),
    alias!("auth_2fa_disable"),
    alias!("auth_2fa_status"),
    alias!("auth_2fa_login_verify"),
    alias!("profile_change_password"),
    alias!("profile_change_username"),
    alias!("profile_update_profile"),
    // 批次1a-2a：MEK 轮换命令自 src/commands/mek_rotation_commands.rs 收编
    alias!("mek_rotation_status"),
    alias!("mek_rotation_rotate_now"),
    alias!("mek_rotation_history"),
];

// ========== dispatcher 业务 handler 分发（S2 门禁：dispatcher 状态注入） ==========
//
// 每条 handler 的形态一致：闭包捕获 AppHandle → 运行时 handle.state::<AppState>()
// 取主应用真实状态 → 从 Json args 提取旧 IPC 参数（键名与前端 invoke 完全一致，
// 禁止按 transport 改写）→ 直接调用 auth_commands / profile_commands 原函数
// → serde 序列化返回值，与旧 Tauri command 路径输出一致。

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::{auth_commands, profile_commands};
use crate::db::connection::AppState;
use crate::models::user::{
    ChangeUsernameRequest, LoginRequest, ResetPasswordRequest, UpdateProfileRequest,
};

use super::crypto::mek_rotation_commands;

/// 从 args 提取必填字符串参数；缺失/类型不符按旧 Tauri 反序列化失败语义返回错误。
fn arg_str(args: &Json, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Json::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("参数 {key} 缺失或不是字符串"))
}

fn arg_i64(args: &Json, key: &str) -> Result<i64, String> {
    args.get(key)
        .and_then(Json::as_i64)
        .ok_or_else(|| format!("参数 {key} 缺失或不是整数"))
}

fn arg_bool(args: &Json, key: &str) -> Result<bool, String> {
    args.get(key)
        .and_then(Json::as_bool)
        .ok_or_else(|| format!("参数 {key} 缺失或不是布尔值"))
}

fn arg_opt_str(args: &Json, key: &str) -> Result<Option<String>, String> {
    match args.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("参数 {key} 不是字符串或 null")),
    }
}

fn parse_request<T: serde::de::DeserializeOwned>(args: &Json) -> Result<T, String> {
    args.get("request")
        .cloned()
        .ok_or_else(|| "参数 request 缺失".to_string())
        .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
}

fn to_json<T: serde::Serialize>(value: T) -> Result<Json, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

/// 逐条分发到旧命令函数。legacy 为 IPC_ALIASES.legacy_command 值。
pub(crate) async fn dispatch_legacy<R: Runtime>(
    handle: AppHandle<R>,
    legacy: &'static str,
    args: Json,
) -> Result<Json, String> {
    let state = handle.state::<AppState>();

    match legacy {
        "register" => {
            let username = arg_str(&args, "username")?;
            let password = arg_str(&args, "password")?;
            // 键名与前端真实调用一致（camelCase，`create_temp_account` 同规则）；
            // `auth_commands::register` 未声明 `rename_all`，故旧路径同键名同样成立。
            let is_permanent = arg_bool(&args, "isPermanent")?;
            to_json(auth_commands::register(state, username, password, is_permanent).await?)
        }
        "login" => {
            let request = parse_request::<LoginRequest>(&args)?;
            to_json(auth_commands::login(state, request).await?)
        }
        "create_temp_account" => {
            // 前端以 camelCase durationHours 传参（BUG-023），与旧路径同形。
            let duration_hours = arg_i64(&args, "durationHours")?;
            let username = arg_opt_str(&args, "username")?;
            to_json(auth_commands::create_temp_account(state, duration_hours, username).await?)
        }
        "recover_by_phrase" => {
            // 同 register：键名与前端真实调用一致（camelCase）。
            let username = arg_str(&args, "username")?;
            let recovery_phrase = arg_str(&args, "recoveryPhrase")?;
            let new_password = arg_str(&args, "newPassword")?;
            to_json(
                auth_commands::recover_by_phrase(state, username, recovery_phrase, new_password)
                    .await?,
            )
        }
        "logout" => to_json(auth_commands::logout(state).await?),
        "auth_verify_token" => to_json(auth_commands::auth_verify_token(state).await?),
        "auth_get_permissions" => to_json(auth_commands::auth_get_permissions(state).await?),
        "auth_reset_password" => {
            let request = parse_request::<ResetPasswordRequest>(&args)?;
            to_json(auth_commands::auth_reset_password(state, request).await?)
        }
        "auth_restore_session" => {
            let token = arg_str(&args, "token")?;
            to_json(auth_commands::auth_restore_session(state, token).await?)
        }
        "session_list" => to_json(auth_commands::session_list(state).await?),
        "session_revoke" => {
            // 同 register：键名与前端真实调用一致（camelCase）。
            let session_id = arg_str(&args, "sessionId")?;
            to_json(auth_commands::session_revoke(state, session_id).await?)
        }
        "auth_2fa_setup" => to_json(auth_commands::auth_2fa_setup(state).await?),
        "auth_2fa_verify" => {
            let code = arg_str(&args, "code")?;
            to_json(auth_commands::auth_2fa_verify(state, code).await?)
        }
        // 旧实现不消费 code（Tauri 反序列化忽略未声明的多余键），保持同形。
        "auth_2fa_enable" => to_json(auth_commands::auth_2fa_enable(state).await?),
        "auth_2fa_disable" => {
            let code = arg_str(&args, "code")?;
            to_json(auth_commands::auth_2fa_disable(state, code).await?)
        }
        "auth_2fa_status" => to_json(auth_commands::auth_2fa_status(state).await?),
        "auth_2fa_login_verify" => {
            // 旧签名 (user_id, code, token) 与前端实际发送的 {token, code} 不符；
            // 已改为由挑战 token 反查用户（见 auth_commands::auth_2fa_login_verify）。
            let token = arg_str(&args, "token")?;
            let code = arg_str(&args, "code")?;
            to_json(auth_commands::auth_2fa_login_verify(state, token, code).await?)
        }
        "profile_change_password" => {
            let request = parse_request::<ResetPasswordRequest>(&args)?;
            to_json(profile_commands::profile_change_password(state, request).await?)
        }
        "profile_change_username" => {
            let request = parse_request::<ChangeUsernameRequest>(&args)?;
            to_json(profile_commands::profile_change_username(state, request).await?)
        }
        "profile_update_profile" => {
            let request = parse_request::<UpdateProfileRequest>(&args)?;
            to_json(profile_commands::profile_update_profile(state, request).await?)
        }
        // 批次1a-2a：MEK 轮换命令（auth 私有实现，见 crypto/mek_rotation_commands.rs）。
        // `user_id` / `kek_base64` 在旧 Tauri 路径按默认 camelCase 绑定，故此处同形取键。
        "mek_rotation_status" => {
            let user_id = arg_i64(&args, "userId")?;
            to_json(mek_rotation_commands::mek_rotation_status(state, user_id).await?)
        }
        "mek_rotation_rotate_now" => {
            let user_id = arg_i64(&args, "userId")?;
            let kek_base64 = arg_str(&args, "kekBase64")?;
            let reason = arg_str(&args, "reason")?;
            to_json(
                mek_rotation_commands::mek_rotation_rotate_now(
                    state, user_id, kek_base64, reason,
                )
                .await?,
            )
        }
        "mek_rotation_history" => {
            let user_id = arg_i64(&args, "userId")?;
            let limit = args.get("limit").and_then(Json::as_i64);
            to_json(mek_rotation_commands::mek_rotation_history(state, user_id, limit).await?)
        }
        _ => Err(format!("未知 auth 逻辑命令: {legacy}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "register",
        "login",
        "create_temp_account",
        "recover_by_phrase",
        "logout",
        "auth_verify_token",
        "auth_get_permissions",
        "auth_reset_password",
        "auth_restore_session",
        "session_list",
        "session_revoke",
        "auth_2fa_setup",
        "auth_2fa_verify",
        "auth_2fa_enable",
        "auth_2fa_disable",
        "auth_2fa_status",
        "auth_2fa_login_verify",
        "profile_change_password",
        "profile_change_username",
        "profile_update_profile",
        "mek_rotation_status",
        "mek_rotation_rotate_now",
        "mek_rotation_history",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 23);
        assert_eq!(actual.len(), IPC_ALIASES.len(), "legacy_command 存在重复");
        assert_eq!(actual, expected);
    }

    #[test]
    fn alias_map_uses_formal_id_and_unique_short_code_commands() {
        let new_commands: HashSet<_> = IPC_ALIASES.iter().map(|alias| alias.new_command).collect();

        assert_eq!(
            new_commands.len(),
            IPC_ALIASES.len(),
            "new_command 存在重复"
        );
        for alias in IPC_ALIASES {
            assert_eq!(alias.plugin_id, PLUGIN_ID);
            assert_eq!(alias.short_code, SHORT_CODE);
            assert_eq!(
                alias.new_command,
                format!("{SHORT_CODE}:plugin:{}", alias.legacy_command)
            );
        }
    }

    #[test]
    fn temp_account_duration_uses_frontend_camel_case_key() {
        let args = serde_json::json!({"durationHours": 24, "username": "temp"});
        assert_eq!(arg_i64(&args, "durationHours").unwrap(), 24);
        assert_eq!(
            arg_opt_str(&args, "username").unwrap().as_deref(),
            Some("temp")
        );
        assert!(arg_i64(&args, "duration_hours").is_err());
    }
}
