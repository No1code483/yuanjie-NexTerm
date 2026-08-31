use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinuxKernelVersion {
    pub version: String,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub release_type: String,
    pub release_date: String,
    pub download_url: String,
    pub size_mb: f64,
    pub is_lts: bool,
    pub is_stable: bool,
    pub changelog: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinuxKernelSource {
    pub version: String,
    pub download_dir: String,
    pub extracted_dir: String,
    pub total_files: u64,
    pub total_size_bytes: u64,
    pub download_status: String,
    pub downloaded_at: String,
    pub arch: String,
    pub config_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinuxSourceFile {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub file_type: String,
    pub language: String,
    pub is_directory: bool,
    pub modified_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinuxSourceDirectory {
    pub path: String,
    pub name: String,
    pub entries: Vec<LinuxSourceFile>,
    pub total_entries: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinuxFileContent {
    pub path: String,
    pub name: String,
    pub content: String,
    pub line_count: u64,
    pub language: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinuxSearchResult {
    pub file_path: String,
    pub file_name: String,
    pub line_number: u64,
    pub line_content: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinuxSearchResponse {
    pub query: String,
    pub results: Vec<LinuxSearchResult>,
    pub total_matches: u64,
    pub files_searched: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinuxDownloadProgress {
    pub version: String,
    pub status: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub progress_percent: f64,
    pub speed_mbps: f64,
    pub estimated_remaining_secs: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinuxEnvironmentInfo {
    pub has_kernel: bool,
    pub active_version: Option<String>,
    pub installed_versions: Vec<String>,
    pub source_dir: Option<String>,
    pub total_downloaded_sources: u64,
    pub environment_ready: bool,
    pub working_directory: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinuxBuildConfig {
    pub arch: String,
    pub optimization_level: String,
    pub smp_support: bool,
    pub networking_support: bool,
    pub filesystem_types: Vec<String>,
    pub device_drivers: Vec<String>,
    pub debugging_enabled: bool,
    pub kernel_config_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinuxConfigOption {
    pub key: String,
    pub value: String,
    pub description: String,
    pub category: String,
    pub is_enabled: bool,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelBuildRequest {
    pub source_dir: String,
    pub arch: String,
    pub defconfig: Option<String>,
    pub jobs: Option<u32>,
    pub clean_first: Option<bool>,
    pub target: Option<String>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelBuildResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_secs: f64,
    pub timed_out: bool,
    pub artifact_path: Option<String>,
    pub errors: Vec<KernelBuildError>,
    pub warnings: Vec<KernelBuildWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelBuildError {
    pub line_number: usize,
    pub file_path: Option<String>,
    pub message: String,
    pub error_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelBuildWarning {
    pub line_number: usize,
    pub file_path: Option<String>,
    pub message: String,
    pub warning_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchInfo {
    pub file_name: String,
    pub file_path: String,
    pub author: String,
    pub date: String,
    pub subject: String,
    pub files_changed: Vec<String>,
    pub insertions: usize,
    pub deletions: usize,
    pub hunks_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchApplyRequest {
    pub source_dir: String,
    pub patch_path: String,
    pub dry_run: Option<bool>,
    pub reverse: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchApplyResult {
    pub patch_file: String,
    pub applied: bool,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub hunks: Vec<PatchHunkResult>,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchHunkResult {
    pub file_path: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfigItem {
    pub key: String,
    pub value: String,
    pub disabled: bool,
    pub is_module: bool,
    pub help_text: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfigRequest {
    pub source_dir: String,
    pub config_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfigResult {
    pub config_path: String,
    pub items: Vec<KernelConfigItem>,
    pub total_enabled: usize,
    pub total_disabled: usize,
    pub total_modules: usize,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelModuleInfo {
    pub name: String,
    pub size_mb: f64,
    pub used_by: Vec<String>,
    pub depends_on: Vec<String>,
    pub parameters: Vec<KernelModuleParam>,
    pub path: String,
    pub license: String,
    pub description: String,
    pub loaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelModuleParam {
    pub name: String,
    pub value: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelLogEntry {
    pub timestamp: String,
    pub facility: String,
    pub level: String,
    pub message: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelLogAnalysis {
    pub total_entries: usize,
    pub entries: Vec<KernelLogEntry>,
    pub by_level: std::collections::HashMap<String, usize>,
    pub by_facility: std::collections::HashMap<String, usize>,
    pub critical_events: Vec<KernelLogEntry>,
    pub boot_time: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfProfileRequest {
    pub command: Option<String>,
    pub pid: Option<u32>,
    pub duration_secs: u32,
    pub event: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfProfileResult {
    pub event: String,
    pub duration_secs: u32,
    pub samples: u64,
    pub top_functions: Vec<PerfFunctionStat>,
    pub top_symbols: Vec<PerfSymbolStat>,
    pub raw_output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfFunctionStat {
    pub function_name: String,
    pub overhead_percent: f64,
    pub samples: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfSymbolStat {
    pub symbol: String,
    pub overhead_percent: f64,
    pub samples: u64,
}

// ========== 系统状态模型（前端 Linux.tsx 状态面板 Tab） ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxShellStatus {
    pub is_installed: bool,
    pub is_running: bool,
    pub shell_type: String,
    pub version: String,
    pub process_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxSystemInfo {
    pub hostname: String,
    pub uptime_secs: u64,
    pub load_average: Vec<f64>,
    pub processes: u32,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub kernel_version: String,
    pub os_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxIsoProgress {
    pub is_running: bool,
    pub progress: f64,
    pub current_step: String,
    pub speed_mbps: f64,
    pub estimated_remaining_secs: u64,
    pub iso_name: String,
    pub total_size_mb: f64,
    pub downloaded_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxStatusPanel {
    pub system: LinuxSystemInfo,
    pub environment: LinuxEnvironmentInfo,
    pub shell: LinuxShellStatus,
    pub iso: LinuxIsoProgress,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxBenchmarkCategory {
    pub name: String,
    pub score: f64,
    pub unit: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxBenchmarkResult {
    pub categories: Vec<LinuxBenchmarkCategory>,
    pub total_score: f64,
    pub duration_secs: f64,
    pub system_info: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxStressScenario {
    pub name: String,
    pub passed: bool,
    pub duration_secs: f64,
    pub max_memory_mb: f64,
    pub max_cpu_percent: f64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxStressResult {
    pub scenarios: Vec<LinuxStressScenario>,
    pub total_duration_secs: f64,
    pub all_passed: bool,
    pub summary: String,
}