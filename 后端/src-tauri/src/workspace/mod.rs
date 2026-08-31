//! 工作区管理 — 对标 VSCode Workspace
//!
//! 提供多根工作区、工作区配置、最近工作区列表等功能。

pub mod config;
pub mod recent;
pub mod roots;
pub mod settings;

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::app_error::AppError;

/// 工作区信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    /// 工作区 ID
    pub id: String,
    /// 工作区名称
    pub name: String,
    /// 根目录列表
    pub roots: Vec<WorkspaceRoot>,
    /// 工作区配置文件路径
    pub config_path: Option<PathBuf>,
    /// 创建时间
    pub created_at: i64,
    /// 最后打开时间
    pub last_opened: i64,
    /// 打开次数
    pub open_count: u32,
}

/// 工作区根目录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRoot {
    /// 路径
    pub path: PathBuf,
    /// 显示名称
    pub name: String,
    /// 是否为主根目录
    pub is_primary: bool,
}

/// 工作区管理器
#[derive(Debug)]
pub struct WorkspaceManager {
    /// 当前活跃工作区
    active: Option<WorkspaceInfo>,
    /// 工作区索引
    workspaces: HashMap<String, WorkspaceInfo>,
    /// 最近工作区管理器
    recent: recent::RecentWorkspaces,
    /// 全局设置
    global_settings: settings::WorkspaceSettings,
}

impl WorkspaceManager {
    pub fn new() -> Self {
        Self {
            active: None,
            workspaces: HashMap::new(),
            recent: recent::RecentWorkspaces::new(),
            global_settings: settings::WorkspaceSettings::default(),
        }
    }

    /// 打开工作区
    pub fn open(&mut self, paths: Vec<PathBuf>) -> Result<&WorkspaceInfo, AppError> {
        if paths.is_empty() {
            return Err(AppError::Validation("工作区路径不能为空".into()));
        }

        let roots: Vec<WorkspaceRoot> = paths
            .into_iter()
            .enumerate()
            .map(|(i, path)| {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                WorkspaceRoot {
                    path,
                    name,
                    is_primary: i == 0,
                }
            })
            .collect();

        let primary_name = &roots[0].name;
        let id = format!(
            "ws_{}_{}",
            primary_name,
            chrono::Utc::now().timestamp()
        );

        let now = chrono::Utc::now().timestamp();

        let info = WorkspaceInfo {
            id: id.clone(),
            name: primary_name.clone(),
            roots,
            config_path: None,
            created_at: now,
            last_opened: now,
            open_count: 0,
        };

        self.workspaces.insert(id.clone(), info.clone());
        self.recent.add(&info);
        self.active = Some(info.clone());

        Ok(self.workspaces.get(&id).unwrap())
    }

    /// 获取当前活跃工作区
    pub fn active(&self) -> Option<&WorkspaceInfo> {
        self.active.as_ref()
    }

    /// 获取工作区根目录列表
    pub fn roots(&self) -> Vec<&PathBuf> {
        self.active
            .iter()
            .flat_map(|ws| ws.roots.iter().map(|r| &r.path))
            .collect()
    }

    /// 获取工作区文件夹名
    pub fn folder_name(&self) -> Option<&str> {
        self.active.as_ref().map(|ws| ws.name.as_str())
    }

    /// 获取最近工作区列表
    pub fn recent_workspaces(&self) -> Vec<WorkspaceInfo> {
        self.recent.list()
    }

    /// 获取工作区设置
    pub fn settings(&self) -> &settings::WorkspaceSettings {
        &self.global_settings
    }

    /// 更新工作区设置
    pub fn update_settings(&mut self, settings: settings::WorkspaceSettings) {
        self.global_settings = settings;
    }

    /// 关闭工作区
    pub fn close(&mut self) {
        self.active = None;
    }

    /// 获取工作区中的所有文件
    pub fn all_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Some(ref ws) = self.active {
            for root in &ws.roots {
                Self::collect_files(&root.path, &mut files);
            }
        }
        files
    }

    /// 递归收集文件
    fn collect_files(dir: &PathBuf, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // 跳过隐藏目录和 node_modules
                    if let Some(name) = path.file_name() {
                        let name = name.to_string_lossy();
                        if name.starts_with('.') || name == "node_modules" || name == "target" {
                            continue;
                        }
                    }
                    Self::collect_files(&path, files);
                } else if path.is_file() {
                    files.push(path);
                }
            }
        }
    }

    /// 在工作区中搜索文件
    pub fn search_files(&self, query: &str) -> Vec<PathBuf> {
        self.all_files()
            .into_iter()
            .filter(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().to_lowercase().contains(&query.to_lowercase()))
                    .unwrap_or(false)
            })
            .collect()
    }
}