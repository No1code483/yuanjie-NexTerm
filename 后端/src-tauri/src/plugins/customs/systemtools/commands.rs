//! customs.systemtools 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发。
//!
//! 与 customs.auth 同构：alias 只做旧名 → 逻辑名的映射登记；真实 handler 在
//! commands::dispatch_legacy —— 闭包仅捕获 AppHandle，运行时经
//! handle.state::<AppState>() 解析主应用真实 AppState（契约 06_Rust代码契约 §8.1），
//! 业务实现复用 system_commands 原函数，返回值序列化与旧 IPC 路径一致。
//!
//! 批次4c S1：10 system 命令登记为 alias；extension_*（2）/ adapter_*（3）
//! 裁定 T2/T3 零消费判删 —— 后端命令函数已删除，不在 alias 中。
//! 批次6a S1：24 L3 systemtools 命令登记为 alias（perf 4 + backup 5 + restore 4
//! + health_check 2 + font 4 + custom_theme 5）。

pub const PLUGIN_ID: &str = "customs.systemtools";
pub const SHORT_CODE: &str = "st";

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
            new_command: concat!("st:plugin:", $command),
        }
    };
}

/// 34 条 alias：10 system + 24 L3 systemtools 命令。
/// extension_* / adapter_* 裁定删除（零前端消费） —— 不在 alias 中。
pub const IPC_ALIASES: &[IpcAlias] = &[
    // ===== L1 system 命令（10 条） =====
    alias!("get_system_config"),
    alias!("set_system_config"),
    alias!("get_all_system_configs"),
    alias!("system_open_file"),
    alias!("system_open_url"),
    alias!("system_get_app_info"),
    alias!("clipboard_write_text"),
    alias!("clipboard_read_text"),
    alias!("window_set_always_on_top"),
    alias!("window_screenshot"),
    // ===== L3 systemtools 命令（24 条）=====
    // perf 命令（4）
    alias!("record_perf_metric"),
    alias!("perf_get_slow_queries"),
    alias!("perf_get_metrics_summary"),
    alias!("perf_get_metric_timeseries"),
    // backup 命令（5）
    alias!("backup_create_now"),
    alias!("backup_list"),
    alias!("backup_delete"),
    alias!("backup_stats"),
    alias!("backup_verify"),
    // restore 命令（4）
    alias!("restore_from_backup"),
    alias!("export_to_zip"),
    alias!("check_import_compatibility"),
    alias!("import_from_zip"),
    // health_check 命令（2）
    alias!("health_check_run"),
    alias!("health_check_repair"),
    // font 命令（4）
    alias!("font_list_custom"),
    alias!("font_upload"),
    alias!("font_delete"),
    alias!("font_get_dir"),
    // custom_theme 命令（5）
    alias!("custom_theme_list"),
    alias!("custom_theme_get"),
    alias!("custom_theme_upsert"),
    alias!("custom_theme_delete"),
    alias!("custom_theme_migrate_local"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::{backup_commands, custom_theme_commands, font_commands, health_check_commands, perf_commands, restore_commands, system_commands};
use crate::db::connection::AppState;

fn arg_str(args: &Json, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Json::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("参数 {key} 缺失或不是字符串"))
}

fn arg_str_opt(args: &Json, key: &str) -> Result<Option<String>, String> {
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

fn arg_i64_opt(args: &Json, key: &str) -> Result<Option<i64>, String> {
    match args.get(key) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => {
            let v = v.as_i64().ok_or_else(|| format!("参数 {key} 不是整数"))?;
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

fn arg_usize_opt(args: &Json, key: &str) -> Result<Option<usize>, String> {
    match args.get(key) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => {
            let v = v.as_i64().ok_or_else(|| format!("参数 {key} 不是整数"))?;
            Ok(Some(v as usize))
        }
        None => Ok(None),
    }
}

fn arg_vec_i64(args: &Json, key: &str) -> Result<Vec<i64>, String> {
    args.get(key)
        .and_then(Json::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_i64())
                .collect::<Vec<_>>()
        })
        .ok_or_else(|| format!("参数 {key} 缺失或不是数组"))
}

fn arg_bool(args: &Json, key: &str) -> Result<bool, String> {
    args.get(key)
        .and_then(Json::as_bool)
        .ok_or_else(|| format!("参数 {key} 缺失或不是布尔值"))
}

fn to_json<T: serde::Serialize>(value: T) -> Result<Json, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

/// 逐条分发到旧命令函数。legacy 为 IPC_ALIASES.legacy_command 值。
/// system_commands / *_commands 使用 #[tauri::command] + State<'_, AppState> + require_auth 模式。
/// 直接调用原函数（参数与前端 invoke 一致），复用 require_auth。
///
/// font 命令原函数使用泛型 `AppHandle<R>`，dispatcher 直接传递 `handle` 无需转换。
pub(crate) async fn dispatch_legacy<R: Runtime>(
    handle: AppHandle<R>,
    legacy: &'static str,
    args: Json,
) -> Result<Json, String> {
    let state = handle.state::<AppState>();

    match legacy {
        // ===== L1 system 命令（10 条） =====
        "get_system_config" => {
            let key = arg_str(&args, "key")?;
            to_json(system_commands::get_system_config(state, key).await?)
        }
        "set_system_config" => {
            let key = arg_str(&args, "key")?;
            let value = arg_str(&args, "value")?;
            to_json(system_commands::set_system_config(state, key, value).await?)
        }
        "get_all_system_configs" => {
            to_json(system_commands::get_all_system_configs(state).await?)
        }
        "system_open_file" => {
            let path = arg_str(&args, "path")?;
            to_json(system_commands::system_open_file(state, path).await?)
        }
        "system_open_url" => {
            let url = arg_str(&args, "url")?;
            to_json(system_commands::system_open_url(state, url).await?)
        }
        "system_get_app_info" => {
            to_json(system_commands::system_get_app_info(state).await?)
        }
        "clipboard_write_text" => {
            let text = arg_str(&args, "text")?;
            to_json(system_commands::clipboard_write_text(state, handle.clone(), text).await?)
        }
        "clipboard_read_text" => {
            to_json(system_commands::clipboard_read_text(state, handle.clone()).await?)
        }
        "window_set_always_on_top" => {
            let always_on_top = arg_bool(&args, "alwaysOnTop")?;
            to_json(
                system_commands::window_set_always_on_top(state, handle.clone(), always_on_top)
                    .await?,
            )
        }
        "window_screenshot" => {
            to_json(system_commands::window_screenshot(state, handle.clone()).await?)
        }

        // ===== L3 systemtools 命令（24 条）=====
        // --- perf 命令（4）---
        "record_perf_metric" => {
            let metric_name = arg_str(&args, "metricName")?;
            let metric_value_ms = arg_i64(&args, "metricValueMs")?;
            let route = arg_str_opt(&args, "route")?;
            let command_name = arg_str_opt(&args, "commandName")?;
            let metadata = arg_str_opt(&args, "metadata")?;
            to_json(
                perf_commands::record_perf_metric(
                    state, metric_name, metric_value_ms, route, command_name, metadata,
                )
                    .await?,
            )
        }
        "perf_get_slow_queries" => {
            let limit = arg_i64_opt(&args, "limit")?;
            let command_name = arg_str_opt(&args, "commandName")?;
            let min_duration_ms = arg_i64_opt(&args, "minDurationMs")?;
            to_json(
                perf_commands::perf_get_slow_queries(state, limit, command_name, min_duration_ms)
                    .await?,
            )
        }
        "perf_get_metrics_summary" => {
            let days = arg_i64_opt(&args, "days")?;
            let metric_name = arg_str_opt(&args, "metricName")?;
            to_json(perf_commands::perf_get_metrics_summary(state, days, metric_name).await?)
        }
        "perf_get_metric_timeseries" => {
            let metric_name = arg_str(&args, "metricName")?;
            let limit = arg_i64_opt(&args, "limit")?;
            to_json(
                perf_commands::perf_get_metric_timeseries(state, metric_name, limit).await?,
            )
        }

        // --- backup 命令（5）---
        "backup_create_now" => {
            let backup_type = arg_str(&args, "backupType")?;
            let label = arg_str_opt(&args, "label")?;
            to_json(backup_commands::backup_create_now(state, backup_type, label).await?)
        }
        "backup_list" => {
            let backup_type = arg_str_opt(&args, "backupType")?;
            let limit = arg_usize_opt(&args, "limit")?;
            to_json(backup_commands::backup_list(state, backup_type, limit).await?)
        }
        "backup_delete" => {
            let backup_path = arg_str(&args, "backupPath")?;
            to_json(backup_commands::backup_delete(state, backup_path).await?)
        }
        "backup_stats" => {
            to_json(backup_commands::backup_stats(state).await?)
        }
        "backup_verify" => {
            let backup_path = arg_str(&args, "backupPath")?;
            to_json(backup_commands::backup_verify(state, backup_path).await?)
        }

        // --- restore 命令（4）---
        "restore_from_backup" => {
            let backup_path = arg_str(&args, "backupPath")?;
            to_json(restore_commands::restore_from_backup(state, backup_path).await?)
        }
        "export_to_zip" => {
            let exported_by = arg_str_opt(&args, "exportedBy")?;
            to_json(restore_commands::export_to_zip(state, exported_by).await?)
        }
        "check_import_compatibility" => {
            let zip_path = arg_str(&args, "zipPath")?;
            to_json(restore_commands::check_import_compatibility(state, zip_path).await?)
        }
        "import_from_zip" => {
            let zip_path = arg_str(&args, "zipPath")?;
            to_json(restore_commands::import_from_zip(state, zip_path).await?)
        }

        // --- health_check 命令（2）---
        "health_check_run" => {
            to_json(health_check_commands::health_check_run(state).await?)
        }
        "health_check_repair" => {
            to_json(health_check_commands::health_check_repair(state).await?)
        }

        // --- font 命令（4）— font 命令原函数使用泛型 AppHandle<R>，直接传递 clone ---
        "font_list_custom" => {
            to_json(font_commands::font_list_custom(state, handle.clone()).await?)
        }
        "font_upload" => {
            let src_path = arg_str(&args, "srcPath")?;
            to_json(font_commands::font_upload(state, handle.clone(), src_path).await?)
        }
        "font_delete" => {
            let filename = arg_str(&args, "filename")?;
            to_json(font_commands::font_delete(state, handle.clone(), filename).await?)
        }
        "font_get_dir" => {
            to_json(font_commands::font_get_dir(state, handle.clone()).await?)
        }

        // --- custom_theme 命令（5）---
        "custom_theme_list" => {
            to_json(custom_theme_commands::custom_theme_list(state).await?)
        }
        "custom_theme_get" => {
            let id = arg_i64(&args, "id")?;
            to_json(custom_theme_commands::custom_theme_get(state, id).await?)
        }
        "custom_theme_upsert" => {
            let request: crate::models::custom_theme::UpsertCustomThemeRequest =
                serde_json::from_value(args.get("request").cloned().unwrap_or(Json::Null))
                    .map_err(|e| format!("参数 request 解析失败: {e}"))?;
            to_json(custom_theme_commands::custom_theme_upsert(state, request).await?)
        }
        "custom_theme_delete" => {
            let id = arg_i64(&args, "id")?;
            to_json(custom_theme_commands::custom_theme_delete(state, id).await?)
        }
        "custom_theme_migrate_local" => {
            let themes_json = arg_str(&args, "themesJson")?;
            to_json(custom_theme_commands::custom_theme_migrate_local(state, themes_json).await?)
        }

        _ => Err(format!("未知 systemtools 逻辑命令: {legacy}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        // system 10
        "get_system_config",
        "set_system_config",
        "get_all_system_configs",
        "system_open_file",
        "system_open_url",
        "system_get_app_info",
        "clipboard_write_text",
        "clipboard_read_text",
        "window_set_always_on_top",
        "window_screenshot",
        // perf 4
        "record_perf_metric",
        "perf_get_slow_queries",
        "perf_get_metrics_summary",
        "perf_get_metric_timeseries",
        // backup 5
        "backup_create_now",
        "backup_list",
        "backup_delete",
        "backup_stats",
        "backup_verify",
        // restore 4
        "restore_from_backup",
        "export_to_zip",
        "check_import_compatibility",
        "import_from_zip",
        // health_check 2
        "health_check_run",
        "health_check_repair",
        // font 4
        "font_list_custom",
        "font_upload",
        "font_delete",
        "font_get_dir",
        // custom_theme 5
        "custom_theme_list",
        "custom_theme_get",
        "custom_theme_upsert",
        "custom_theme_delete",
        "custom_theme_migrate_local",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 34);
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