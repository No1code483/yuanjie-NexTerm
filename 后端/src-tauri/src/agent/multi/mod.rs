// 多 Agent 编排 v2 — 对标 Codex multi_agents_v2
// 提供完整的 Agent 树形结构、任务委派、并发执行、结果合并

pub mod delegation;
pub mod merge;
pub mod dag;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent::control::AgentControl;

use self::delegation::{DelegationManager, DelegationTask, DelegationPriority, DelegationStats};
use self::merge::{ResultMerger, MergeStrategy};
use self::dag::{DagGraph, DagStats, DagNode};

/// 多 Agent 编排器
pub struct MultiAgentOrchestrator {
    /// Agent 控制器
    _control: Arc<RwLock<AgentControl>>,
    /// 任务委派管理器
    delegation: DelegationManager,
    /// DAG 执行图
    dag: DagGraph,
    /// 活跃的编排任务
    active_orchestrations: HashMap<String, OrchestrationPlan>,
}

/// 编排计划
#[derive(Debug, Clone)]
pub struct OrchestrationPlan {
    /// 计划 ID
    pub id: String,
    /// 计划名称
    pub name: String,
    /// Agent 角色映射 (role -> count)
    pub agent_roles: HashMap<String, u32>,
    /// 委派任务列表
    pub tasks: Vec<DelegationTask>,
    /// 合并策略
    pub merge_strategy: MergeStrategy,
    /// 进度
    pub progress: f64,
}

impl MultiAgentOrchestrator {
    pub fn new(control: Arc<RwLock<AgentControl>>) -> Self {
        Self {
            _control: control,
            delegation: DelegationManager::new(),
            dag: DagGraph::new(),
            active_orchestrations: HashMap::new(),
        }
    }

    /// 创建编排计划
    pub fn create_plan(
        &mut self,
        name: String,
        agent_roles: HashMap<String, u32>,
        merge_strategy: MergeStrategy,
    ) -> OrchestrationPlan {
        let id = format!("orch_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("0"));
        let plan = OrchestrationPlan {
            id: id.clone(),
            name,
            agent_roles,
            tasks: Vec::new(),
            merge_strategy,
            progress: 0.0,
        };
        self.active_orchestrations.insert(id, plan.clone());
        plan
    }

    /// 添加委派任务
    pub fn add_delegation_task(
        &mut self,
        description: String,
        target_role: String,
        priority: DelegationPriority,
        dependencies: Vec<String>,
        timeout_secs: u64,
    ) -> DelegationTask {
        self.delegation.create_task(description, target_role, priority, dependencies, timeout_secs)
    }

    /// 获取委派统计
    pub fn delegation_stats(&self) -> DelegationStats {
        self.delegation.stats()
    }

    /// 获取就绪任务
    pub fn ready_tasks(&self) -> Vec<&DelegationTask> {
        self.delegation.ready_tasks()
    }

    /// 获取 DAG 统计
    pub fn dag_stats(&self) -> DagStats {
        self.dag.stats()
    }

    /// 获取 DAG 就绪节点
    pub fn dag_ready_nodes(&self) -> Vec<&DagNode> {
        self.dag.ready_nodes()
    }

    /// 添加 DAG 节点
    pub fn add_dag_node(
        &mut self,
        id: String,
        name: String,
        description: String,
        agent_role: String,
        dependencies: Vec<String>,
        estimated_duration_ms: u64,
    ) -> Result<(), String> {
        self.dag.add_node(id, name, description, agent_role, dependencies, estimated_duration_ms)
    }

    /// 创建合并器
    pub fn create_merger(&self, strategy: MergeStrategy) -> ResultMerger {
        ResultMerger::new(strategy)
    }

    /// 获取编排计划
    pub fn get_plan(&self, plan_id: &str) -> Option<&OrchestrationPlan> {
        self.active_orchestrations.get(plan_id)
    }

    /// 列出所有编排计划
    pub fn list_plans(&self) -> Vec<&OrchestrationPlan> {
        self.active_orchestrations.values().collect()
    }
}