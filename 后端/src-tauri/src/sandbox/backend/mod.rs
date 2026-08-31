//! 沙箱后端抽象层 — 对标 Codex SandboxBackend
//!
//! 提供统一的沙箱后端接口，支持：
//! - Process: 进程隔离（跨平台基础实现）
//! - Landlock: Linux Landlock LSM (5.13+)
//! - Seatbelt: macOS Sandbox
//! - WindowsSandbox: Windows Restricted Token + AppContainer

use async_trait::async_trait;

use crate::error::app_error::AppError;
use crate::models::sandbox::{ExecuteRequest, ExecuteResult};

/// 沙箱后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxBackendType {
    /// 基础进程隔离（跨平台）
    Process,
    /// Linux Landlock LSM
    Landlock,
    /// macOS Seatbelt Sandbox
    Seatbelt,
    /// Windows Sandbox
    WindowsSandbox,
}

impl SandboxBackendType {
    /// 自动检测当前平台最佳沙箱类型
    pub fn detect_best() -> Self {
        #[cfg(target_os = "linux")]
        {
            // Linux 5.13+ 支持 Landlock
            if Self::is_landlock_available() {
                return SandboxBackendType::Landlock;
            }
            SandboxBackendType::Process
        }
        #[cfg(target_os = "macos")]
        {
            SandboxBackendType::Seatbelt
        }
        #[cfg(target_os = "windows")]
        {
            SandboxBackendType::WindowsSandbox
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            SandboxBackendType::Process
        }
    }

    /// 检测 Landlock 是否可用
    #[allow(dead_code)]
    fn is_landlock_available() -> bool {
        #[cfg(target_os = "linux")]
        {
            // 检查 /proc/self/status 中是否包含 Landlock
            std::fs::read_to_string("/proc/self/status")
                .map(|s| s.contains("landlock"))
                .unwrap_or(false)
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            SandboxBackendType::Process => "process",
            SandboxBackendType::Landlock => "landlock",
            SandboxBackendType::Seatbelt => "seatbelt",
            SandboxBackendType::WindowsSandbox => "windows_sandbox",
        }
    }
}

/// 沙箱后端统一 trait
#[async_trait]
pub trait SandboxBackend: Send + Sync {
    /// 获取后端类型
    fn backend_type(&self) -> SandboxBackendType;

    /// 初始化沙箱环境
    async fn initialize(&mut self) -> Result<(), AppError>;

    /// 在沙箱中执行命令
    async fn execute(&self, req: &ExecuteRequest) -> Result<ExecuteResult, AppError>;

    /// 检查沙箱是否活跃
    fn is_active(&self) -> bool;

    /// 清理沙箱资源
    async fn cleanup(&mut self) -> Result<(), AppError>;
}

/// 沙箱后端工厂
pub struct SandboxBackendFactory;

impl SandboxBackendFactory {
    /// 根据类型创建沙箱后端
    pub fn create(backend_type: SandboxBackendType) -> Box<dyn SandboxBackend> {
        match backend_type {
            SandboxBackendType::Process => Box::new(process::ProcessBackend::new()),
            SandboxBackendType::Landlock => Box::new(landlock::LandlockBackend::new()),
            SandboxBackendType::Seatbelt => Box::new(seatbelt::SeatbeltBackend::new()),
            SandboxBackendType::WindowsSandbox => {
                Box::new(windows_sandbox::WindowsSandboxBackend::new())
            }
        }
    }

    /// 自动选择最佳沙箱后端
    pub fn create_best() -> Box<dyn SandboxBackend> {
        Self::create(SandboxBackendType::detect_best())
    }
}

pub mod process;
pub mod landlock;
pub mod seatbelt;
pub mod windows_sandbox;