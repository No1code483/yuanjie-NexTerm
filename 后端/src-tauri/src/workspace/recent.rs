//! 最近打开的工作区

use std::collections::VecDeque;

use super::WorkspaceInfo;

/// 最近工作区管理器
#[derive(Debug)]
pub struct RecentWorkspaces {
    /// 最近工作区列表 (最多 20 个)
    workspaces: VecDeque<WorkspaceInfo>,
    /// 最大容量
    max_capacity: usize,
}

impl RecentWorkspaces {
    pub fn new() -> Self {
        Self {
            workspaces: VecDeque::new(),
            max_capacity: 20,
        }
    }

    /// 添加工作区到最近列表
    pub fn add(&mut self, workspace: &WorkspaceInfo) {
        // 移除已存在的相同工作区
        self.workspaces.retain(|w| w.id != workspace.id);

        // 如果超过容量，移除最旧的
        if self.workspaces.len() >= self.max_capacity {
            self.workspaces.pop_back();
        }

        // 添加为第一个（最近）
        let mut info = workspace.clone();
        info.last_opened = chrono::Utc::now().timestamp();
        info.open_count += 1;
        self.workspaces.push_front(info);
    }

    /// 获取最近工作区列表
    pub fn list(&self) -> Vec<WorkspaceInfo> {
        self.workspaces.iter().cloned().collect()
    }

    /// 移除工作区
    pub fn remove(&mut self, id: &str) {
        self.workspaces.retain(|w| w.id != id);
    }

    /// 清空列表
    pub fn clear(&mut self) {
        self.workspaces.clear();
    }

    /// 获取数量
    pub fn count(&self) -> usize {
        self.workspaces.len()
    }
}