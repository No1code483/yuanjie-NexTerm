//! 安全策略配置
//!
//! 定义和管理安全策略，包括：
//! - 文件系统访问策略
//! - 命令执行策略
//! - 网络访问策略
//! - 审批策略

use serde::{Deserialize, Serialize};

use super::approval::{ApprovalPolicy, ApprovalType};

/// 安全策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// 策略名称
    pub name: String,
    /// 策略描述
    pub description: String,
    /// 是否启用
    pub enabled: bool,
    /// 文件系统策略
    pub filesystem: FileSystemPolicy,
    /// 命令执行策略
    pub command: CommandPolicy,
    /// 网络策略
    pub network: NetworkPolicy,
    /// 审批策略映射
    pub approval_policies: ApprovalPolicyMap,
    /// 风险阈值
    pub risk_thresholds: RiskThresholds,
}

/// 文件系统策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemPolicy {
    /// 允许读取的路径
    pub read_allow: Vec<String>,
    /// 允许写入的路径
    pub write_allow: Vec<String>,
    /// 禁止访问的路径
    pub deny: Vec<String>,
    /// 最大文件大小 (bytes)
    pub max_file_size: u64,
    /// 是否允许访问隐藏文件
    pub allow_hidden: bool,
    /// 是否允许访问系统目录
    pub allow_system_dirs: bool,
}

impl Default for FileSystemPolicy {
    fn default() -> Self {
        Self {
            read_allow: vec![".".into()],
            write_allow: vec![".".into()],
            deny: vec![
                "/etc/passwd".into(),
                "/etc/shadow".into(),
                "~/.ssh".into(),
                "~/.gnupg".into(),
            ],
            max_file_size: 100 * 1024 * 1024, // 100 MB
            allow_hidden: false,
            allow_system_dirs: false,
        }
    }
}

/// 命令执行策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPolicy {
    /// 允许的命令
    pub allow: Vec<String>,
    /// 禁止的命令
    pub deny: Vec<String>,
    /// 最大执行时间 (ms)
    pub max_execution_ms: u64,
    /// 是否允许管道
    pub allow_pipes: bool,
    /// 是否允许重定向
    pub allow_redirects: bool,
}

impl Default for CommandPolicy {
    fn default() -> Self {
        Self {
            allow: vec![
                "python".into(),
                "python3".into(),
                "node".into(),
                "rustc".into(),
                "cargo".into(),
                "go".into(),
                "gcc".into(),
                "make".into(),
                "npm".into(),
                "pip".into(),
                "git".into(),
            ],
            deny: vec![
                "rm -rf".into(),
                "mkfs".into(),
                "dd".into(),
                "shutdown".into(),
                "reboot".into(),
                "chmod 777".into(),
                "sudo".into(),
                "su".into(),
            ],
            max_execution_ms: 30_000,
            allow_pipes: true,
            allow_redirects: false,
        }
    }
}

/// 网络策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    /// 是否允许出站连接
    pub allow_outbound: bool,
    /// 允许的域名
    pub allowed_domains: Vec<String>,
    /// 禁止的域名
    pub denied_domains: Vec<String>,
    /// 允许的端口
    pub allowed_ports: Vec<u16>,
    /// 最大连接数
    pub max_connections: u32,
    /// 是否允许原始 IP 连接
    pub allow_raw_ip: bool,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            allow_outbound: true,
            allowed_domains: vec![
                "api.openai.com".into(),
                "api.anthropic.com".into(),
                "github.com".into(),
                "pypi.org".into(),
                "crates.io".into(),
                "npmjs.com".into(),
            ],
            denied_domains: vec!["localhost".into(), "127.0.0.1".into(), "0.0.0.0".into()],
            allowed_ports: vec![80, 443, 8080, 3000],
            max_connections: 10,
            allow_raw_ip: false,
        }
    }
}

/// 审批策略映射
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalPolicyMap {
    pub file_write: ApprovalPolicy,
    pub file_delete: ApprovalPolicy,
    pub command_execute: ApprovalPolicy,
    pub network_request: ApprovalPolicy,
    pub system_config: ApprovalPolicy,
    pub env_modify: ApprovalPolicy,
    pub package_install: ApprovalPolicy,
    pub secret_access: ApprovalPolicy,
    pub database_operation: ApprovalPolicy,
}

impl Default for ApprovalPolicyMap {
    fn default() -> Self {
        Self {
            file_write: ApprovalPolicy::AskUser,
            file_delete: ApprovalPolicy::AskUser,
            command_execute: ApprovalPolicy::AskUser,
            network_request: ApprovalPolicy::AutoApprove,
            system_config: ApprovalPolicy::AskUser,
            env_modify: ApprovalPolicy::AskUser,
            package_install: ApprovalPolicy::AskUser,
            secret_access: ApprovalPolicy::AutoDeny,
            database_operation: ApprovalPolicy::AskUser,
        }
    }
}

impl ApprovalPolicyMap {
    /// 获取指定类型的审批策略
    pub fn get(&self, approval_type: &ApprovalType) -> ApprovalPolicy {
        match approval_type {
            ApprovalType::FileWrite => self.file_write,
            ApprovalType::FileDelete => self.file_delete,
            ApprovalType::CommandExecute => self.command_execute,
            ApprovalType::NetworkRequest => self.network_request,
            ApprovalType::SystemConfig => self.system_config,
            ApprovalType::EnvModify => self.env_modify,
            ApprovalType::PackageInstall => self.package_install,
            ApprovalType::SecretAccess => self.secret_access,
            ApprovalType::DatabaseOperation => self.database_operation,
            ApprovalType::Custom(_) => ApprovalPolicy::AskUser,
        }
    }
}

/// 风险阈值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskThresholds {
    /// 自动批准阈值 (0.0 - 1.0)
    pub auto_approve: f64,
    /// 自动拒绝阈值 (0.0 - 1.0)
    pub auto_deny: f64,
    /// 需要审查阈值 (0.0 - 1.0)
    pub review: f64,
}

impl Default for RiskThresholds {
    fn default() -> Self {
        Self {
            auto_approve: 0.3,
            auto_deny: 0.7,
            review: 0.5,
        }
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            name: "default".into(),
            description: "默认安全策略".into(),
            enabled: true,
            filesystem: FileSystemPolicy::default(),
            command: CommandPolicy::default(),
            network: NetworkPolicy::default(),
            approval_policies: ApprovalPolicyMap::default(),
            risk_thresholds: RiskThresholds::default(),
        }
    }
}

impl SecurityPolicy {
    /// 创建严格策略
    pub fn strict() -> Self {
        Self {
            name: "strict".into(),
            description: "严格安全策略 - 最高安全级别".into(),
            enabled: true,
            filesystem: FileSystemPolicy {
                write_allow: vec![],
                allow_hidden: false,
                allow_system_dirs: false,
                ..Default::default()
            },
            command: CommandPolicy {
                allow: vec!["python3".into(), "node".into()],
                max_execution_ms: 10_000,
                allow_pipes: false,
                ..Default::default()
            },
            network: NetworkPolicy {
                allow_outbound: true,
                max_connections: 3,
                allow_raw_ip: false,
                ..Default::default()
            },
            approval_policies: ApprovalPolicyMap {
                file_write: ApprovalPolicy::AskUser,
                file_delete: ApprovalPolicy::AutoDeny,
                command_execute: ApprovalPolicy::AskUser,
                network_request: ApprovalPolicy::AskUser,
                system_config: ApprovalPolicy::AutoDeny,
                env_modify: ApprovalPolicy::AutoDeny,
                package_install: ApprovalPolicy::AutoDeny,
                secret_access: ApprovalPolicy::AutoDeny,
                database_operation: ApprovalPolicy::AutoDeny,
            },
            risk_thresholds: RiskThresholds {
                auto_approve: 0.1,
                auto_deny: 0.5,
                review: 0.3,
            },
        }
    }

    /// 创建宽松策略（开发模式）
    pub fn relaxed() -> Self {
        Self {
            name: "relaxed".into(),
            description: "宽松安全策略 - 开发模式".into(),
            enabled: true,
            filesystem: FileSystemPolicy {
                allow_hidden: true,
                allow_system_dirs: true,
                ..Default::default()
            },
            command: CommandPolicy {
                max_execution_ms: 60_000,
                allow_pipes: true,
                allow_redirects: true,
                ..Default::default()
            },
            network: NetworkPolicy {
                max_connections: 50,
                allow_raw_ip: true,
                ..Default::default()
            },
            approval_policies: ApprovalPolicyMap {
                file_write: ApprovalPolicy::AutoApprove,
                file_delete: ApprovalPolicy::AutoApprove,
                command_execute: ApprovalPolicy::AutoApprove,
                network_request: ApprovalPolicy::AutoApprove,
                system_config: ApprovalPolicy::AutoApprove,
                env_modify: ApprovalPolicy::AutoApprove,
                package_install: ApprovalPolicy::AutoApprove,
                secret_access: ApprovalPolicy::AskUser,
                database_operation: ApprovalPolicy::AutoApprove,
            },
            risk_thresholds: RiskThresholds {
                auto_approve: 0.7,
                auto_deny: 0.95,
                review: 0.8,
            },
        }
    }
}