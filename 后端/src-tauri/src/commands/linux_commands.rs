use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::linux::{
    KernelBuildResult, KernelConfigResult, KernelLogAnalysis, KernelModuleInfo,
    LinuxBenchmarkResult, LinuxDownloadProgress, LinuxEnvironmentInfo, LinuxFileContent,
    LinuxIsoProgress, LinuxKernelVersion, LinuxSearchResponse, LinuxShellStatus,
    LinuxSourceDirectory, LinuxStatusPanel, LinuxStressResult, LinuxSystemInfo,
    PerfProfileResult,
};
use crate::services::linux_service;
use std::process::Command;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

// ========== 环境信息 ==========

#[tauri::command]
pub async fn linux_get_environment(state: State<'_, AppState>) -> Result<ApiResponse<LinuxEnvironmentInfo>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(linux_service::get_environment()))
}

// ========== 内核版本 ==========

#[tauri::command]
pub async fn linux_list_versions(state: State<'_, AppState>) -> Result<ApiResponse<Vec<LinuxKernelVersion>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(linux_service::list_versions()))
}

// ========== 状态面板 ==========

#[tauri::command]
pub async fn linux_get_status_panel(state: State<'_, AppState>) -> Result<ApiResponse<LinuxStatusPanel>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(linux_service::get_status_panel()))
}

// ========== Shell 状态 ==========

#[tauri::command]
pub async fn linux_get_shell_status(state: State<'_, AppState>) -> Result<ApiResponse<LinuxShellStatus>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(linux_service::get_shell_status()))
}

// ========== 系统信息 ==========

#[tauri::command]
pub async fn linux_get_system_info(state: State<'_, AppState>) -> Result<ApiResponse<LinuxSystemInfo>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(linux_service::get_system_info()))
}

// ========== ISO 进度 ==========

#[tauri::command]
pub async fn linux_get_iso_progress(state: State<'_, AppState>) -> Result<ApiResponse<LinuxIsoProgress>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(linux_service::get_iso_progress()))
}

// ========== 源码目录浏览 ==========

#[tauri::command]
pub async fn linux_list_directory(
    state: State<'_, AppState>,
    version: String,
    path: Option<String>,
) -> Result<ApiResponse<LinuxSourceDirectory>, String> {
    crate::commands::common::require_auth(&state).await?;
    linux_service::list_directory(&version, path.as_deref())
        .map(ApiResponse::success)
}

// ========== 文件查看 ==========

#[tauri::command]
pub async fn linux_view_file(
    state: State<'_, AppState>,
    version: String,
    file_path: String,
) -> Result<ApiResponse<LinuxFileContent>, String> {
    crate::commands::common::require_auth(&state).await?;
    linux_service::view_file(&version, &file_path)
        .map(ApiResponse::success)
}

// ========== 源码搜索 ==========

#[tauri::command]
pub async fn linux_search_source(
    state: State<'_, AppState>,
    version: String,
    query: String,
    max_results: Option<usize>,
) -> Result<ApiResponse<LinuxSearchResponse>, String> {
    crate::commands::common::require_auth(&state).await?;
    linux_service::search_source(&version, &query, max_results.unwrap_or(50))
        .map(ApiResponse::success)
}

// ========== 内核下载 ==========

#[tauri::command]
pub async fn linux_download_kernel(
    state: State<'_, AppState>,
    version: String,
) -> Result<ApiResponse<LinuxDownloadProgress>, String> {
    crate::commands::common::require_auth(&state).await?;
    linux_service::download_kernel(&version)
        .await
        .map(ApiResponse::success)
}

// ========== 设置活跃版本 ==========

#[tauri::command]
pub async fn linux_set_active(state: State<'_, AppState>, version: String) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    linux_service::set_active_version(&version)
        .map(|_| ApiResponse::success_msg("已切换活跃版本"))
}

// ========== 移除内核版本 ==========

#[tauri::command]
pub async fn linux_remove_kernel(state: State<'_, AppState>, version: String) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    linux_service::remove_kernel(&version)
        .map(|_| ApiResponse::success_msg("内核版本已移除"))
}

// ========== 内核构建 ==========

#[tauri::command]
pub async fn linux_build_kernel(
    state: State<'_, AppState>,
    source_dir: String,
    arch: String,
    jobs: Option<u32>,
) -> Result<ApiResponse<KernelBuildResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(linux_service::build_kernel(
        &source_dir,
        &arch,
        jobs.unwrap_or(4),
    )))
}

// ========== 内核配置分析 ==========

#[tauri::command]
pub async fn linux_analyze_config(
    state: State<'_, AppState>,
    source_dir: String,
) -> Result<ApiResponse<KernelConfigResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(linux_service::analyze_config(&source_dir)))
}

// ========== 内核模块列表 ==========

#[tauri::command]
pub async fn linux_list_modules(state: State<'_, AppState>) -> Result<ApiResponse<Vec<KernelModuleInfo>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(linux_service::list_modules()))
}

// ========== 日志分析 ==========

#[tauri::command]
pub async fn linux_analyze_logs(
    state: State<'_, AppState>,
    level: Option<String>,
) -> Result<ApiResponse<KernelLogAnalysis>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(linux_service::analyze_logs(
        level.as_deref(),
    )))
}

// ========== 性能分析 ==========

#[tauri::command]
pub async fn linux_perf_profile(
    state: State<'_, AppState>,
    duration_secs: Option<u32>,
) -> Result<ApiResponse<PerfProfileResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(linux_service::perf_profile(
        duration_secs.unwrap_or(30),
    )))
}

// ========== 基准评测 ==========

#[tauri::command]
pub async fn linux_run_benchmark(state: State<'_, AppState>) -> Result<ApiResponse<LinuxBenchmarkResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(linux_service::run_benchmark()))
}

// ========== 压力测试 ==========

#[tauri::command]
pub async fn linux_run_stress(state: State<'_, AppState>) -> Result<ApiResponse<LinuxStressResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(linux_service::run_stress()))
}

// ========== Task 12.1: Docker Commands ==========

fn run_docker_cmd(args: &[&str]) -> Result<String, String> {
    let output = Command::new("docker")
        .args(args)
        .output()
        .map_err(|e| format!("Docker 命令执行失败: {}", e))?;
    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|e| format!("UTF-8 解析失败: {}", e))
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Docker 错误: {}", err))
    }
}

#[tauri::command]
pub async fn docker_list_containers(
    state: State<'_, AppState>,
    all: Option<bool>,
) -> Result<ApiResponse<Vec<serde_json::Value>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let show_all = all.unwrap_or(true);
    let mut args = vec!["ps"];
    if show_all { args.push("-a"); }
    args.extend_from_slice(&["--format", "{{json .}}"]);
    match run_docker_cmd(&args) {
        Ok(output) => {
            let containers: Vec<serde_json::Value> = output
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .collect();
            Ok(ApiResponse::success(containers))
        }
        Err(e) => Ok(ApiResponse::error(5001, &e)),
    }
}

#[tauri::command]
pub async fn docker_container_start(
    state: State<'_, AppState>,
    container_id: String,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    match run_docker_cmd(&["start", &container_id]) {
        Ok(_) => Ok(ApiResponse::success_msg("容器已启动")),
        Err(e) => Ok(ApiResponse::error(5001, &e)),
    }
}

#[tauri::command]
pub async fn docker_container_stop(
    state: State<'_, AppState>,
    container_id: String,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    match run_docker_cmd(&["stop", &container_id]) {
        Ok(_) => Ok(ApiResponse::success_msg("容器已停止")),
        Err(e) => Ok(ApiResponse::error(5001, &e)),
    }
}

#[tauri::command]
pub async fn docker_container_logs(
    state: State<'_, AppState>,
    container_id: String,
    tail: Option<u32>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut args: Vec<String> = vec!["logs".to_string()];
    if let Some(n) = tail {
        args.push("--tail".to_string());
        args.push(n.to_string());
    }
    args.push(container_id);
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    match run_docker_cmd(&args_str) {
        Ok(logs) => Ok(ApiResponse::success(logs)),
        Err(e) => Ok(ApiResponse::error(5001, &e)),
    }
}

#[tauri::command]
pub async fn docker_list_images(state: State<'_, AppState>) -> Result<ApiResponse<Vec<serde_json::Value>>, String> {
    crate::commands::common::require_auth(&state).await?;
    match run_docker_cmd(&["images", "--format", "{{json .}}"]) {
        Ok(output) => {
            let images: Vec<serde_json::Value> = output
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .collect();
            Ok(ApiResponse::success(images))
        }
        Err(e) => Ok(ApiResponse::error(5001, &e)),
    }
}

// ========== Task 12.3: Network Monitoring ==========

#[tauri::command]
pub async fn network_stats(state: State<'_, AppState>) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();

    // Get network stats from sysinfo
    let networks = sysinfo::Networks::new_with_refreshed_list();
    let mut total_received: u64 = 0;
    let mut total_transmitted: u64 = 0;

    for (_name, data) in networks.iter() {
        total_received += data.total_received();
        total_transmitted += data.total_transmitted();
    }

    // Get connection count via netstat on Windows
    let connections_count = Command::new("netstat")
        .args(["-an"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.lines().filter(|l| l.starts_with("  TCP") || l.starts_with("  UDP")).count() as u64)
        .unwrap_or(0);

    let stats = serde_json::json!({
        "total_received": total_received,
        "total_transmitted": total_transmitted,
        "total_received_mb": format!("{:.2}", total_received as f64 / 1_048_576.0),
        "total_transmitted_mb": format!("{:.2}", total_transmitted as f64 / 1_048_576.0),
        "connections_count": connections_count,
    });

    Ok(ApiResponse::success(stats))
}