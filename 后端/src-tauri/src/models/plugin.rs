use serde::{Deserialize, Serialize};

/// 插件清单 - 对标 Codex 的 plugin.json manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// 插件唯一标识
    pub name: String,
    /// 显示名称
    pub display_name: String,
    /// 版本号 (semver)
    pub version: String,
    /// 描述
    pub description: String,
    /// 作者
    pub author: Option<String>,
    /// 许可证
    pub license: Option<String>,
    /// 主页
    pub homepage: Option<String>,
    /// 仓库
    pub repository: Option<String>,
    /// 最低 YuanCode 版本要求
    pub min_yuan_version: Option<String>,
    /// 插件分类
    pub category: PluginCategory,
    /// 标签
    pub tags: Vec<String>,
    /// 图标
    pub icon: Option<String>,
    /// 入口配置
    #[serde(default)]
    pub entry: PluginEntry,
    /// 权限声明
    #[serde(default)]
    pub permissions: Vec<PluginPermission>,
    /// 依赖的其他插件
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
    /// 提供的技能
    #[serde(default)]
    pub skills: Vec<String>,
    /// 提供的 MCP 服务器
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
    /// 提供的命令
    #[serde(default)]
    pub commands: Vec<PluginCommand>,
    /// 钩子
    #[serde(default)]
    pub hooks: Vec<PluginHook>,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// 插件分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginCategory {
    #[serde(rename = "git")]
    Git,
    #[serde(rename = "testing")]
    Testing,
    #[serde(rename = "formatting")]
    Formatting,
    #[serde(rename = "docs")]
    Documentation,
    #[serde(rename = "security")]
    Security,
    #[serde(rename = "database")]
    Database,
    #[serde(rename = "deployment")]
    Deployment,
    #[serde(rename = "utility")]
    Utility,
    #[serde(rename = "other")]
    Other,
}

/// 插件入口配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginEntry {
    /// 技能目录
    #[serde(default)]
    pub skills_dir: Option<String>,
    /// MCP 服务器配置
    #[serde(default)]
    pub mcp: Option<McpServerConfig>,
    /// 主入口脚本
    pub main: Option<String>,
    /// 配置入口
    pub config: Option<String>,
}

/// 插件权限
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPermission {
    pub name: String,
    pub description: String,
    /// 是否需要审批
    #[serde(default)]
    pub requires_approval: bool,
}

/// 插件依赖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub name: String,
    /// 版本要求 (如 ">=1.0.0")
    pub version: String,
    /// 是否可选
    #[serde(default)]
    pub optional: bool,
}

/// MCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    /// 连接超时 (ms)
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    /// 自动重连
    #[serde(default)]
    pub auto_reconnect: bool,
}

/// 插件命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommand {
    pub name: String,
    pub description: String,
    /// 对应的 Tauri command 名称
    pub handler: String,
    /// 参数 schema
    #[serde(default)]
    pub args: serde_json::Value,
}

/// 插件钩子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHook {
    /// 钩子类型
    pub event: PluginHookEvent,
    /// 处理函数
    pub handler: String,
}

/// 插件钩子事件
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PluginHookEvent {
    #[serde(rename = "on_load")]
    OnLoad,
    #[serde(rename = "on_unload")]
    OnUnload,
    #[serde(rename = "on_enable")]
    OnEnable,
    #[serde(rename = "on_disable")]
    OnDisable,
    #[serde(rename = "on_project_open")]
    OnProjectOpen,
    #[serde(rename = "on_project_close")]
    OnProjectClose,
    #[serde(rename = "on_file_save")]
    OnFileSave,
    #[serde(rename = "on_file_change")]
    OnFileChange,
    #[serde(rename = "on_git_commit")]
    OnGitCommit,
    #[serde(rename = "on_build_complete")]
    OnBuildComplete,
}

/// 插件状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginState {
    /// 未加载
    NotLoaded,
    /// 加载中
    Loading,
    /// 已加载
    Loaded,
    /// 已启用
    Enabled,
    /// 已禁用
    Disabled,
    /// 错误
    Error(String),
    /// 已卸载
    Unloaded,
}

/// 插件运行时信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRuntime {
    pub manifest: PluginManifest,
    pub state: PluginState,
    pub install_path: String,
    pub loaded_at: Option<i64>,
    pub error: Option<String>,
    /// 加载耗时 (ms)
    pub load_duration_ms: Option<u64>,
}