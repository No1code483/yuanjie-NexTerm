//! Agent 控制编排 — 对标 Codex agent/control.rs
//!
//! 多 Agent 编排的核心模块，负责：
//! - 创建/销毁/分叉 Agent
//! - 列出/查询 Agent 状态
//! - 向 Agent 发送任务
//! - 在 Session 中注入 Agent 上下文

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::agent::communication::AgentCommunication;
use crate::agent::registry::AgentRegistry;
use crate::agent::status::AgentStatus;
use crate::error::app_error::AppError;
use crate::models::agent::{AgentRoleConfig, LiveAgentSnapshot};

/// 单个 Agent 实例
#[derive(Debug, Clone)]
pub struct Agent {
    pub agent_id: String,
    pub session_id: String,
    pub role: AgentRoleConfig,
    pub parent_id: Option<String>,
    status: AgentStatus,
    pub turns_executed: u32,
    pub tokens_used: i64,
    pub spawned_at: i64,
}

impl Agent {
    pub fn snapshot(&self) -> LiveAgentSnapshot {
        LiveAgentSnapshot {
            agent_id: self.agent_id.clone(),
            session_id: self.session_id.clone(),
            role_name: self.role.name.clone(),
            status: self.status.as_str().to_string(),
            parent_id: self.parent_id.clone(),
            max_turns: self.role.max_turns,
            turns_executed: self.turns_executed,
            token_budget: self.role.token_budget,
            tokens_used: self.tokens_used,
            spawned_at: self.spawned_at,
        }
    }

    pub fn status(&self) -> &AgentStatus {
        &self.status
    }

    pub fn increment_turn(&mut self) -> Result<(), AppError> {
        if self.turns_executed >= self.role.max_turns {
            return Err(AppError::Internal(format!(
                "Agent {} 已达到最大回合数 {}",
                self.agent_id, self.role.max_turns
            )));
        }
        self.turns_executed += 1;
        Ok(())
    }

    pub fn add_tokens(&mut self, tokens: i64) -> Result<(), AppError> {
        if let Some(budget) = self.role.token_budget {
            if self.tokens_used + tokens > budget {
                return Err(AppError::Internal(format!(
                    "Agent {} Token 预算超限: 已用 {}/{}",
                    self.agent_id, self.tokens_used, budget
                )));
            }
        }
        self.tokens_used += tokens;
        Ok(())
    }

    pub fn transition(&mut self, target: AgentStatus) {
        self.status = target;
    }
}

/// Agent 控制中心 — 对标 Codex AgentControl
pub struct AgentControl {
    next_id: AtomicUsize,
    agents: HashMap<String, Agent>,
    /// 按 session_id 分组
    session_agents: HashMap<String, Vec<String>>,
    registry: AgentRegistry,
    communication: AgentCommunication,
}

impl AgentControl {
    pub fn new() -> Self {
        Self {
            next_id: AtomicUsize::new(0),
            agents: HashMap::new(),
            session_agents: HashMap::new(),
            registry: AgentRegistry::new(50, 10),
            communication: AgentCommunication::new(),
        }
    }

    /// 创建 Agent
    pub fn spawn_agent(
        &mut self,
        session_id: &str,
        role: AgentRoleConfig,
        parent_id: Option<String>,
    ) -> Result<&Agent, AppError> {
        let agent_id = format!("agent_{}", self.next_id.fetch_add(1, Ordering::Relaxed));

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let agent = Agent {
            agent_id: agent_id.clone(),
            session_id: session_id.to_string(),
            role,
            parent_id,
            status: AgentStatus::Running,
            turns_executed: 0,
            tokens_used: 0,
            spawned_at: now,
        };

        self.agents.insert(agent_id.clone(), agent);
        self.session_agents
            .entry(session_id.to_string())
            .or_default()
            .push(agent_id.clone());

        Ok(self.agents.get(&agent_id).unwrap())
    }

    /// 分叉 Agent（从父 Agent 继承角色）
    pub fn fork_agent(
        &mut self,
        parent_agent_id: &str,
        new_role: Option<AgentRoleConfig>,
    ) -> Result<&Agent, AppError> {
        let parent = self
            .agents
            .get(parent_agent_id)
            .ok_or_else(|| AppError::NotFound)?;

        let role = new_role.unwrap_or_else(|| parent.role.clone());
        let session_id = parent.session_id.clone();

        self.spawn_agent(&session_id, role, Some(parent_agent_id.to_string()))
    }

    /// 列出 Session 的所有 Agent
    pub fn list_session_agents(&self, session_id: &str) -> Vec<LiveAgentSnapshot> {
        self.session_agents
            .get(session_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.agents.get(id))
                    .map(|a| a.snapshot())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 列出所有 Agent
    pub fn list_all_agents(&self) -> Vec<LiveAgentSnapshot> {
        self.agents.values().map(|a| a.snapshot()).collect()
    }

    /// 获取 Agent
    pub fn get_agent(&self, agent_id: &str) -> Option<&Agent> {
        self.agents.get(agent_id)
    }

    /// 获取可变 Agent
    pub fn get_agent_mut(&mut self, agent_id: &str) -> Option<&mut Agent> {
        self.agents.get_mut(agent_id)
    }

    /// 终止 Agent
    pub fn abort_agent(&mut self, agent_id: &str, reason: String) -> Result<(), AppError> {
        let agent = self
            .agents
            .get_mut(agent_id)
            .ok_or(AppError::NotFound)?;
        agent.status = AgentStatus::Errored(reason);
        Ok(())
    }

    /// 状态转换
    pub fn transition_agent(&mut self, agent_id: &str, target: AgentStatus) -> Result<(), AppError> {
        let agent = self
            .agents
            .get_mut(agent_id)
            .ok_or(AppError::NotFound)?;
        agent.transition(target);
        Ok(())
    }

    /// 增加回合数
    pub fn increment_turn(&mut self, agent_id: &str) -> Result<(), AppError> {
        let agent = self
            .agents
            .get_mut(agent_id)
            .ok_or(AppError::NotFound)?;
        agent.increment_turn()
    }

    /// 增加 Token 使用量
    pub fn add_tokens(&mut self, agent_id: &str, tokens: i64) -> Result<(), AppError> {
        let agent = self
            .agents
            .get_mut(agent_id)
            .ok_or(AppError::NotFound)?;
        agent.add_tokens(tokens)
    }

    /// 列出所有可用角色
    pub fn list_roles(&self) -> Vec<AgentRoleConfig> {
        self.registry
            .role_registry()
            .list_all()
            .into_iter()
            .map(|r| AgentRoleConfig {
                name: r.name.clone(),
                system_prompt: r.system_prompt.clone().unwrap_or_default(),
                available_tools: r.allowed_tools.clone(),
                permission_level: crate::models::agent::PermissionLevel::Full,
                max_turns: 100,
                token_budget: None,
            })
            .collect()
    }

    /// 获取 Agent 通信模块
    pub fn communication(&self) -> &AgentCommunication {
        &self.communication
    }

    /// 获取注册表
    pub fn registry(&self) -> &AgentRegistry {
        &self.registry
    }

    /// 清理已终止的 Agent
    pub fn cleanup_terminal_agents(&mut self) -> usize {
        let before = self.agents.len();
        self.agents.retain(|_, a| a.status.is_active());
        before - self.agents.len()
    }

    /// Agent 总数
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// 活跃 Agent 数
    pub fn active_agent_count(&self) -> usize {
        self.agents
            .values()
            .filter(|a| a.status.is_active())
            .count()
    }
}