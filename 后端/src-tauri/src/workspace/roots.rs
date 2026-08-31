//! 多根工作区管理

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 多根工作区
#[derive(Debug, Clone, Default)]
pub struct MultiRootWorkspace {
    /// 根目录映射
    roots: HashMap<String, WorkspaceRoot>,
    /// 根目录顺序
    order: Vec<String>,
    /// 主根目录 ID
    primary: Option<String>,
}

/// 工作区根目录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRoot {
    /// 唯一 ID
    pub id: String,
    /// 路径
    pub path: PathBuf,
    /// 显示名称
    pub name: String,
    /// 索引
    pub index: usize,
}

impl MultiRootWorkspace {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加根目录
    pub fn add_root(&mut self, path: PathBuf) -> String {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        let id = format!("root_{}", self.roots.len());
        let root = WorkspaceRoot {
            id: id.clone(),
            path,
            name,
            index: self.order.len(),
        };

        self.roots.insert(id.clone(), root);
        self.order.push(id.clone());

        if self.primary.is_none() {
            self.primary = Some(id.clone());
        }

        id
    }

    /// 移除根目录
    pub fn remove_root(&mut self, id: &str) -> bool {
        if self.roots.remove(id).is_some() {
            self.order.retain(|i| i != id);
            if self.primary.as_deref() == Some(id) {
                self.primary = self.order.first().cloned();
            }
            true
        } else {
            false
        }
    }

    /// 设置主根目录
    pub fn set_primary(&mut self, id: &str) {
        if self.roots.contains_key(id) {
            self.primary = Some(id.to_string());
        }
    }

    /// 获取所有根目录
    pub fn roots(&self) -> Vec<&WorkspaceRoot> {
        self.order
            .iter()
            .filter_map(|id| self.roots.get(id))
            .collect()
    }

    /// 获取主根目录
    pub fn primary_root(&self) -> Option<&WorkspaceRoot> {
        self.primary
            .as_ref()
            .and_then(|id| self.roots.get(id))
    }

    /// 获取根目录数量
    pub fn count(&self) -> usize {
        self.roots.len()
    }

    /// 判断路径属于哪个根目录
    pub fn find_root(&self, path: &PathBuf) -> Option<&WorkspaceRoot> {
        self.roots
            .values()
            .find(|r| path.starts_with(&r.path))
    }
}