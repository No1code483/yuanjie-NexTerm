use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::models::sandbox::{
    ExecuteRequest, ExecuteResult, FileOperationRequest, FileOperationResult, SandboxConfig,
    SandboxHistoryRecord, SandboxInfo, SandboxSnapshot,
};
use crate::sandbox::SandboxManager;

pub struct SandboxService {
    manager: Arc<RwLock<SandboxManager>>,
    history: Arc<RwLock<Vec<SandboxHistoryRecord>>>,
}

impl SandboxService {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(SandboxManager::new())),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn manager(&self) -> &Arc<RwLock<SandboxManager>> {
        &self.manager
    }

    pub async fn create_sandbox(&self, config: SandboxConfig) -> Result<SandboxInfo, AppError> {
        let mut mgr = self.manager.write().await;
        let sandbox = mgr.create(config)?;
        Ok(sandbox.info())
    }

    pub async fn list_sandboxes(&self) -> Result<Vec<SandboxInfo>, AppError> {
        let mgr = self.manager.read().await;
        Ok(mgr.list())
    }

    pub async fn list_sandboxes_by_agent(&self, agent_id: &str) -> Result<Vec<SandboxInfo>, AppError> {
        let mgr = self.manager.read().await;
        Ok(mgr.list_by_agent(agent_id))
    }

    pub async fn get_sandbox(&self, sandbox_id: &str) -> Result<SandboxSnapshot, AppError> {
        let mgr = self.manager.read().await;
        Ok(mgr.get(sandbox_id)?.snapshot())
    }

    pub async fn execute(&self, sandbox_id: &str, req: ExecuteRequest) -> Result<ExecuteResult, AppError> {
        let mut mgr = self.manager.write().await;
        let result = mgr.execute(sandbox_id, req.clone()).await?;

        // 记录执行历史
        let record = SandboxHistoryRecord {
            id: format!("exec_{}", chrono::Utc::now().timestamp_millis()),
            sandbox_id: sandbox_id.to_string(),
            lang: req.command.clone(),
            risk: if result.exit_code == 0 { 10.0 } else { 50.0 },
            passed: result.exit_code == 0,
            duration: format!("{}ms", result.duration_ms),
            timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
            exit_code: result.exit_code,
            stdout: result.stdout.clone(),
            stderr: result.stderr.clone(),
        };
        let mut history = self.history.write().await;
        history.push(record);
        if history.len() > 100 {
            history.remove(0);
        }

        Ok(result)
    }

    pub async fn get_history(&self) -> Result<Vec<SandboxHistoryRecord>, AppError> {
        let history = self.history.read().await;
        Ok(history.iter().rev().take(20).cloned().collect())
    }

    pub async fn read_file(&self, sandbox_id: &str, req: FileOperationRequest) -> Result<FileOperationResult, AppError> {
        let mgr = self.manager.read().await;
        mgr.read_file(sandbox_id, req).await
    }

    pub async fn write_file(&self, sandbox_id: &str, req: FileOperationRequest) -> Result<FileOperationResult, AppError> {
        let mgr = self.manager.read().await;
        mgr.write_file(sandbox_id, req).await
    }

    pub async fn delete_file(&self, sandbox_id: &str, path: &str) -> Result<FileOperationResult, AppError> {
        let mgr = self.manager.read().await;
        mgr.delete_file(sandbox_id, path).await
    }

    pub async fn list_files(&self, sandbox_id: &str, path: &str) -> Result<Vec<String>, AppError> {
        let mgr = self.manager.read().await;
        mgr.list_files(sandbox_id, path).await
    }

    pub async fn terminate_sandbox(&self, sandbox_id: &str) -> Result<(), AppError> {
        let mut mgr = self.manager.write().await;
        mgr.terminate(sandbox_id)
    }

    pub async fn cleanup_terminated(&self) -> Result<usize, AppError> {
        let mut mgr = self.manager.write().await;
        Ok(mgr.cleanup_terminated())
    }

    pub async fn get_count(&self) -> Result<usize, AppError> {
        let mgr = self.manager.read().await;
        Ok(mgr.count())
    }
}