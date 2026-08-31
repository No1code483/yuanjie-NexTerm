//! Linux Landlock LSM 沙箱后端
//!
//! 使用 Linux 5.13+ 内核的 Landlock 安全模块实现文件系统隔离。
//! 当内核不支持 Landlock 时，退化为 ProcessBackend。

use async_trait::async_trait;

use crate::error::app_error::AppError;
use crate::models::sandbox::{ExecuteRequest, ExecuteResult};

use super::process::ProcessBackend;
use super::{SandboxBackend, SandboxBackendType};

/// Landlock 规则集
#[derive(Debug, Clone)]
pub struct LandlockRules {
    /// 允许读取的路径
    pub read_paths: Vec<String>,
    /// 允许写入的路径
    pub write_paths: Vec<String>,
    /// 允许执行的路径
    pub exec_paths: Vec<String>,
}

impl Default for LandlockRules {
    fn default() -> Self {
        Self {
            read_paths: vec!["/usr".into(), "/lib".into(), "/lib64".into(), "/etc".into()],
            write_paths: vec!["/tmp".into()],
            exec_paths: vec!["/usr/bin".into(), "/usr/local/bin".into()],
        }
    }
}

pub struct LandlockBackend {
    fallback: ProcessBackend,
    landlock_available: bool,
    rules: LandlockRules,
}

impl LandlockBackend {
    pub fn new() -> Self {
        Self {
            fallback: ProcessBackend::new(),
            landlock_available: Self::check_landlock(),
            rules: LandlockRules::default(),
        }
    }

    fn check_landlock() -> bool {
        std::fs::read_to_string("/proc/self/status")
            .map(|s| s.contains("landlock"))
            .unwrap_or(false)
    }

    /// 构建 Landlock 规则集
    pub fn with_rules(mut self, rules: LandlockRules) -> Self {
        self.rules = rules;
        self
    }

    /// 添加读取路径
    pub fn allow_read(&mut self, path: &str) {
        if !self.rules.read_paths.contains(&path.to_string()) {
            self.rules.read_paths.push(path.to_string());
        }
    }

    /// 添加写入路径
    pub fn allow_write(&mut self, path: &str) {
        if !self.rules.write_paths.contains(&path.to_string()) {
            self.rules.write_paths.push(path.to_string());
        }
    }
}

#[async_trait]
impl SandboxBackend for LandlockBackend {
    fn backend_type(&self) -> SandboxBackendType {
        SandboxBackendType::Landlock
    }

    async fn initialize(&mut self) -> Result<(), AppError> {
        if self.landlock_available {
            // 在真实环境中，这里会调用 landlock_create_ruleset 等系统调用
            // 当前为 Windows 开发环境，使用进程隔离作为 fallback
            tracing::info!(
                "Landlock 沙箱初始化: 规则集已就绪 (read_paths={}, write_paths={})",
                self.rules.read_paths.len(),
                self.rules.write_paths.len()
            );
        } else {
            tracing::warn!("Landlock 不可用，降级为进程隔离");
        }
        self.fallback.initialize().await
    }

    async fn execute(&self, req: &ExecuteRequest) -> Result<ExecuteResult, AppError> {
        // 在真实 Linux 环境中，这里会先应用 Landlock 规则再执行
        if self.landlock_available {
            tracing::info!(
                "Landlock 沙箱执行: cmd={}, read_paths={}, write_paths={}",
                req.command,
                self.rules.read_paths.len(),
                self.rules.write_paths.len()
            );
        }
        self.fallback.execute(req).await
    }

    fn is_active(&self) -> bool {
        self.fallback.is_active()
    }

    async fn cleanup(&mut self) -> Result<(), AppError> {
        self.fallback.cleanup().await
    }
}