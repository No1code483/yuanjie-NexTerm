//! terminal.linux 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发（阶段3 批次3b）。
//!
//! 与 boards.ai / boards.terminal 同构：alias 只做旧名 → 逻辑名的映射登记；真实 handler
//! 在 `dispatch_legacy` —— 闭包仅捕获 AppHandle，运行时经 `handle.state::<AppState>()`
//! 解析主应用真实 AppState（契约 06_Rust代码契约 §8.1）；业务实现复用 `linux_commands`
//! 原函数，返回值序列化与旧 IPC 路径一致（V1 输入/输出快照等价）。
//!
//! **参数键口径**：Tauri v2 默认 `ArgumentCase::Camel`；本 dispatcher 以 snake_case 为先、
//! camelCase 回退，兼容两种口径，不按 transport 静默改写输入。
//!
//! **前端零消费（裁定 T9）**：唯一消费者 components/Linux.tsx 为死文件随批判删，
//! 25 条全部「留 + alias 无前端方法」（43-A 口径）；S6 全量回放无覆盖损失。
//! **无 AppHandle / 无 emit**（S1 实测）→ S5 零改动；docker 系依赖本机 docker daemon，
//! 测试环境错误路径两轨同判。

pub const PLUGIN_ID: &str = "terminal.linux";
pub const SHORT_CODE: &str = "lx";

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
            new_command: concat!("lx:plugin:", $command),
        }
    };
}

/// 25 条 alias：linux_commands 全量（linux_* 19 + docker_* 5 + network_stats）。
/// `main.rs` 旧 transport 于本批 S7 移除。
pub const IPC_ALIASES: &[IpcAlias] = &[
    // 环境与状态（6）
    alias!("linux_get_environment"),
    alias!("linux_list_versions"),
    alias!("linux_get_status_panel"),
    alias!("linux_get_shell_status"),
    alias!("linux_get_system_info"),
    alias!("linux_get_iso_progress"),
    // 内核源码系（4）
    alias!("linux_list_directory"),
    alias!("linux_view_file"),
    alias!("linux_search_source"),
    alias!("linux_download_kernel"),
    // 内核管理系（3）
    alias!("linux_set_active"),
    alias!("linux_remove_kernel"),
    alias!("linux_build_kernel"),
    // 分析系（4）
    alias!("linux_analyze_config"),
    alias!("linux_list_modules"),
    alias!("linux_analyze_logs"),
    alias!("linux_perf_profile"),
    // 评测系（2）
    alias!("linux_run_benchmark"),
    alias!("linux_run_stress"),
    // Docker（5）
    alias!("docker_list_containers"),
    alias!("docker_container_start"),
    alias!("docker_container_stop"),
    alias!("docker_container_logs"),
    alias!("docker_list_images"),
    // 网络统计（1）
    alias!("network_stats"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::linux_commands;
use crate::db::connection::AppState;

/// `snake_case` → `camelCase`（仅用于参数键回退探测，不改变任何入参值）。
fn to_camel(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    let mut upper = false;
    for ch in key.chars() {
        if ch == '_' {
            upper = true;
            continue;
        }
        if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// 取参数：snake_case 优先，camelCase 回退。
fn get<'a>(args: &'a Json, key: &str) -> Option<&'a Json> {
    args.get(key).or_else(|| {
        let camel = to_camel(key);
        args.get(camel.as_str())
    })
}

fn arg_str(args: &Json, key: &str) -> Result<String, String> {
    get(args, key)
        .and_then(Json::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("参数 {key} 缺失或不是字符串"))
}

fn arg_opt_str(args: &Json, key: &str) -> Result<Option<String>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_str()
            .map(|s| Some(s.to_string()))
            .ok_or_else(|| format!("参数 {key} 不是字符串或 null")),
    }
}

fn arg_opt_bool(args: &Json, key: &str) -> Result<Option<bool>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| format!("参数 {key} 不是布尔或 null")),
    }
}

fn arg_opt_u32(args: &Json, key: &str) -> Result<Option<u32>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .map(Some)
            .ok_or_else(|| format!("参数 {key} 不是 u32 或 null")),
    }
}

fn arg_opt_usize(args: &Json, key: &str) -> Result<Option<usize>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .map(Some)
            .ok_or_else(|| format!("参数 {key} 不是 usize 或 null")),
    }
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
        // ===== 环境与状态 =====
        "linux_get_environment" => to_json(linux_commands::linux_get_environment(state).await?),
        "linux_list_versions" => to_json(linux_commands::linux_list_versions(state).await?),
        "linux_get_status_panel" => to_json(linux_commands::linux_get_status_panel(state).await?),
        "linux_get_shell_status" => to_json(linux_commands::linux_get_shell_status(state).await?),
        "linux_get_system_info" => to_json(linux_commands::linux_get_system_info(state).await?),
        "linux_get_iso_progress" => to_json(linux_commands::linux_get_iso_progress(state).await?),
        // ===== 内核源码系 =====
        "linux_list_directory" => {
            let version = arg_str(&args, "version")?;
            let path = arg_opt_str(&args, "path")?;
            to_json(linux_commands::linux_list_directory(state, version, path).await?)
        }
        "linux_view_file" => {
            let version = arg_str(&args, "version")?;
            let file_path = arg_str(&args, "file_path")?;
            to_json(linux_commands::linux_view_file(state, version, file_path).await?)
        }
        "linux_search_source" => {
            let version = arg_str(&args, "version")?;
            let query = arg_str(&args, "query")?;
            let max_results = arg_opt_usize(&args, "max_results")?;
            to_json(linux_commands::linux_search_source(state, version, query, max_results).await?)
        }
        "linux_download_kernel" => {
            let version = arg_str(&args, "version")?;
            to_json(linux_commands::linux_download_kernel(state, version).await?)
        }
        // ===== 内核管理系 =====
        "linux_set_active" => {
            let version = arg_str(&args, "version")?;
            to_json(linux_commands::linux_set_active(state, version).await?)
        }
        "linux_remove_kernel" => {
            let version = arg_str(&args, "version")?;
            to_json(linux_commands::linux_remove_kernel(state, version).await?)
        }
        "linux_build_kernel" => {
            let source_dir = arg_str(&args, "source_dir")?;
            let arch = arg_str(&args, "arch")?;
            let jobs = arg_opt_u32(&args, "jobs")?;
            to_json(linux_commands::linux_build_kernel(state, source_dir, arch, jobs).await?)
        }
        // ===== 分析系 =====
        "linux_analyze_config" => {
            let source_dir = arg_str(&args, "source_dir")?;
            to_json(linux_commands::linux_analyze_config(state, source_dir).await?)
        }
        "linux_list_modules" => to_json(linux_commands::linux_list_modules(state).await?),
        "linux_analyze_logs" => {
            let level = arg_opt_str(&args, "level")?;
            to_json(linux_commands::linux_analyze_logs(state, level).await?)
        }
        "linux_perf_profile" => {
            let duration_secs = arg_opt_u32(&args, "duration_secs")?;
            to_json(linux_commands::linux_perf_profile(state, duration_secs).await?)
        }
        // ===== 评测系 =====
        "linux_run_benchmark" => to_json(linux_commands::linux_run_benchmark(state).await?),
        "linux_run_stress" => to_json(linux_commands::linux_run_stress(state).await?),
        // ===== Docker =====
        "docker_list_containers" => {
            let all = arg_opt_bool(&args, "all")?;
            to_json(linux_commands::docker_list_containers(state, all).await?)
        }
        "docker_container_start" => {
            let container_id = arg_str(&args, "container_id")?;
            to_json(linux_commands::docker_container_start(state, container_id).await?)
        }
        "docker_container_stop" => {
            let container_id = arg_str(&args, "container_id")?;
            to_json(linux_commands::docker_container_stop(state, container_id).await?)
        }
        "docker_container_logs" => {
            let container_id = arg_str(&args, "container_id")?;
            let tail = arg_opt_u32(&args, "tail")?;
            to_json(linux_commands::docker_container_logs(state, container_id, tail).await?)
        }
        "docker_list_images" => to_json(linux_commands::docker_list_images(state).await?),
        // ===== 网络统计 =====
        "network_stats" => to_json(linux_commands::network_stats(state).await?),
        _ => Err(format!("未知 terminal.linux 逻辑命令: {legacy}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "linux_get_environment",
        "linux_list_versions",
        "linux_get_status_panel",
        "linux_get_shell_status",
        "linux_get_system_info",
        "linux_get_iso_progress",
        "linux_list_directory",
        "linux_view_file",
        "linux_search_source",
        "linux_download_kernel",
        "linux_set_active",
        "linux_remove_kernel",
        "linux_build_kernel",
        "linux_analyze_config",
        "linux_list_modules",
        "linux_analyze_logs",
        "linux_perf_profile",
        "linux_run_benchmark",
        "linux_run_stress",
        "docker_list_containers",
        "docker_container_start",
        "docker_container_stop",
        "docker_container_logs",
        "docker_list_images",
        "network_stats",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 25);
        assert_eq!(actual.len(), IPC_ALIASES.len(), "legacy_command 存在重复");
        assert_eq!(actual, expected);
    }

    #[test]
    fn alias_map_uses_formal_id_and_unique_short_code_commands() {
        let new_commands: HashSet<_> = IPC_ALIASES.iter().map(|alias| alias.new_command).collect();

        assert_eq!(new_commands.len(), IPC_ALIASES.len(), "new_command 存在重复");
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
