//! customs.sync 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发。
//!
//! alias 只做旧名 → 逻辑名的映射登记；真实 handler 在
//! commands::dispatch_legacy —— 闭包仅捕获 AppHandle，运行时经
//! handle.state::<AppState>() 解析主应用真实 AppState（契约 06_Rust代码契约 §8.1），
//! 业务实现复用 commands::sync_commands 原函数，返回值序列化与旧 IPC 路径一致。
//!
//! 批次6b S1：21 sync 命令登记为 alias。

pub const PLUGIN_ID: &str = "customs.sync";
pub const SHORT_CODE: &str = "sy";

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
            new_command: concat!("sy:plugin:", $command),
        }
    };
}

/// 21 条 alias：sync 21 条命令。
pub const IPC_ALIASES: &[IpcAlias] = &[
    // ===== 基础同步命令（8）=====
    alias!("sync_enqueue"),
    alias!("sync_fetch_pending"),
    alias!("sync_mark_synced"),
    alias!("sync_mark_failed"),
    alias!("sync_get_queue_stats"),
    alias!("sync_register_device"),
    alias!("sync_list_devices"),
    alias!("sync_unregister_device"),
    // ===== Phase 2 命令（6）=====
    alias!("sync_test_transport"),
    alias!("sync_run_once"),
    alias!("sync_get_status"),
    alias!("sync_list_conflicts"),
    alias!("sync_resolve_conflict"),
    alias!("sync_e2ee_validate"),
    // ===== E2EE 命令（3）=====
    alias!("sync_ecdh_generate_keypair"),
    alias!("sync_e2ee_encrypt"),
    // ===== Phase 3 Task 1 命令（2）=====
    alias!("sync_get_network_status"),
    alias!("sync_check_network_now"),
    // ===== Phase 3 Task 5 命令（3）=====
    alias!("sync_record_config_change"),
    alias!("sync_flush_config_queue"),
    alias!("sync_get_pending_config_count"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::sync_commands;
use crate::db::connection::AppState;

fn arg_str(args: &Json, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Json::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("参数 {key} 缺失或不是字符串"))
}

fn arg_opt_str(args: &Json, key: &str) -> Result<Option<String>, String> {
    match args.get(key) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => {
            let v = v.as_str().ok_or_else(|| format!("参数 {key} 不是字符串"))?;
            Ok(Some(v.to_string()))
        }
        None => Ok(None),
    }
}

fn arg_i64(args: &Json, key: &str) -> Result<i64, String> {
    args.get(key)
        .and_then(Json::as_i64)
        .ok_or_else(|| format!("参数 {key} 缺失或不是整数"))
}

fn arg_opt_i64(args: &Json, key: &str) -> Result<Option<i64>, String> {
    match args.get(key) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => {
            let v = v.as_i64().ok_or_else(|| format!("参数 {key} 不是整数"))?;
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

fn arg_usize(args: &Json, key: &str) -> Result<usize, String> {
    args.get(key)
        .and_then(Json::as_u64)
        .map(|v| v as usize)
        .ok_or_else(|| format!("参数 {key} 缺失或不是整数"))
}

fn arg_opt_usize(args: &Json, key: &str) -> Result<Option<usize>, String> {
    match args.get(key) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => {
            let v = v.as_u64().map(|v| v as usize).ok_or_else(|| format!("参数 {key} 不是整数"))?;
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

fn arg_bool(args: &Json, key: &str) -> Result<bool, String> {
    args.get(key)
        .and_then(Json::as_bool)
        .ok_or_else(|| format!("参数 {key} 缺失或不是布尔值"))
}

fn arg_json<T: serde::de::DeserializeOwned>(args: &Json, key: &str) -> Result<T, String> {
    args.get(key)
        .ok_or_else(|| format!("参数 {key} 缺失"))
        .and_then(|v| serde_json::from_value(v.clone()).map_err(|e| format!("参数 {key} 反序列化失败: {}", e)))
}

fn to_json<T: serde::Serialize>(value: T) -> Result<Json, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

/// 逐条分发到旧命令函数。legacy 为 IPC_ALIASES.legacy_command 值。
/// sync_commands 使用 State<'_, AppState> + require_auth 模式，部分命令需要 AppHandle。
/// 直接调用原函数复用 require_auth。
pub(crate) async fn dispatch_legacy<R: Runtime>(
    handle: AppHandle<R>,
    legacy: &'static str,
    args: Json,
) -> Result<Json, String> {
    let state = handle.state::<AppState>();

    match legacy {
        "sync_enqueue" => {
            let table_name = arg_str(&args, "tableName")?;
            let record_id = arg_i64(&args, "recordId")?;
            let operation = arg_str(&args, "operation")?;
            let payload = arg_str(&args, "payload")?;
            let device_id = arg_opt_str(&args, "deviceId")?;
            to_json(sync_commands::sync_enqueue(state, table_name, record_id, operation, payload, device_id).await?)
        }
        "sync_fetch_pending" => {
            let limit = arg_opt_i64(&args, "limit")?;
            to_json(sync_commands::sync_fetch_pending(state, limit).await?)
        }
        "sync_mark_synced" => {
            let id = arg_i64(&args, "id")?;
            to_json(sync_commands::sync_mark_synced(state, id).await?)
        }
        "sync_mark_failed" => {
            let id = arg_i64(&args, "id")?;
            let error_msg = arg_str(&args, "errorMsg")?;
            to_json(sync_commands::sync_mark_failed(state, id, error_msg).await?)
        }
        "sync_get_queue_stats" => {
            to_json(sync_commands::sync_get_queue_stats(state).await?)
        }
        "sync_register_device" => {
            let id = arg_str(&args, "id")?;
            let device_name = arg_str(&args, "deviceName")?;
            let device_type = arg_str(&args, "deviceType")?;
            let device_os = arg_str(&args, "deviceOs")?;
            let public_key = arg_str(&args, "publicKey")?;
            let is_current = arg_bool(&args, "isCurrent")?;
            to_json(sync_commands::sync_register_device(
                state, id, device_name, device_type, device_os, public_key, is_current,
            ).await?)
        }
        "sync_list_devices" => {
            to_json(sync_commands::sync_list_devices(state).await?)
        }
        "sync_unregister_device" => {
            let device_id = arg_str(&args, "deviceId")?;
            to_json(sync_commands::sync_unregister_device(state, device_id).await?)
        }
        "sync_test_transport" => {
            let backend_type = arg_str(&args, "backendType")?;
            let config_json = arg_str(&args, "configJson")?;
            to_json(sync_commands::sync_test_transport(state, backend_type, config_json).await?)
        }
        "sync_run_once" => {
            to_json(sync_commands::sync_run_once(state).await?)
        }
        "sync_get_status" => {
            to_json(sync_commands::sync_get_status(state).await?)
        }
        "sync_list_conflicts" => {
            to_json(sync_commands::sync_list_conflicts(state).await?)
        }
        "sync_resolve_conflict" => {
            let id = arg_i64(&args, "id")?;
            let resolution = arg_str(&args, "resolution")?;
            let resolved_payload = arg_opt_str(&args, "resolvedPayload")?;
            to_json(sync_commands::sync_resolve_conflict(state, id, resolution, resolved_payload).await?)
        }
        "sync_ecdh_generate_keypair" => {
            to_json(sync_commands::sync_ecdh_generate_keypair(state).await?)
        }
        "sync_e2ee_encrypt" => {
            let payload = arg_str(&args, "payload")?;
            let peer_public_key_b64 = arg_str(&args, "peerPublicKeyB64")?;
            to_json(sync_commands::sync_e2ee_encrypt(state, payload, peer_public_key_b64).await?)
        }
        "sync_e2ee_validate" => {
            let encrypted: crate::plugins::_legacy::services::sync::e2ee::EncryptedPayload = arg_json(&args, "encrypted")?;
            to_json(sync_commands::sync_e2ee_validate(state, encrypted).await?)
        }
        "sync_get_network_status" => {
            to_json(sync_commands::sync_get_network_status(state).await?)
        }
        "sync_check_network_now" => {
            to_json(sync_commands::sync_check_network_now(handle.clone(), state).await?)
        }
        "sync_record_config_change" => {
            let config_key = arg_str(&args, "configKey")?;
            let config_value = arg_str(&args, "configValue")?;
            to_json(sync_commands::sync_record_config_change(state, config_key, config_value).await?)
        }
        "sync_flush_config_queue" => {
            to_json(sync_commands::sync_flush_config_queue(state).await?)
        }
        "sync_get_pending_config_count" => {
            to_json(sync_commands::sync_get_pending_config_count(state).await?)
        }
        _ => Err(format!("未知 sync 逻辑命令: {legacy}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "sync_enqueue",
        "sync_fetch_pending",
        "sync_mark_synced",
        "sync_mark_failed",
        "sync_get_queue_stats",
        "sync_register_device",
        "sync_list_devices",
        "sync_unregister_device",
        "sync_test_transport",
        "sync_run_once",
        "sync_get_status",
        "sync_list_conflicts",
        "sync_resolve_conflict",
        "sync_ecdh_generate_keypair",
        "sync_e2ee_encrypt",
        "sync_e2ee_validate",
        "sync_get_network_status",
        "sync_check_network_now",
        "sync_record_config_change",
        "sync_flush_config_queue",
        "sync_get_pending_config_count",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 21);
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
}
