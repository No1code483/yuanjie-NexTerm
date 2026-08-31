//! 项目管理

pub mod build;
pub mod dependency;
pub mod template;

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 项目信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// 项目 ID
    pub id: String,
    /// 项目名称
    pub name: String,
    /// 项目路径
    pub path: PathBuf,
    /// 项目类型
    pub project_type: ProjectType,
    /// 语言
    pub language: String,
    /// 构建配置
    pub build_config: Option<build::BuildConfig>,
    /// 依赖列表
    pub dependencies: Vec<dependency::Dependency>,
    /// 创建时间
    pub created_at: i64,
    /// 修改时间
    pub modified_at: i64,
}

/// 项目类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectType {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Web,
    Unknown,
}

impl ProjectType {
    /// 从路径检测项目类型
    pub fn detect(path: &PathBuf) -> Self {
        if path.join("Cargo.toml").exists() {
            ProjectType::Rust
        } else if path.join("requirements.txt").exists() || path.join("pyproject.toml").exists() {
            ProjectType::Python
        } else if path.join("tsconfig.json").exists() {
            ProjectType::TypeScript
        } else if path.join("package.json").exists() {
            ProjectType::JavaScript
        } else if path.join("go.mod").exists() {
            ProjectType::Go
        } else if path.join("index.html").exists() {
            ProjectType::Web
        } else {
            ProjectType::Unknown
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            ProjectType::Rust => "Rust",
            ProjectType::Python => "Python",
            ProjectType::JavaScript => "JavaScript",
            ProjectType::TypeScript => "TypeScript",
            ProjectType::Go => "Go",
            ProjectType::Web => "Web",
            ProjectType::Unknown => "Unknown",
        }
    }
}

/// 项目管理器
#[derive(Debug)]
pub struct ProjectManager {
    /// 项目映射
    projects: HashMap<String, ProjectInfo>,
}

impl ProjectManager {
    pub fn new() -> Self {
        Self {
            projects: HashMap::new(),
        }
    }

    /// 注册项目
    pub fn register(&mut self, path: PathBuf) -> Result<&ProjectInfo, String> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled".to_string());

        let id = format!("proj_{}", chrono::Utc::now().timestamp());
        let project_type = ProjectType::detect(&path);

        let info = ProjectInfo {
            id: id.clone(),
            name,
            path,
            project_type: project_type.clone(),
            language: project_type.as_str().to_string(),
            build_config: None,
            dependencies: Vec::new(),
            created_at: chrono::Utc::now().timestamp(),
            modified_at: chrono::Utc::now().timestamp(),
        };

        self.projects.insert(id, info.clone());
        Ok(self.projects.values().last().unwrap())
    }

    /// 获取项目
    pub fn get(&self, id: &str) -> Option<&ProjectInfo> {
        self.projects.get(id)
    }

    /// 列出所有项目
    pub fn list(&self) -> Vec<&ProjectInfo> {
        self.projects.values().collect()
    }

    /// 移除项目
    pub fn remove(&mut self, id: &str) -> bool {
        self.projects.remove(id).is_some()
    }
}