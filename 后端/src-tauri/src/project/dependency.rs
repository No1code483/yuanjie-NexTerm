//! 依赖管理

use serde::{Deserialize, Serialize};

/// 依赖信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// 包名
    pub name: String,
    /// 版本
    pub version: String,
    /// 是否为开发依赖
    pub dev: bool,
    /// 来源
    pub source: DependencySource,
    /// 描述
    pub description: Option<String>,
}

/// 依赖来源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencySource {
    /// npm/pip/cargo 等注册表
    Registry,
    /// Git 仓库
    Git { url: String, branch: Option<String> },
    /// 本地路径
    Local { path: String },
    /// 自定义 URL
    Url { url: String },
}

/// 依赖管理器
#[derive(Debug, Default)]
pub struct DependencyManager {
    /// 依赖列表
    dependencies: Vec<Dependency>,
}

impl DependencyManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加依赖
    pub fn add(&mut self, dep: Dependency) {
        if !self.dependencies.iter().any(|d| d.name == dep.name) {
            self.dependencies.push(dep);
        }
    }

    /// 移除依赖
    pub fn remove(&mut self, name: &str) -> bool {
        let len = self.dependencies.len();
        self.dependencies.retain(|d| d.name != name);
        self.dependencies.len() < len
    }

    /// 列出所有依赖
    pub fn list(&self) -> &[Dependency] {
        &self.dependencies
    }

    /// 列出生产依赖
    pub fn production_deps(&self) -> Vec<&Dependency> {
        self.dependencies.iter().filter(|d| !d.dev).collect()
    }

    /// 列出开发依赖
    pub fn dev_deps(&self) -> Vec<&Dependency> {
        self.dependencies.iter().filter(|d| d.dev).collect()
    }
}