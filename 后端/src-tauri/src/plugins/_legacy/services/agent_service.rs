use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent::status::AgentStatus;
use crate::agent::AgentControl;
use crate::error::app_error::AppError;
use crate::models::agent::{
    AgentListResponse, AgentMessageRequest, AgentRoleConfig, AgentSpawnRequest,
    LiveAgentSnapshot,
};

pub struct AgentService {
    control: Arc<RwLock<AgentControl>>,
}

impl AgentService {
    pub fn new() -> Self {
        Self {
            control: Arc::new(RwLock::new(AgentControl::new())),
        }
    }

    pub fn control(&self) -> &Arc<RwLock<AgentControl>> {
        &self.control
    }

    pub async fn spawn_agent(&self, req: AgentSpawnRequest) -> Result<LiveAgentSnapshot, AppError> {
        let mut ctrl = self.control.write().await;
        let agent = ctrl.spawn_agent(&req.session_id, req.role, req.parent_id)?;
        Ok(agent.snapshot())
    }

    pub async fn fork_agent(
        &self,
        parent_agent_id: &str,
        new_role: Option<AgentRoleConfig>,
    ) -> Result<LiveAgentSnapshot, AppError> {
        let mut ctrl = self.control.write().await;
        let agent = ctrl.fork_agent(parent_agent_id, new_role)?;
        Ok(agent.snapshot())
    }

    pub async fn list_agents(&self, session_id: &str) -> Result<AgentListResponse, AppError> {
        let ctrl = self.control.read().await;
        let agents = ctrl.list_session_agents(session_id);
        let total = agents.len();
        Ok(AgentListResponse {
            session_id: session_id.to_string(),
            agents,
            total_count: total,
        })
    }

    pub async fn list_all_agents(&self) -> Result<Vec<LiveAgentSnapshot>, AppError> {
        let ctrl = self.control.read().await;
        Ok(ctrl.list_all_agents())
    }

    pub async fn get_agent_status(&self, agent_id: &str) -> Result<String, AppError> {
        let ctrl = self.control.read().await;
        let agent = ctrl.get_agent(agent_id).ok_or(AppError::NotFound)?;
        Ok(agent.status().as_str().to_string())
    }

    pub async fn abort_agent(&self, agent_id: &str, reason: &str) -> Result<(), AppError> {
        let mut ctrl = self.control.write().await;
        ctrl.abort_agent(agent_id, reason.to_string())
    }

    pub async fn transition_agent(&self, agent_id: &str, target: AgentStatus) -> Result<(), AppError> {
        let mut ctrl = self.control.write().await;
        ctrl.transition_agent(agent_id, target)
    }

    pub async fn increment_turn(&self, agent_id: &str) -> Result<(), AppError> {
        let mut ctrl = self.control.write().await;
        ctrl.increment_turn(agent_id)
    }

    pub async fn add_tokens(&self, agent_id: &str, tokens: i64) -> Result<(), AppError> {
        let mut ctrl = self.control.write().await;
        ctrl.add_tokens(agent_id, tokens)
    }

    pub async fn send_agent_message(&self, req: AgentMessageRequest) -> Result<String, AppError> {
        let ctrl = self.control.read().await;
        let msg = ctrl.communication().send_message(req).await;
        Ok(msg.id)
    }

    pub async fn receive_agent_messages(
        &self,
        agent_id: &str,
    ) -> Result<Vec<crate::agent::communication::AgentMessage>, AppError> {
        let ctrl = self.control.read().await;
        Ok(ctrl.communication().receive_messages(agent_id).await)
    }

    pub async fn list_roles(&self) -> Result<Vec<AgentRoleConfig>, AppError> {
        let ctrl = self.control.read().await;
        let roles = ctrl.list_roles();
        Ok(roles)
    }

    pub async fn cleanup_terminal(&self) -> Result<usize, AppError> {
        let mut ctrl = self.control.write().await;
        Ok(ctrl.cleanup_terminal_agents())
    }

    pub async fn get_agent_count(&self) -> Result<(usize, usize), AppError> {
        let ctrl = self.control.read().await;
        Ok((ctrl.agent_count(), ctrl.active_agent_count()))
    }
}