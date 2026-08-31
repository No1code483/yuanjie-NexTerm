//! 沙箱执行层 — 对标 Codex SandboxManager
//!
//! 提供安全隔离的代码执行环境：
//! - 文件系统沙箱：路径白名单/黑名单，读写权限控制
//! - 命令执行沙箱：允许/禁止特定命令
//! - 网络沙箱：控制网络访问权限
//! - 审批策略：危险操作需要用户确认

pub mod backend;
pub mod executor;
pub mod permissions;
pub mod primitives;

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::error::app_error::AppError;
use crate::models::sandbox::{
    ExecuteRequest, ExecuteResult, FileOperationRequest, FileOperationResult, SandboxConfig,
    SandboxInfo, SandboxSnapshot, SandboxState,
};
use crate::sandbox::executor::SandboxExecutor;
use crate::sandbox::primitives::validate_config;

/// 沙箱实例
#[derive(Debug)]
pub struct SandboxInstance {
    info: SandboxInfo,
    config: SandboxConfig,
    state: SandboxState,
    total_duration_ms: u64,
}

impl SandboxInstance {
    pub fn info(&self) -> SandboxInfo {
        self.info.clone()
    }

    pub fn snapshot(&self) -> SandboxSnapshot {
        SandboxSnapshot {
            sandbox_id: self.info.sandbox_id.clone(),
            sandbox_type: self.info.sandbox_type.clone(),
            state: format!("{:?}", self.state),
            permission_name: self.info.permission_name.clone(),
            agent_id: self.info.agent_id.clone(),
            working_dir: self.config.working_dir.clone(),
            timeout_ms: self.config.timeout_ms,
            created_at: self.info.created_at,
            executions_count: self.info.executions_count,
            total_duration_ms: self.total_duration_ms,
        }
    }
}

/// 沙箱管理器 — 对标 Codex SandboxManager
#[derive(Debug)]
pub struct SandboxManager {
    instances: HashMap<String, SandboxInstance>,
    next_id: AtomicUsize,
}

impl SandboxManager {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            next_id: AtomicUsize::new(0),
        }
    }

    /// 创建沙箱
    pub fn create(&mut self, config: SandboxConfig) -> Result<&SandboxInstance, AppError> {
        validate_config(&config)?;

        let id = format!(
            "sandbox_{}",
            self.next_id.fetch_add(1, Ordering::Relaxed)
        );

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let instance = SandboxInstance {
            info: SandboxInfo {
                sandbox_id: id.clone(),
                sandbox_type: format!("{:?}", config.sandbox_type),
                permission_name: config.permission_profile.name.clone(),
                state: "Idle".into(),
                agent_id: config.agent_id.clone(),
                created_at: now,
                executions_count: 0,
            },
            config,
            state: SandboxState::Idle,
            total_duration_ms: 0,
        };

        self.instances.insert(id, instance);
        Ok(self.instances.values().last().unwrap())
    }

    /// 列出所有沙箱
    pub fn list(&self) -> Vec<SandboxInfo> {
        self.instances
            .values()
            .map(|s| s.info.clone())
            .collect()
    }

    /// 按 Agent 列出沙箱
    pub fn list_by_agent(&self, agent_id: &str) -> Vec<SandboxInfo> {
        self.instances
            .values()
            .filter(|s| s.info.agent_id.as_deref() == Some(agent_id))
            .map(|s| s.info.clone())
            .collect()
    }

    /// 获取沙箱
    pub fn get(&self, sandbox_id: &str) -> Result<&SandboxInstance, AppError> {
        self.instances
            .get(sandbox_id)
            .ok_or(AppError::NotFound)
    }

    /// 执行命令
    pub async fn execute(
        &mut self,
        sandbox_id: &str,
        req: ExecuteRequest,
    ) -> Result<ExecuteResult, AppError> {
        let instance = self
            .instances
            .get_mut(sandbox_id)
            .ok_or(AppError::NotFound)?;

        instance.state = SandboxState::Running;
        instance.info.executions_count += 1;

        let result = SandboxExecutor::execute_command(
            &req,
            &instance.config.permission_profile,
            instance.config.timeout_ms,
        )
        .await;

        instance.state = SandboxState::Idle;
        instance.total_duration_ms += result
            .as_ref()
            .map(|r| r.duration_ms)
            .unwrap_or(0);

        result
    }

    /// 读取文件
    pub async fn read_file(
        &self,
        sandbox_id: &str,
        req: FileOperationRequest,
    ) -> Result<FileOperationResult, AppError> {
        let instance = self.instances.get(sandbox_id).ok_or(AppError::NotFound)?;
        SandboxExecutor::read_file(&req, &instance.config.permission_profile, &instance.config.working_dir).await
    }

    /// 写入文件
    pub async fn write_file(
        &self,
        sandbox_id: &str,
        req: FileOperationRequest,
    ) -> Result<FileOperationResult, AppError> {
        let instance = self.instances.get(sandbox_id).ok_or(AppError::NotFound)?;
        SandboxExecutor::write_file(&req, &instance.config.permission_profile, &instance.config.working_dir).await
    }

    /// 删除文件
    pub async fn delete_file(
        &self,
        sandbox_id: &str,
        path: &str,
    ) -> Result<FileOperationResult, AppError> {
        let instance = self.instances.get(sandbox_id).ok_or(AppError::NotFound)?;
        SandboxExecutor::delete_file(path, &instance.config.permission_profile, &instance.config.working_dir).await
    }

    /// 列出文件
    pub async fn list_files(
        &self,
        sandbox_id: &str,
        path: &str,
    ) -> Result<Vec<String>, AppError> {
        let instance = self.instances.get(sandbox_id).ok_or(AppError::NotFound)?;
        SandboxExecutor::list_files(path, &instance.config.permission_profile, &instance.config.working_dir).await
    }

    /// 终止沙箱
    pub fn terminate(&mut self, sandbox_id: &str) -> Result<(), AppError> {
        if let Some(instance) = self.instances.get_mut(sandbox_id) {
            instance.state = SandboxState::Terminated;
            Ok(())
        } else {
            Err(AppError::NotFound)
        }
    }

    /// 清理已终止的沙箱
    pub fn cleanup_terminated(&mut self) -> usize {
        let before = self.instances.len();
        self.instances
            .retain(|_, s| s.state != SandboxState::Terminated);
        before - self.instances.len()
    }

    /// 活跃沙箱数量
    pub fn count(&self) -> usize {
        self.instances.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sandbox() {
        let mut manager = SandboxManager::new();
        let config = SandboxConfig::default();
        let result = manager.create(config);
        assert!(result.is_ok());
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_terminate_and_cleanup() {
        let mut manager = SandboxManager::new();
        let sandbox = manager.create(SandboxConfig::default()).unwrap();
        let id = sandbox.info.sandbox_id.clone();

        manager.terminate(&id).unwrap();
        let cleaned = manager.cleanup_terminated();
        assert_eq!(cleaned, 1);
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_list_by_agent() {
        let mut manager = SandboxManager::new();
        let mut config = SandboxConfig::default();
        config.agent_id = Some("agent_0".into());
        manager.create(config).unwrap();

        let mut config2 = SandboxConfig::default();
        config2.agent_id = Some("agent_1".into());
        manager.create(config2).unwrap();

        assert_eq!(manager.list_by_agent("agent_0").len(), 1);
        assert_eq!(manager.list_by_agent("agent_1").len(), 1);
        assert_eq!(manager.list_by_agent("agent_2").len(), 0);
    }
}