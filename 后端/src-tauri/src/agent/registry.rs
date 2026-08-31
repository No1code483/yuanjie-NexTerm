//! Agent 注册表 — 对标 Codex agent/registry.rs
//!
//! 管理活跃 Agent 的生命周期，强制限制：
//! - 每个 Session 的最大子 Agent 数量
//! - Agent 嵌套深度
//! - Agent 昵称唯一性

use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::agent::role::RoleRegistry;
use crate::agent::status::AgentStatus;

/// Agent 树节点元数据
#[derive(Debug, Clone)]
pub struct AgentMetadata {
    /// Agent 名称
    pub agent_name: String,
    /// Agent 角色
    pub agent_role: String,
    /// 父 Agent ID（None = 根 Agent）
    pub parent_id: Option<String>,
    /// 当前状态
    pub status: AgentStatus,
    /// 最后任务描述
    pub last_task: Option<String>,
    /// 嵌套深度
    pub depth: i32,
    /// 创建时间戳
    pub created_at: u64,
}

/// 活跃 Agent 追踪
#[derive(Default)]
struct ActiveAgents {
    /// agent_id -> metadata
    agent_map: HashMap<String, AgentMetadata>,
    /// 已使用的昵称集合
    used_nicknames: HashSet<String>,
    /// 昵称重置计数
    nickname_reset_count: usize,
}

/// Agent 注册表 — 对标 Codex AgentRegistry
///
/// 线程安全，由 AgentControl 持有
pub struct AgentRegistry {
    active_agents: Mutex<ActiveAgents>,
    /// 历史总 Agent 数
    total_count: AtomicUsize,
    /// 最大并发 Agent 数
    max_concurrent: usize,
    /// 最大嵌套深度
    max_depth: i32,
    /// 角色注册表
    role_registry: RoleRegistry,
}

impl AgentRegistry {
    pub fn new(max_concurrent: usize, max_depth: i32) -> Self {
        Self {
            active_agents: Mutex::new(ActiveAgents::default()),
            total_count: AtomicUsize::new(0),
            max_concurrent,
            max_depth,
            role_registry: RoleRegistry::new(),
        }
    }

    /// 预分配 Agent 槽位，成功返回 agent_id
    pub fn reserve_spawn_slot(
        &self,
        agent_name: &str,
        agent_role: &str,
        parent_id: Option<&str>,
        depth: i32,
        created_at: u64,
    ) -> Result<String, String> {
        let mut agents = self.active_agents.lock().map_err(|e| e.to_string())?;

        // 检查并发限制
        if agents.agent_map.len() >= self.max_concurrent {
            return Err(format!(
                "Agent 并发数已达上限 ({})",
                self.max_concurrent
            ));
        }

        // 检查深度限制
        if depth > self.max_depth {
            return Err(format!(
                "Agent 嵌套深度已达上限 ({})",
                self.max_depth
            ));
        }

        // 生成唯一昵称
        let nickname = self.generate_nickname(agent_name, agent_role);
        let agent_id = format!("agent_{}", self.total_count.fetch_add(1, Ordering::Relaxed));

        agents.agent_map.insert(
            agent_id.clone(),
            AgentMetadata {
                agent_name: nickname.clone(),
                agent_role: agent_role.to_string(),
                parent_id: parent_id.map(|s| s.to_string()),
                status: AgentStatus::PendingInit,
                last_task: None,
                depth,
                created_at,
            },
        );

        agents.used_nicknames.insert(nickname);
        Ok(agent_id)
    }

    /// 释放 Agent 槽位
    pub fn release_agent(&self, agent_id: &str) -> Option<AgentMetadata> {
        let mut agents = self.active_agents.lock().ok()?;
        let meta = agents.agent_map.remove(agent_id)?;
        agents.used_nicknames.remove(&meta.agent_name);
        Some(meta)
    }

    /// 更新 Agent 状态
    pub fn update_status(&self, agent_id: &str, status: AgentStatus) -> Result<(), String> {
        let mut agents = self.active_agents.lock().map_err(|e| e.to_string())?;
        let meta = agents
            .agent_map
            .get_mut(agent_id)
            .ok_or_else(|| format!("Agent 不存在: {agent_id}"))?;
        meta.status = status;
        Ok(())
    }

    /// 更新 Agent 最后任务
    pub fn update_last_task(&self, agent_id: &str, task: &str) -> Result<(), String> {
        let mut agents = self.active_agents.lock().map_err(|e| e.to_string())?;
        let meta = agents
            .agent_map
            .get_mut(agent_id)
            .ok_or_else(|| format!("Agent 不存在: {agent_id}"))?;
        meta.last_task = Some(task.to_string());
        Ok(())
    }

    /// 获取 Agent 元数据
    pub fn get_agent(&self, agent_id: &str) -> Option<AgentMetadata> {
        let agents = self.active_agents.lock().ok()?;
        agents.agent_map.get(agent_id).cloned()
    }

    /// 列出所有活跃 Agent
    pub fn list_active(&self) -> Vec<(String, AgentMetadata)> {
        let agents = self.active_agents.lock().ok();
        match agents {
            Some(map) => map
                .agent_map
                .iter()
                .map(|(id, meta)| (id.clone(), meta.clone()))
                .collect(),
            None => Vec::new(),
        }
    }

    /// 列出子 Agent
    pub fn list_children(&self, parent_id: &str) -> Vec<(String, AgentMetadata)> {
        let agents = self.active_agents.lock().ok();
        match agents {
            Some(map) => map
                .agent_map
                .iter()
                .filter(|(_, meta)| meta.parent_id.as_deref() == Some(parent_id))
                .map(|(id, meta)| (id.clone(), meta.clone()))
                .collect(),
            None => Vec::new(),
        }
    }

    /// 活跃 Agent 数量
    pub fn active_count(&self) -> usize {
        self.active_agents
            .lock()
            .map(|a| a.agent_map.len())
            .unwrap_or(0)
    }

    /// 总 Agent 数（历史累计）
    pub fn total_count(&self) -> usize {
        self.total_count.load(Ordering::Relaxed)
    }

    /// 获取角色注册表
    pub fn role_registry(&self) -> &RoleRegistry {
        &self.role_registry
    }

    /// 终止所有 Agent
    pub fn shutdown_all(&self) -> Vec<String> {
        let mut agents = self.active_agents.lock().unwrap();
        let ids: Vec<String> = agents.agent_map.keys().cloned().collect();
        agents.agent_map.clear();
        agents.used_nicknames.clear();
        ids
    }

    /// 生成唯一昵称 — 对标 Codex format_agent_nickname
    fn generate_nickname(&self, agent_name: &str, agent_role: &str) -> String {
        let candidates = self
            .role_registry
            .get(agent_role)
            .map(|r| r.nickname_candidates.clone())
            .unwrap_or_default();

        let agents = self.active_agents.lock().unwrap();

        // 如果角色有候选昵称，随机选一个未使用的
        if !candidates.is_empty() {
            let mut rng = rand::thread_rng();
            let available: Vec<&String> = candidates
                .iter()
                .filter(|n| !agents.used_nicknames.contains(*n))
                .collect();
            if let Some(nickname) = available.choose(&mut rng) {
                return (*nickname).clone();
            }
        }

        // 回退：数字后缀
        let base = format!("{agent_name}-{agent_role}");
        let count = agents.nickname_reset_count + 1;
        format!("{base}-{count}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reserve_and_release() {
        let registry = AgentRegistry::new(10, 5);
        let id = registry
            .reserve_spawn_slot("test", "default", None, 0, 0)
            .unwrap();
        assert_eq!(registry.active_count(), 1);

        registry.release_agent(&id);
        assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn test_max_concurrent_limit() {
        let registry = AgentRegistry::new(2, 5);
        registry.reserve_spawn_slot("a", "default", None, 0, 0).unwrap();
        registry.reserve_spawn_slot("b", "default", None, 0, 0).unwrap();
        let result = registry.reserve_spawn_slot("c", "default", None, 0, 0);
        assert!(result.is_err());
    }
}