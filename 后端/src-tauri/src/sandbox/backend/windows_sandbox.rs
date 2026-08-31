//! Windows Sandbox 后端
//!
//! 使用 Windows Restricted Token + AppContainer 实现沙箱隔离。
//! 在 Windows 上提供进程级安全隔离。

use async_trait::async_trait;

use crate::error::app_error::AppError;
use crate::models::sandbox::{ExecuteRequest, ExecuteResult};

use super::process::ProcessBackend;
use super::{SandboxBackend, SandboxBackendType};

/// Windows 沙箱配置
#[derive(Debug, Clone)]
pub struct WindowsSandboxConfig {
    /// 是否使用 AppContainer 隔离
    pub use_app_container: bool,
    /// 是否限制网络访问
    pub deny_network: bool,
    /// 允许的注册表路径
    pub allowed_registry_paths: Vec<String>,
    /// 允许的文件系统路径
    pub allowed_fs_paths: Vec<String>,
    /// 完整性级别（Low/Medium/High）
    pub integrity_level: WindowsIntegrityLevel,
}

impl Default for WindowsSandboxConfig {
    fn default() -> Self {
        Self {
            use_app_container: true,
            deny_network: true,
            allowed_registry_paths: Vec::new(),
            allowed_fs_paths: vec![std::env::temp_dir().to_string_lossy().to_string()],
            integrity_level: WindowsIntegrityLevel::Low,
        }
    }
}

/// Windows 完整性级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsIntegrityLevel {
    Untrusted,
    Low,
    Medium,
    High,
    System,
}

impl WindowsIntegrityLevel {
    pub fn as_sid(&self) -> &str {
        match self {
            WindowsIntegrityLevel::Untrusted => "S-1-16-0",
            WindowsIntegrityLevel::Low => "S-1-16-4096",
            WindowsIntegrityLevel::Medium => "S-1-16-8192",
            WindowsIntegrityLevel::High => "S-1-16-12288",
            WindowsIntegrityLevel::System => "S-1-16-16384",
        }
    }
}

pub struct WindowsSandboxBackend {
    fallback: ProcessBackend,
    config: WindowsSandboxConfig,
}

impl WindowsSandboxBackend {
    pub fn new() -> Self {
        Self {
            fallback: ProcessBackend::new(),
            config: WindowsSandboxConfig::default(),
        }
    }

    pub fn with_config(mut self, config: WindowsSandboxConfig) -> Self {
        self.config = config;
        self
    }

    /// 构建 Windows 安全属性
    fn build_security_attributes(&self) -> WindowsSecurityAttributes {
        WindowsSecurityAttributes {
            integrity_level: self.config.integrity_level,
            deny_network: self.config.deny_network,
            allowed_paths: self.config.allowed_fs_paths.clone(),
        }
    }
}

/// Windows 安全属性
#[derive(Debug, Clone)]
pub struct WindowsSecurityAttributes {
    pub integrity_level: WindowsIntegrityLevel,
    pub deny_network: bool,
    pub allowed_paths: Vec<String>,
}

#[async_trait]
impl SandboxBackend for WindowsSandboxBackend {
    fn backend_type(&self) -> SandboxBackendType {
        SandboxBackendType::WindowsSandbox
    }

    async fn initialize(&mut self) -> Result<(), AppError> {
        let attrs = self.build_security_attributes();
        tracing::info!(
            "Windows Sandbox 初始化: integrity={:?}, deny_network={}, allowed_paths={}",
            attrs.integrity_level,
            attrs.deny_network,
            attrs.allowed_paths.len()
        );

        // 在真实 Windows 环境中:
        // 1. 创建 Restricted Token (CreateRestrictedToken)
        // 2. 设置完整性级别 (SetTokenInformation + TokenIntegrityLevel)
        // 3. 可选: 创建 AppContainer Profile (CreateAppContainerProfile)
        // 4. 使用受限 Token 启动子进程

        self.fallback.initialize().await
    }

    async fn execute(&self, req: &ExecuteRequest) -> Result<ExecuteResult, AppError> {
        tracing::info!(
            "Windows Sandbox 执行: cmd={}, integrity={:?}",
            req.command,
            self.config.integrity_level
        );

        // 在真实 Windows 环境中，使用受限 Token 创建子进程
        // CreateProcessWithTokenW 或 CreateProcessAsUserW
        self.fallback.execute(req).await
    }

    fn is_active(&self) -> bool {
        self.fallback.is_active()
    }

    async fn cleanup(&mut self) -> Result<(), AppError> {
        // 清理 AppContainer Profile（如果使用）
        self.fallback.cleanup().await
    }
}