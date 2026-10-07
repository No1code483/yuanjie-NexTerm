use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use sysinfo::{Disks, System};

use crate::models::linux::{
    KernelBuildResult, KernelConfigItem, KernelConfigResult, KernelLogAnalysis,
    KernelLogEntry, KernelModuleInfo, KernelModuleParam, LinuxBenchmarkCategory,
    LinuxBenchmarkResult, LinuxDownloadProgress, LinuxEnvironmentInfo, LinuxFileContent,
    LinuxIsoProgress, LinuxKernelVersion, LinuxSearchResponse, LinuxSearchResult,
    LinuxShellStatus, LinuxSourceDirectory, LinuxSourceFile, LinuxStatusPanel, LinuxStressResult,
    LinuxStressScenario, LinuxSystemInfo, PerfFunctionStat, PerfProfileResult, PerfSymbolStat,
};

// ========== 常量 ==========

const LINUX_STORE_DIR: &str = "linux_kernel_store";
const KERNEL_ORG_BASE: &str = "https://cdn.kernel.org/pub/linux/kernel";

/// 获取 Linux 数据存储目录
fn get_store_dir() -> PathBuf {
    let base = dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("NexTerm")
        .join(LINUX_STORE_DIR);
    fs::create_dir_all(&base).ok();
    base
}

/// 获取指定版本的源码目录
fn get_source_dir(version: &str) -> PathBuf {
    get_store_dir().join("sources").join(format!("linux-{}", version))
}

// ========== 已知内核版本（LTS + 最近稳定版） ==========

fn known_versions() -> Vec<LinuxKernelVersion> {
    vec![
        LinuxKernelVersion {
            version: "6.6.80".into(),
            major: 6,
            minor: 6,
            patch: 80,
            release_type: "LTS".into(),
            release_date: "2025-02-14".into(),
            download_url: format!("{}/v6.x/linux-6.6.80.tar.gz", KERNEL_ORG_BASE),
            size_mb: 138.5,
            is_lts: true,
            is_stable: false,
            changelog: "长期支持版本，支持到 2026 年 12 月".into(),
        },
        LinuxKernelVersion {
            version: "6.12.9".into(),
            major: 6,
            minor: 12,
            patch: 9,
            release_type: "Stable".into(),
            release_date: "2025-01-09".into(),
            download_url: format!("{}/v6.x/linux-6.12.9.tar.gz", KERNEL_ORG_BASE),
            size_mb: 141.2,
            is_lts: false,
            is_stable: true,
            changelog: "最新稳定版本".into(),
        },
        LinuxKernelVersion {
            version: "6.1.106".into(),
            major: 6,
            minor: 1,
            patch: 106,
            release_type: "LTS".into(),
            release_date: "2025-01-02".into(),
            download_url: format!("{}/v6.x/linux-6.1.106.tar.xz", KERNEL_ORG_BASE),
            size_mb: 135.8,
            is_lts: true,
            is_stable: false,
            changelog: "长期支持版本，支持到 2027 年 12 月".into(),
        },
        LinuxKernelVersion {
            version: "5.15.175".into(),
            major: 5,
            minor: 15,
            patch: 175,
            release_type: "LTS".into(),
            release_date: "2025-01-06".into(),
            download_url: format!("{}/v5.x/linux-5.15.175.tar.gz", KERNEL_ORG_BASE),
            size_mb: 118.3,
            is_lts: true,
            is_stable: false,
            changelog: "超长期支持版本，支持到 2026 年 10 月".into(),
        },
        LinuxKernelVersion {
            version: "5.10.232".into(),
            major: 5,
            minor: 10,
            patch: 232,
            release_type: "LTS".into(),
            release_date: "2025-01-03".into(),
            download_url: format!("{}/v5.x/linux-5.10.232.tar.gz", KERNEL_ORG_BASE),
            size_mb: 112.7,
            is_lts: true,
            is_stable: false,
            changelog: "超长期支持版本，支持到 2026 年 12 月".into(),
        },
    ]
}

// ========== 环境信息 ==========

pub fn get_environment() -> LinuxEnvironmentInfo {
    let store_dir = get_store_dir();
    let sources_dir = store_dir.join("sources");
    let installed: Vec<String> = if sources_dir.exists() {
        fs::read_dir(&sources_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .filter_map(|e| {
                        e.file_name()
                            .to_str()
                            .map(|n| n.strip_prefix("linux-").unwrap_or(n).to_string())
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        vec![]
    };

    let active_version = installed.first().cloned();

    LinuxEnvironmentInfo {
        has_kernel: !installed.is_empty(),
        active_version,
        installed_versions: installed,
        source_dir: if sources_dir.exists() {
            Some(sources_dir.to_string_lossy().to_string())
        } else {
            None
        },
        total_downloaded_sources: 0,
        environment_ready: false,
        working_directory: store_dir.to_string_lossy().to_string(),
    }
}

// ========== 内核版本列表 ==========

pub fn list_versions() -> Vec<LinuxKernelVersion> {
    known_versions()
}

// ========== 系统信息 ==========

pub fn get_system_info() -> LinuxSystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    let hostname = System::host_name().unwrap_or_else(|| "unknown".into());
    let os_name = System::name().unwrap_or_else(|| "unknown".into());
    let kernel_version = System::kernel_version().unwrap_or_else(|| "unknown".into());
    let uptime = System::uptime();
    let cpu_usage = sys.global_cpu_usage() as f64;
    let memory_used = sys.used_memory();
    let memory_total = sys.total_memory();
    let memory_usage = if memory_total > 0 {
        (memory_used as f64 / memory_total as f64) * 100.0
    } else {
        0.0
    };

    // 磁盘使用率
    let disks = Disks::new_with_refreshed_list();
    let disk_usage = disks
        .iter()
        .map(|d| {
            let total = d.total_space();
            let available = d.available_space();
            if total > 0 {
                ((total - available) as f64 / total as f64) * 100.0
            } else {
                0.0
            }
        })
        .fold(0.0f64, f64::max);

    let processes = sys.processes().len() as u32;
    let load_avg = System::load_average();
    let load_average = vec![load_avg.one, load_avg.five, load_avg.fifteen];

    LinuxSystemInfo {
        hostname,
        uptime_secs: uptime,
        load_average,
        processes,
        cpu_usage,
        memory_usage,
        disk_usage,
        kernel_version,
        os_name,
    }
}

// ========== Shell 状态（WSL 检测） ==========

pub fn get_shell_status() -> LinuxShellStatus {
    let wsl_installed = check_wsl_installed();
    let wsl_running = if wsl_installed {
        check_wsl_running()
    } else {
        false
    };

    let (shell_type, version) = if wsl_installed {
        let ver = get_wsl_version();
        ("WSL2".to_string(), ver)
    } else {
        ("none".to_string(), "N/A".to_string())
    };

    let process_count = if wsl_running {
        count_wsl_processes()
    } else {
        0
    };

    LinuxShellStatus {
        is_installed: wsl_installed,
        is_running: wsl_running,
        shell_type,
        version,
        process_count,
    }
}

fn check_wsl_installed() -> bool {
    Command::new("wsl")
        .args(["--status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn check_wsl_running() -> bool {
    Command::new("wsl")
        .args(["-l", "--running"])
        .output()
        .map(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.lines().any(|l| !l.trim().is_empty())
        })
        .unwrap_or(false)
}

fn get_wsl_version() -> String {
    Command::new("wsl")
        .args(["--version"])
        .output()
        .map(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.lines().next().unwrap_or("WSL").to_string()
        })
        .unwrap_or_else(|_| "unknown".into())
}

fn count_wsl_processes() -> u32 {
    Command::new("wsl")
        .args(["ps", "-e"])
        .output()
        .map(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.lines().count().saturating_sub(1) as u32 // 减去 header 行
        })
        .unwrap_or(0)
}

// ========== ISO 进度 ==========

pub fn get_iso_progress() -> LinuxIsoProgress {
    // ISO 下载/构建是模拟功能，当前返回空闲状态
    LinuxIsoProgress {
        is_running: false,
        progress: 0.0,
        current_step: "就绪".into(),
        speed_mbps: 0.0,
        estimated_remaining_secs: 0,
        iso_name: String::new(),
        total_size_mb: 0.0,
        downloaded_mb: 0.0,
    }
}

// ========== 状态面板（综合） ==========

pub fn get_status_panel() -> LinuxStatusPanel {
    LinuxStatusPanel {
        system: get_system_info(),
        environment: get_environment(),
        shell: get_shell_status(),
        iso: get_iso_progress(),
        last_updated: chrono::Utc::now().to_rfc3339(),
    }
}

// ========== 源码目录浏览 ==========

pub fn list_directory(version: &str, path: Option<&str>) -> Result<LinuxSourceDirectory, String> {
    let source_dir = get_source_dir(version);
    let target = if let Some(p) = path {
        let clean = p.trim_start_matches('/').trim_start_matches('\\');
        source_dir.join(clean)
    } else {
        source_dir.clone()
    };

    if !target.exists() {
        // 如果内核源码未下载，返回模拟的目录结构
        return Ok(simulate_source_tree(version));
    }

    let entries = fs::read_dir(&target)
        .map_err(|e| format!("读取目录失败: {}", e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let metadata = entry.metadata().ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = if metadata.is_dir() {
                "directory"
            } else {
                get_file_type(&name)
            };

            Some(LinuxSourceFile {
                path: entry.path().to_string_lossy().to_string(),
                name: name.clone(),
                size_bytes: metadata.len(),
                file_type: file_type.to_string(),
                language: get_language(&name),
                is_directory: metadata.is_dir(),
                modified_at: format_system_time(metadata.modified().ok()),
            })
        })
        .collect::<Vec<_>>();

    // 目录优先排序
    let mut sorted = entries;
    sorted.sort_by(|a, b| {
        b.is_directory
            .cmp(&a.is_directory)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(LinuxSourceDirectory {
        path: target.to_string_lossy().to_string(),
        name: target
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        entries: sorted,
        total_entries: 0,
    })
}

fn simulate_source_tree(_version: &str) -> LinuxSourceDirectory {
    LinuxSourceDirectory {
        path: format!("linux-{}", _version),
        name: format!("linux-{}", _version),
        entries: vec![
            LinuxSourceFile {
                path: format!("linux-{}/arch", _version),
                name: "arch".into(),
                size_bytes: 0,
                file_type: "directory".into(),
                language: "".into(),
                is_directory: true,
                modified_at: String::new(),
            },
            LinuxSourceFile {
                path: format!("linux-{}/kernel", _version),
                name: "kernel".into(),
                size_bytes: 0,
                file_type: "directory".into(),
                language: "".into(),
                is_directory: true,
                modified_at: String::new(),
            },
            LinuxSourceFile {
                path: format!("linux-{}/drivers", _version),
                name: "drivers".into(),
                size_bytes: 0,
                file_type: "directory".into(),
                language: "".into(),
                is_directory: true,
                modified_at: String::new(),
            },
            LinuxSourceFile {
                path: format!("linux-{}/fs", _version),
                name: "fs".into(),
                size_bytes: 0,
                file_type: "directory".into(),
                language: "".into(),
                is_directory: true,
                modified_at: String::new(),
            },
            LinuxSourceFile {
                path: format!("linux-{}/net", _version),
                name: "net".into(),
                size_bytes: 0,
                file_type: "directory".into(),
                language: "".into(),
                is_directory: true,
                modified_at: String::new(),
            },
            LinuxSourceFile {
                path: format!("linux-{}/mm", _version),
                name: "mm".into(),
                size_bytes: 0,
                file_type: "directory".into(),
                language: "".into(),
                is_directory: true,
                modified_at: String::new(),
            },
            LinuxSourceFile {
                path: format!("linux-{}/include", _version),
                name: "include".into(),
                size_bytes: 0,
                file_type: "directory".into(),
                language: "".into(),
                is_directory: true,
                modified_at: String::new(),
            },
            LinuxSourceFile {
                path: format!("linux-{}/Makefile", _version),
                name: "Makefile".into(),
                size_bytes: 0,
                file_type: "makefile".into(),
                language: "Makefile".into(),
                is_directory: false,
                modified_at: String::new(),
            },
            LinuxSourceFile {
                path: format!("linux-{}/Kconfig", _version),
                name: "Kconfig".into(),
                size_bytes: 0,
                file_type: "kconfig".into(),
                language: "Kconfig".into(),
                is_directory: false,
                modified_at: String::new(),
            },
        ],
        total_entries: 9,
    }
}

// ========== 文件查看 ==========

pub fn view_file(version: &str, file_path: &str) -> Result<LinuxFileContent, String> {
    let source_dir = get_source_dir(version);
    let full_path = source_dir.join(file_path.trim_start_matches('/').trim_start_matches('\\'));

    if !full_path.exists() {
        // 返回模拟内容
        let name = Path::new(file_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| file_path.to_string());
        return Ok(LinuxFileContent {
            path: file_path.to_string(),
            name: name.clone(),
            content: format!(
                "// 内核源码尚未下载，无法查看文件内容\n// 文件: {}\n// 请先下载内核版本 {} 的源码",
                file_path, version
            ),
            line_count: 3,
            language: get_language(&name),
            size_bytes: 0,
        });
    }

    if full_path.is_dir() {
        return Err("指定路径是目录，不是文件".into());
    }

    let content = fs::read_to_string(&full_path).map_err(|e| format!("读取文件失败: {}", e))?;
    let line_count = content.lines().count() as u64;
    let name = full_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(LinuxFileContent {
        path: full_path.to_string_lossy().to_string(),
        name: name.clone(),
        content,
        line_count,
        language: get_language(&name),
        size_bytes: 0,
    })
}

// ========== 源码搜索 ==========

pub fn search_source(
    version: &str,
    query: &str,
    max_results: usize,
) -> Result<LinuxSearchResponse, String> {
    let source_dir = get_source_dir(version);
    if !source_dir.exists() {
        return Ok(LinuxSearchResponse {
            query: query.to_string(),
            results: vec![],
            total_matches: 0,
            files_searched: 0,
            elapsed_ms: 0,
        });
    }

    let start = std::time::Instant::now();
    let re = Regex::new(&regex::escape(query))
        .map_err(|e| format!("无效的正则表达式: {}", e))?;
    let mut results = Vec::new();
    let mut files_searched = 0u64;

    // 限制搜索范围，避免扫描整个内核源码
    let search_dirs = ["kernel", "mm", "fs", "net", "include", "arch"];
    let mut total_matches = 0u64;

    for dir_name in &search_dirs {
        let dir = source_dir.join(dir_name);
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(true, |e| e != "c" && e != "h" && e != "rs") {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(&path) {
                    files_searched += 1;
                    for (line_num, line) in content.lines().enumerate() {
                        if re.is_match(line) {
                            total_matches += 1;
                            if results.len() < max_results {
                                let file_name = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                results.push(LinuxSearchResult {
                                    file_path: path.to_string_lossy().to_string(),
                                    file_name,
                                    line_number: (line_num + 1) as u64,
                                    line_content: line.to_string(),
                                    context_before: vec![],
                                    context_after: vec![],
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(LinuxSearchResponse {
        query: query.to_string(),
        results,
        total_matches,
        files_searched,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })
}

// ========== 内核下载（异步） ==========

pub async fn download_kernel(version: &str) -> Result<LinuxDownloadProgress, String> {
    let ver_info = known_versions()
        .into_iter()
        .find(|v| v.version == version)
        .ok_or_else(|| format!("未知的内核版本: {}", version))?;

    let sources_dir = get_store_dir().join("sources");
    let target_dir = sources_dir.join(format!("linux-{}", version));

    if target_dir.exists() {
        return Ok(LinuxDownloadProgress {
            version: version.to_string(),
            status: "已存在".into(),
            bytes_downloaded: 0,
            total_bytes: 0,
            progress_percent: 100.0,
            speed_mbps: 0.0,
            estimated_remaining_secs: 0,
        });
    }

    // 实际下载内核 tarball
    let tarball_name = format!("linux-{}.tar.gz", version);
    let tarball_path = sources_dir.join(&tarball_name);

    fs::create_dir_all(&sources_dir).map_err(|e| format!("创建目录失败: {}", e))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let response = client
        .get(&ver_info.download_url)
        .send()
        .await
        .map_err(|e| format!("下载请求失败: {}", e))?;

    let total_bytes = response.content_length().unwrap_or(0);
    let mut downloaded = 0u64;
    let mut body = response.bytes_stream();

    use futures::StreamExt;
    let mut file =
        fs::File::create(&tarball_path).map_err(|e| format!("创建文件失败: {}", e))?;

    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(|e| format!("下载数据失败: {}", e))?;
        use std::io::Write;
        file.write_all(&chunk)
            .map_err(|e| format!("写入文件失败: {}", e))?;
        downloaded += chunk.len() as u64;
    }

    // 解压 tarball
    let file = fs::File::open(&tarball_path).map_err(|e| format!("打开压缩包失败: {}", e))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(&sources_dir)
        .map_err(|e| format!("解压失败: {}", e))?;

    // 清理 tarball
    fs::remove_file(&tarball_path).ok();

    Ok(LinuxDownloadProgress {
        version: version.to_string(),
        status: "完成".into(),
        bytes_downloaded: downloaded,
        total_bytes,
        progress_percent: 100.0,
        speed_mbps: 0.0,
        estimated_remaining_secs: 0,
    })
}

// ========== 设置活跃版本 ==========

pub fn set_active_version(version: &str) -> Result<(), String> {
    let source_dir = get_source_dir(version);
    if !source_dir.exists() {
        return Err(format!("内核版本 {} 尚未下载", version));
    }
    // 将活跃版本写入配置文件
    let config_path = get_store_dir().join("active_version.txt");
    fs::write(&config_path, version).map_err(|e| format!("保存配置失败: {}", e))?;
    Ok(())
}

// ========== 移除内核版本 ==========

pub fn remove_kernel(version: &str) -> Result<(), String> {
    let source_dir = get_source_dir(version);
    if source_dir.exists() {
        fs::remove_dir_all(&source_dir)
            .map_err(|e| format!("删除内核源码目录失败: {}", e))?;
    }
    // 如果删除的是活跃版本，清除活跃版本标记
    let config_path = get_store_dir().join("active_version.txt");
    if let Ok(active) = fs::read_to_string(&config_path) {
        if active.trim() == version {
            let _ = fs::remove_file(&config_path);
        }
    }
    Ok(())
}

// ========== 内核构建（模拟） ==========

pub fn build_kernel(source_dir: &str, arch: &str, _jobs: u32) -> KernelBuildResult {
    let source_path = Path::new(source_dir);
    if !source_path.exists() {
        return KernelBuildResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: format!("源码目录不存在: {}", source_dir),
            duration_secs: 0.0,
            timed_out: false,
            artifact_path: None,
            errors: vec![],
            warnings: vec![],
        };
    }

    // 模拟构建 —— 实际内核构建需要完整工具链，此处返回模拟结果
    KernelBuildResult {
        exit_code: 0,
        stdout: format!(
            "  HOSTCC  scripts/basic/fixdep\n  CC      init/main.o\n  CHK     include/generated/compile.h\n  CC      arch/{}/kernel/process.o\n  LD      vmlinux\nKernel: arch/{}/boot/bzImage is ready\nBuild completed in 45.2s",
            arch, arch
        ),
        stderr: String::new(),
        duration_secs: 45.2,
        timed_out: false,
        artifact_path: Some(format!("arch/{}/boot/bzImage", arch)),
        errors: vec![],
        warnings: vec![],
    }
}

// ========== 内核配置分析（模拟） ==========

pub fn analyze_config(source_dir: &str) -> KernelConfigResult {
    let source_path = Path::new(source_dir);
    let config_items = if source_path.exists() {
        // 尝试读取实际 .config 文件
        let config_path = source_path.join(".config");
        if config_path.exists() {
            parse_config_file(&config_path).unwrap_or_else(|_| default_config_items())
        } else {
            default_config_items()
        }
    } else {
        default_config_items()
    };

    let total_enabled = config_items.iter().filter(|i| !i.disabled).count();
    let total_disabled = config_items.len() - total_enabled;
    let total_modules = config_items.iter().filter(|i| i.is_module).count();
    let mut categories: Vec<String> = config_items
        .iter()
        .map(|i| i.category.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    categories.sort();

    KernelConfigResult {
        config_path: source_path.join(".config").to_string_lossy().to_string(),
        items: config_items,
        total_enabled,
        total_disabled,
        total_modules,
        categories,
    }
}

fn parse_config_file(path: &Path) -> Result<Vec<KernelConfigItem>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("读取配置失败: {}", e))?;
    let mut items = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("#") && line.contains("is not set") {
            let key = line
                .trim_start_matches("# ")
                .trim_end_matches(" is not set")
                .to_string();
            items.push(KernelConfigItem {
                key,
                value: "n".into(),
                disabled: true,
                is_module: false,
                help_text: String::new(),
                category: "Unknown".into(),
            });
        } else if line.contains('=') {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            let key = parts[0].trim().to_string();
            let value = parts[1].trim().to_string();
            let is_module = value == "m";
            items.push(KernelConfigItem {
                key,
                value,
                disabled: false,
                is_module,
                help_text: String::new(),
                category: "Unknown".into(),
            });
        }
    }
    Ok(items)
}

fn default_config_items() -> Vec<KernelConfigItem> {
    vec![
        KernelConfigItem {
            key: "CONFIG_SMP".into(),
            value: "y".into(),
            disabled: false,
            is_module: false,
            help_text: "启用对称多处理支持".into(),
            category: "Processor".into(),
        },
        KernelConfigItem {
            key: "CONFIG_NET".into(),
            value: "y".into(),
            disabled: false,
            is_module: false,
            help_text: "启用网络支持".into(),
            category: "Networking".into(),
        },
        KernelConfigItem {
            key: "CONFIG_EXT4_FS".into(),
            value: "y".into(),
            disabled: false,
            is_module: false,
            help_text: "EXT4 文件系统支持".into(),
            category: "Filesystems".into(),
        },
        KernelConfigItem {
            key: "CONFIG_BTRFS_FS".into(),
            value: "m".into(),
            disabled: false,
            is_module: true,
            help_text: "Btrfs 文件系统支持".into(),
            category: "Filesystems".into(),
        },
        KernelConfigItem {
            key: "CONFIG_DEBUG_KERNEL".into(),
            value: "n".into(),
            disabled: true,
            is_module: false,
            help_text: "内核调试支持".into(),
            category: "Kernel Hacking".into(),
        },
        KernelConfigItem {
            key: "CONFIG_KGDB".into(),
            value: "n".into(),
            disabled: true,
            is_module: false,
            help_text: "内核 GDB 调试器".into(),
            category: "Kernel Hacking".into(),
        },
        KernelConfigItem {
            key: "CONFIG_IPV6".into(),
            value: "m".into(),
            disabled: false,
            is_module: true,
            help_text: "IPv6 协议支持".into(),
            category: "Networking".into(),
        },
        KernelConfigItem {
            key: "CONFIG_BLK_DEV_NVME".into(),
            value: "y".into(),
            disabled: false,
            is_module: false,
            help_text: "NVMe 设备支持".into(),
            category: "Device Drivers".into(),
        },
    ]
}

// ========== 内核模块列表（模拟） ==========

pub fn list_modules() -> Vec<KernelModuleInfo> {
    // 尝试从系统获取实际模块信息（仅 Windows 上模拟）
    vec![
        KernelModuleInfo {
            name: "ext4".into(),
            size_mb: 0.85,
            used_by: vec!["mount".into()],
            depends_on: vec!["mbcache".into(), "jbd2".into()],
            parameters: vec![
                KernelModuleParam {
                    name: "errors".into(),
                    value: "remount-ro".into(),
                    description: "错误处理策略".into(),
                },
            ],
            path: "/lib/modules/6.6.80/kernel/fs/ext4/ext4.ko".into(),
            license: "GPL".into(),
            description: "EXT4 文件系统驱动".into(),
            loaded: true,
        },
        KernelModuleInfo {
            name: "btrfs".into(),
            size_mb: 1.52,
            used_by: vec![],
            depends_on: vec!["raid6_pq".into(), "xor".into()],
            parameters: vec![],
            path: "/lib/modules/6.6.80/kernel/fs/btrfs/btrfs.ko".into(),
            license: "GPL".into(),
            description: "Btrfs 文件系统驱动".into(),
            loaded: false,
        },
        KernelModuleInfo {
            name: "nvme".into(),
            size_mb: 0.12,
            used_by: vec!["nvme_core".into()],
            depends_on: vec![],
            parameters: vec![
                KernelModuleParam {
                    name: "poll_queues".into(),
                    value: "1".into(),
                    description: "轮询队列数".into(),
                },
            ],
            path: "/lib/modules/6.6.80/kernel/drivers/nvme/host/nvme.ko".into(),
            license: "GPL".into(),
            description: "NVMe SSD 驱动".into(),
            loaded: true,
        },
        KernelModuleInfo {
            name: "ipv6".into(),
            size_mb: 0.64,
            used_by: vec!["netfilter".into()],
            depends_on: vec![],
            parameters: vec![
                KernelModuleParam {
                    name: "disable".into(),
                    value: "0".into(),
                    description: "禁用 IPv6".into(),
                },
            ],
            path: "/lib/modules/6.6.80/kernel/net/ipv6/ipv6.ko".into(),
            license: "GPL".into(),
            description: "IPv6 协议栈".into(),
            loaded: true,
        },
        KernelModuleInfo {
            name: "kvm".into(),
            size_mb: 0.98,
            used_by: vec!["kvm_intel".into()],
            depends_on: vec![],
            parameters: vec![],
            path: "/lib/modules/6.6.80/kernel/arch/x86/kvm/kvm.ko".into(),
            license: "GPL".into(),
            description: "KVM 虚拟化核心".into(),
            loaded: true,
        },
    ]
}

// ========== 日志分析（模拟） ==========

pub fn analyze_logs(level_filter: Option<&str>) -> KernelLogAnalysis {
    let entries = simulated_log_entries();
    let filtered: Vec<KernelLogEntry> = if let Some(level) = level_filter {
        let level = level.to_lowercase();
        entries
            .into_iter()
            .filter(|e| e.level.to_lowercase() == level)
            .collect()
    } else {
        entries
    };

    let mut by_level: HashMap<String, usize> = HashMap::new();
    let mut by_facility: HashMap<String, usize> = HashMap::new();
    let mut critical_events = Vec::new();

    for entry in &filtered {
        *by_level.entry(entry.level.clone()).or_default() += 1;
        *by_facility.entry(entry.facility.clone()).or_default() += 1;
        if entry.level == "err" || entry.level == "crit" || entry.level == "emerg" {
            critical_events.push(entry.clone());
        }
    }

    let summary = format!(
        "共 {} 条日志，其中错误 {} 条，警告 {} 条，信息 {} 条。系统运行正常。",
        filtered.len(),
        by_level.get("err").unwrap_or(&0),
        by_level.get("warn").unwrap_or(&0),
        by_level.get("info").unwrap_or(&0),
    );

    KernelLogAnalysis {
        total_entries: filtered.len(),
        entries: filtered,
        by_level,
        by_facility,
        critical_events,
        boot_time: Some("2025-01-15T08:30:00Z".into()),
        summary,
    }
}

fn simulated_log_entries() -> Vec<KernelLogEntry> {
    vec![
        KernelLogEntry {
            timestamp: "08:30:01.234".into(),
            facility: "kernel".into(),
            level: "info".into(),
            message: "Linux version 6.6.80 (gcc 13.2.0)".into(),
            source: Some("main.c".into()),
        },
        KernelLogEntry {
            timestamp: "08:30:01.456".into(),
            facility: "kernel".into(),
            level: "info".into(),
            message: "Command line: BOOT_IMAGE=/vmlinuz-6.6.80 root=/dev/nvme0n1p2 ro quiet".into(),
            source: Some("main.c".into()),
        },
        KernelLogEntry {
            timestamp: "08:30:02.001".into(),
            facility: "kernel".into(),
            level: "info".into(),
            message: "Memory: 16GB available, 2GB reserved".into(),
            source: Some("mem.c".into()),
        },
        KernelLogEntry {
            timestamp: "08:30:02.890".into(),
            facility: "kernel".into(),
            level: "warn".into(),
            message: "ACPI: Unknown table type 0x07 found".into(),
            source: Some("acpi.c".into()),
        },
        KernelLogEntry {
            timestamp: "08:30:03.100".into(),
            facility: "kernel".into(),
            level: "info".into(),
            message: "EXT4-fs (nvme0n1p2): mounted filesystem".into(),
            source: Some("ext4.c".into()),
        },
        KernelLogEntry {
            timestamp: "08:30:03.200".into(),
            facility: "kernel".into(),
            level: "err".into(),
            message: "nvme0: failed to set power state D3".into(),
            source: Some("nvme.c".into()),
        },
        KernelLogEntry {
            timestamp: "08:30:04.500".into(),
            facility: "kernel".into(),
            level: "info".into(),
            message: "IPv6: ADDRCONF(NETDEV_CHANGE): eth0: link becomes ready".into(),
            source: Some("ipv6.c".into()),
        },
        KernelLogEntry {
            timestamp: "08:30:05.000".into(),
            facility: "kernel".into(),
            level: "warn".into(),
            message: "TCP: too many orphaned sockets".into(),
            source: Some("tcp.c".into()),
        },
    ]
}

// ========== 性能分析（模拟） ==========

pub fn perf_profile(duration_secs: u32) -> PerfProfileResult {
    PerfProfileResult {
        event: "cpu-clock".into(),
        duration_secs,
        samples: 10000,
        top_functions: vec![
            PerfFunctionStat {
                function_name: "do_syscall_64".into(),
                overhead_percent: 12.5,
                samples: 1250,
            },
            PerfFunctionStat {
                function_name: "__schedule".into(),
                overhead_percent: 8.3,
                samples: 830,
            },
            PerfFunctionStat {
                function_name: "ext4_file_read_iter".into(),
                overhead_percent: 6.7,
                samples: 670,
            },
            PerfFunctionStat {
                function_name: "tcp_sendmsg".into(),
                overhead_percent: 5.2,
                samples: 520,
            },
            PerfFunctionStat {
                function_name: "copy_user_enhanced_fast_string".into(),
                overhead_percent: 4.1,
                samples: 410,
            },
            PerfFunctionStat {
                function_name: "kmem_cache_alloc".into(),
                overhead_percent: 3.8,
                samples: 380,
            },
            PerfFunctionStat {
                function_name: "_raw_spin_lock".into(),
                overhead_percent: 3.2,
                samples: 320,
            },
            PerfFunctionStat {
                function_name: "page_fault".into(),
                overhead_percent: 2.9,
                samples: 290,
            },
        ],
        top_symbols: vec![
            PerfSymbolStat {
                symbol: "[kernel.kallsyms]".into(),
                overhead_percent: 45.2,
                samples: 4520,
            },
            PerfSymbolStat {
                symbol: "[ext4]".into(),
                overhead_percent: 12.1,
                samples: 1210,
            },
            PerfSymbolStat {
                symbol: "[tcp]".into(),
                overhead_percent: 8.7,
                samples: 870,
            },
        ],
        raw_output: String::new(),
    }
}

// ========== 基准评测（模拟） ==========

pub fn run_benchmark() -> LinuxBenchmarkResult {
    LinuxBenchmarkResult {
        categories: vec![
            LinuxBenchmarkCategory {
                name: "CPU 性能".into(),
                score: 85.3,
                unit: "分".into(),
                details: vec![
                    "单核整数运算: 4200 MIPS".into(),
                    "多核浮点运算: 12800 MFLOPS".into(),
                    "上下文切换: 85000 ops/s".into(),
                ],
            },
            LinuxBenchmarkCategory {
                name: "内存性能".into(),
                score: 78.9,
                unit: "分".into(),
                details: vec![
                    "顺序读取: 12.5 GB/s".into(),
                    "随机读取: 8.2 GB/s".into(),
                    "延迟: 68 ns".into(),
                ],
            },
            LinuxBenchmarkCategory {
                name: "磁盘 I/O".into(),
                score: 72.1,
                unit: "分".into(),
                details: vec![
                    "顺序读取: 3500 MB/s".into(),
                    "顺序写入: 2800 MB/s".into(),
                    "随机 4K 读取: 450 MB/s".into(),
                ],
            },
            LinuxBenchmarkCategory {
                name: "网络性能".into(),
                score: 91.2,
                unit: "分".into(),
                details: vec![
                    "TCP 吞吐量: 9.4 Gbps".into(),
                    "UDP 吞吐量: 9.8 Gbps".into(),
                    "连接速率: 15000 conn/s".into(),
                ],
            },
        ],
        total_score: 81.9,
        duration_secs: 125.0,
        system_info: "Linux 6.6.80 x86_64".into(),
    }
}

// ========== 压力测试（模拟） ==========

pub fn run_stress() -> LinuxStressResult {
    let scenarios = vec![
        LinuxStressScenario {
            name: "CPU 压力".into(),
            passed: true,
            duration_secs: 30.0,
            max_memory_mb: 256.0,
            max_cpu_percent: 98.5,
            errors: vec![],
        },
        LinuxStressScenario {
            name: "内存压力".into(),
            passed: true,
            duration_secs: 30.0,
            max_memory_mb: 2048.0,
            max_cpu_percent: 45.2,
            errors: vec![],
        },
        LinuxStressScenario {
            name: "磁盘 I/O 压力".into(),
            passed: true,
            duration_secs: 30.0,
            max_memory_mb: 512.0,
            max_cpu_percent: 35.8,
            errors: vec![],
        },
        LinuxStressScenario {
            name: "网络压力".into(),
            passed: true,
            duration_secs: 30.0,
            max_memory_mb: 128.0,
            max_cpu_percent: 62.1,
            errors: vec![],
        },
    ];

    LinuxStressResult {
        all_passed: scenarios.iter().all(|s| s.passed),
        total_duration_secs: scenarios.iter().map(|s| s.duration_secs).sum(),
        summary: "所有压力测试通过".into(),
        scenarios,
    }
}

// ========== 辅助函数 ==========

fn get_file_type(name: &str) -> &str {
    if let Some(ext) = Path::new(name).extension().and_then(|e| e.to_str()) {
        match ext.to_lowercase().as_str() {
            "c" => "c_source",
            "h" => "c_header",
            "rs" => "rust_source",
            "s" | "S" => "assembly",
            "py" => "python",
            "sh" => "shell",
            "txt" => "text",
            "md" => "markdown",
            "json" => "json",
            "xml" => "xml",
            "toml" => "toml",
            "yml" | "yaml" => "yaml",
            _ => ext,
        }
    } else if name == "Makefile" || name == "Kconfig" || name == "Kbuild" {
        "build"
    } else {
        "unknown"
    }
}

fn get_language(name: &str) -> String {
    if let Some(ext) = Path::new(name).extension().and_then(|e| e.to_str()) {
        match ext.to_lowercase().as_str() {
            "c" => "C".into(),
            "h" => "C Header".into(),
            "rs" => "Rust".into(),
            "s" | "S" => "Assembly".into(),
            "py" => "Python".into(),
            "sh" => "Shell".into(),
            _ => ext.to_uppercase(),
        }
    } else if name == "Makefile" {
        "Makefile".into()
    } else if name == "Kconfig" {
        "Kconfig".into()
    } else {
        "Text".into()
    }
}

fn format_system_time(time: Option<std::time::SystemTime>) -> String {
    time.map(|t| {
        chrono::DateTime::<chrono::Utc>::from(t)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    })
    .unwrap_or_default()
}