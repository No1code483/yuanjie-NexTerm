use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SandboxType {
    FileSystem,
    Process,
    Network,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkPolicy {
    AllowAll,
    AllowList(Vec<String>),
    DenyAll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionProfile {
    pub name: String,
    pub read_paths: Vec<String>,
    pub write_paths: Vec<String>,
    pub allowed_commands: Vec<String>,
    pub network_policy: NetworkPolicy,
    pub max_file_size: u64,
    pub max_output_size: u64,
}

impl PermissionProfile {
    pub fn read_only() -> Self {
        Self {
            name: "read_only".into(),
            read_paths: vec![".".into()],
            write_paths: vec![],
            allowed_commands: vec![],
            network_policy: NetworkPolicy::DenyAll,
            max_file_size: 10 * 1024 * 1024,
            max_output_size: 1024 * 1024,
        }
    }

    pub fn read_write() -> Self {
        Self {
            name: "read_write".into(),
            read_paths: vec![".".into()],
            write_paths: vec![".".into(), "/tmp".into()],
            allowed_commands: vec![],
            network_policy: NetworkPolicy::DenyAll,
            max_file_size: 50 * 1024 * 1024,
            max_output_size: 5 * 1024 * 1024,
        }
    }

    pub fn full_access() -> Self {
        Self {
            name: "full_access".into(),
            read_paths: vec!["/".into()],
            write_paths: vec!["/".into(), "/tmp".into()],
            allowed_commands: vec!["*".into()],
            network_policy: NetworkPolicy::AllowAll,
            max_file_size: 100 * 1024 * 1024,
            max_output_size: 10 * 1024 * 1024,
        }
    }

    pub fn custom(read_paths: Vec<String>, write_paths: Vec<String>, commands: Vec<String>) -> Self {
        Self {
            name: "custom".into(),
            read_paths,
            write_paths,
            allowed_commands: commands,
            network_policy: NetworkPolicy::DenyAll,
            max_file_size: 10 * 1024 * 1024,
            max_output_size: 1024 * 1024,
        }
    }

    pub fn can_read(&self, path: &str) -> bool {
        self.read_paths.iter().any(|allowed| path.starts_with(allowed))
    }

    pub fn can_write(&self, path: &str) -> bool {
        self.write_paths.iter().any(|allowed| path.starts_with(allowed))
    }

    pub fn can_execute(&self, command: &str) -> bool {
        self.allowed_commands.iter().any(|allowed| {
            allowed == "*" || allowed == command
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub sandbox_type: SandboxType,
    pub permission_profile: PermissionProfile,
    pub timeout_ms: u64,
    pub agent_id: Option<String>,
    pub working_dir: String,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            sandbox_type: SandboxType::FileSystem,
            permission_profile: PermissionProfile::read_write(),
            timeout_ms: 30000,
            agent_id: None,
            working_dir: ".".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSaveRequest {
    pub safety_level: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub allowed_languages: Vec<String>,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SandboxState {
    Idle,
    Running,
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInfo {
    pub sandbox_id: String,
    pub sandbox_type: String,
    pub permission_name: String,
    pub state: String,
    pub agent_id: Option<String>,
    pub created_at: i64,
    pub executions_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRequest {
    pub sandbox_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub stdin: Option<String>,
    pub working_dir: Option<String>,
    pub env: Option<Vec<(String, String)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperationRequest {
    pub sandbox_id: String,
    pub path: String,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperationResult {
    pub success: bool,
    pub content: Option<String>,
    pub error: Option<String>,
    pub file_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSnapshot {
    pub sandbox_id: String,
    pub sandbox_type: String,
    pub state: String,
    pub permission_name: String,
    pub agent_id: Option<String>,
    pub working_dir: String,
    pub timeout_ms: u64,
    pub created_at: i64,
    pub executions_count: u64,
    pub total_duration_ms: u64,
}

/// 沙箱执行历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxHistoryRecord {
    pub id: String,
    pub sandbox_id: String,
    pub lang: String,
    pub risk: f64,
    pub passed: bool,
    pub duration: String,
    pub timestamp: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}