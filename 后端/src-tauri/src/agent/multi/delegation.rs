// 任务委派 — 对标 Codex 的多 Agent 任务委派
// 支持 Orchestrator 将子任务分发给专业 Agent

use std::collections::HashMap;
use uuid::Uuid;

use crate::error::app_error::AppError;

/// 委派任务
#[derive(Debug, Clone)]
pub struct DelegationTask {
    /// 任务 ID
    pub id: String,
    /// 任务描述
    pub description: String,
    /// 目标 Agent 角色
    pub target_role: String,
    /// 任务优先级
    pub priority: DelegationPriority,
    /// 依赖任务 ID 列表
    pub dependencies: Vec<String>,
    /// 超时（秒）
    pub timeout_secs: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 任务状态
    pub status: DelegationStatus,
    /// 执行结果
    pub result: Option<String>,
    /// 分配的 Agent ID
    pub assigned_agent_id: Option<String>,
}

/// 任务优先级
#[derive(Debug, Clone, PartialEq)]
pub enum DelegationPriority {
    Critical,
    High,
    Medium,
    Low,
}

/// 委派状态
#[derive(Debug, Clone, PartialEq)]
pub enum DelegationStatus {
    Pending,
    Assigned,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

/// 委派结果
#[derive(Debug, Clone)]
pub struct DelegationResult {
    pub task_id: String,
    pub agent_id: String,
    pub output: String,
    pub duration_ms: u64,
    pub tokens_used: u64,
}

/// 任务委派管理器
pub struct DelegationManager {
    /// 活跃任务映射
    tasks: HashMap<String, DelegationTask>,
    /// 已完成任务结果
    completed: Vec<DelegationResult>,
    /// 失败任务
    failed: Vec<(String, String)>,
}

impl DelegationManager {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            completed: Vec::new(),
            failed: Vec::new(),
        }
    }

    /// 创建委派任务
    pub fn create_task(
        &mut self,
        description: String,
        target_role: String,
        priority: DelegationPriority,
        dependencies: Vec<String>,
        timeout_secs: u64,
    ) -> DelegationTask {
        let id = format!("deleg_{}", Uuid::new_v4().to_string().split('-').next().unwrap_or("0"));
        let task = DelegationTask {
            id: id.clone(),
            description,
            target_role,
            priority,
            dependencies,
            timeout_secs,
            max_retries: 2,
            status: DelegationStatus::Pending,
            result: None,
            assigned_agent_id: None,
        };
        self.tasks.insert(id, task.clone());
        task
    }

    /// 分配 Agent 到任务
    pub fn assign_agent(&mut self, task_id: &str, agent_id: &str) -> Result<(), AppError> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| AppError::Internal(format!("任务不存在: {}", task_id)))?;

        if task.status != DelegationStatus::Pending {
            return Err(AppError::Internal("任务已分配或已完成".into()));
        }

        task.assigned_agent_id = Some(agent_id.to_string());
        task.status = DelegationStatus::Assigned;
        Ok(())
    }

    /// 开始执行任务
    pub fn start_task(&mut self, task_id: &str) -> Result<(), AppError> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| AppError::Internal(format!("任务不存在: {}", task_id)))?;

        task.status = DelegationStatus::Running;
        Ok(())
    }

    /// 完成任务
    pub fn complete_task(
        &mut self,
        task_id: &str,
        output: String,
        duration_ms: u64,
        tokens_used: u64,
    ) -> Result<(), AppError> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| AppError::Internal(format!("任务不存在: {}", task_id)))?;

        task.status = DelegationStatus::Completed;
        task.result = Some(output.clone());

        let result = DelegationResult {
            task_id: task_id.to_string(),
            agent_id: task.assigned_agent_id.clone().unwrap_or_default(),
            output,
            duration_ms,
            tokens_used,
        };
        self.completed.push(result);

        Ok(())
    }

    /// 标记任务失败
    pub fn fail_task(&mut self, task_id: &str, error: &str) -> Result<(), AppError> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| AppError::Internal(format!("任务不存在: {}", task_id)))?;

        task.status = DelegationStatus::Failed(error.to_string());
        self.failed.push((task_id.to_string(), error.to_string()));
        Ok(())
    }

    /// 取消任务
    pub fn cancel_task(&mut self, task_id: &str) -> Result<(), AppError> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| AppError::Internal(format!("任务不存在: {}", task_id)))?;

        task.status = DelegationStatus::Cancelled;
        Ok(())
    }

    /// 获取就绪任务（依赖已满足）
    pub fn ready_tasks(&self) -> Vec<&DelegationTask> {
        self.tasks
            .values()
            .filter(|t| {
                t.status == DelegationStatus::Pending
                    && t.dependencies.iter().all(|dep_id| {
                        self.tasks
                            .get(dep_id)
                            .map(|d| d.status == DelegationStatus::Completed)
                            .unwrap_or(false)
                    })
            })
            .collect()
    }

    /// 获取所有待处理任务
    pub fn pending_tasks(&self) -> Vec<&DelegationTask> {
        self.tasks
            .values()
            .filter(|t| t.status == DelegationStatus::Pending || t.status == DelegationStatus::Assigned)
            .collect()
    }

    /// 获取所有运行中任务
    pub fn running_tasks(&self) -> Vec<&DelegationTask> {
        self.tasks
            .values()
            .filter(|t| t.status == DelegationStatus::Running)
            .collect()
    }

    /// 获取任务状态
    pub fn get_task(&self, task_id: &str) -> Option<&DelegationTask> {
        self.tasks.get(task_id)
    }

    /// 获取所有已完成结果
    pub fn completed_results(&self) -> &[DelegationResult] {
        &self.completed
    }

    /// 任务统计
    pub fn stats(&self) -> DelegationStats {
        let total = self.tasks.len();
        let completed = self.tasks.values().filter(|t| t.status == DelegationStatus::Completed).count();
        let failed = self.tasks.values().filter(|t| matches!(t.status, DelegationStatus::Failed(_))).count();
        let running = self.tasks.values().filter(|t| t.status == DelegationStatus::Running).count();
        let pending = self.tasks.values().filter(|t| t.status == DelegationStatus::Pending).count();

        DelegationStats {
            total,
            completed,
            failed,
            running,
            pending,
        }
    }

    /// 清空已完成任务
    pub fn clear_completed(&mut self) {
        self.tasks.retain(|_, t| t.status != DelegationStatus::Completed);
        self.completed.clear();
    }
}

impl Default for DelegationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 委派统计
#[derive(Debug, Clone)]
pub struct DelegationStats {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub running: usize,
    pub pending: usize,
}