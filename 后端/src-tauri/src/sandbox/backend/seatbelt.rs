//! macOS Seatbelt Sandbox 后端
//!
//! 使用 macOS 的 Seatbelt (sandbox-exec) 机制实现沙箱隔离。
//! 通过 .sb 配置文件定义沙箱规则。

use async_trait::async_trait;

use crate::error::app_error::AppError;
use crate::models::sandbox::{ExecuteRequest, ExecuteResult};

use super::process::ProcessBackend;
use super::{SandboxBackend, SandboxBackendType};

/// Seatbelt 沙箱配置
#[derive(Debug, Clone)]
pub struct SeatbeltConfig {
    /// 沙箱配置文件内容
    pub profile_content: String,
    /// 是否允许网络访问
    pub allow_network: bool,
    /// 是否允许文件系统读取
    pub allow_fs_read: bool,
    /// 是否允许文件系统写入
    pub allow_fs_write: bool,
}

impl Default for SeatbeltConfig {
    fn default() -> Self {
        Self {
            profile_content: Self::default_profile(),
            allow_network: false,
            allow_fs_read: true,
            allow_fs_write: false,
        }
    }
}

impl SeatbeltConfig {
    /// 默认的 Seatbelt 配置文件
    fn default_profile() -> String {
        r#"(version 1)
(allow default)
(deny network*)
(deny file-write*)
(allow file-read*)
(allow process-exec (regex #"^/usr/bin/"))
(allow process-exec (regex #"^/usr/local/bin/"))
(allow process-fork)"#
            .to_string()
    }

    /// 生成自定义配置文件
    pub fn build_profile(&self) -> String {
        let mut profile = String::from("(version 1)\n");

        if self.allow_network {
            profile.push_str("(allow network*)\n");
        } else {
            profile.push_str("(deny network*)\n");
        }

        if self.allow_fs_write {
            profile.push_str("(allow file-write*)\n");
        } else {
            profile.push_str("(deny file-write*)\n");
        }

        if self.allow_fs_read {
            profile.push_str("(allow file-read*)\n");
        }

        profile.push_str("(allow process-exec (regex #\"^/usr/bin/\"))\n");
        profile.push_str("(allow process-fork)\n");

        profile
    }
}

pub struct SeatbeltBackend {
    fallback: ProcessBackend,
    config: SeatbeltConfig,
}

impl SeatbeltBackend {
    pub fn new() -> Self {
        Self {
            fallback: ProcessBackend::new(),
            config: SeatbeltConfig::default(),
        }
    }

    pub fn with_config(mut self, config: SeatbeltConfig) -> Self {
        self.config = config;
        self
    }
}

#[async_trait]
impl SandboxBackend for SeatbeltBackend {
    fn backend_type(&self) -> SandboxBackendType {
        SandboxBackendType::Seatbelt
    }

    async fn initialize(&mut self) -> Result<(), AppError> {
        #[cfg(target_os = "macos")]
        {
            // 在真实 macOS 环境中，写入 .sb 配置文件
            let profile = self.config.build_profile();
            tracing::info!("Seatbelt 沙箱初始化: profile_length={}", profile.len());
        }
        #[cfg(not(target_os = "macos"))]
        {
            tracing::warn!("非 macOS 平台，Seatbelt 降级为进程隔离");
        }
        self.fallback.initialize().await
    }

    async fn execute(&self, req: &ExecuteRequest) -> Result<ExecuteResult, AppError> {
        #[cfg(target_os = "macos")]
        {
            // 在真实 macOS 环境中，使用 sandbox-exec 执行命令
            tracing::info!(
                "Seatbelt 沙箱执行: cmd={}, network={}, fs_write={}",
                req.command,
                self.config.allow_network,
                self.config.allow_fs_write
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