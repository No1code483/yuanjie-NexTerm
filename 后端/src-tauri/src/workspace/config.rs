//! 工作区配置 — .yuan-workspace.json

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 工作区配置文件 (.yuan-workspace.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// 工作区名称
    pub name: String,
    /// 根目录
    pub folders: Vec<WorkspaceFolder>,
    /// 设置覆盖
    pub settings: Option<WorkspaceSettingsOverride>,
    /// 扩展推荐
    pub extensions: Option<Vec<String>>,
    /// 启动任务
    pub launch: Option<LaunchConfig>,
}

/// 工作区文件夹
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFolder {
    /// 路径 (相对于工作区配置文件)
    pub path: String,
    /// 显示名称
    pub name: Option<String>,
}

/// 工作区设置覆盖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSettingsOverride {
    /// 默认语言
    pub default_language: Option<String>,
    /// 排除文件模式
    pub exclude_patterns: Option<Vec<String>>,
    /// 文件编码
    pub encoding: Option<String>,
    /// 缩进大小
    pub tab_size: Option<u8>,
    /// 是否使用空格
    pub insert_spaces: Option<bool>,
    /// 自动保存
    pub auto_save: Option<bool>,
    /// 格式化器
    pub formatter: Option<String>,
}

/// 启动配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchConfig {
    /// 启动命令
    pub command: String,
    /// 工作目录
    pub cwd: Option<String>,
    /// 环境变量
    pub env: Option<Vec<(String, String)>>,
    /// 构建任务
    pub pre_launch_task: Option<String>,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            name: "未命名工作区".into(),
            folders: vec![WorkspaceFolder {
                path: ".".into(),
                name: None,
            }],
            settings: None,
            extensions: None,
            launch: None,
        }
    }
}

impl WorkspaceConfig {
    /// 从文件加载
    pub fn load(path: &PathBuf) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("读取工作区配置文件失败: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("解析工作区配置失败: {}", e))
    }

    /// 保存到文件
    pub fn save(&self, path: &PathBuf) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化工作区配置失败: {}", e))?;
        std::fs::write(path, content)
            .map_err(|e| format!("写入工作区配置失败: {}", e))
    }

    /// 获取所有文件夹路径
    pub fn folder_paths(&self, base_dir: &PathBuf) -> Vec<PathBuf> {
        self.folders
            .iter()
            .map(|f| base_dir.join(&f.path))
            .collect()
    }
}