use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::io_control::IoControlManager;
use crate::models::io_control::{
    ExecuteWithStreamRequest, IoControlStats, OutputConfig, OutputDeltaEvent, StreamedOutput,
};

pub struct IoControlService {
    manager: Arc<RwLock<IoControlManager>>,
}

impl IoControlService {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(IoControlManager::new(None))),
        }
    }

    pub async fn update_config(&self, config: OutputConfig) {
        let mut mgr = self.manager.write().await;
        mgr.update_config(config);
    }

    pub async fn get_config(&self) -> OutputConfig {
        let mgr = self.manager.read().await;
        mgr.get_config().clone()
    }

    pub async fn get_stats(&self) -> IoControlStats {
        let mgr = self.manager.read().await;
        mgr.get_stats().clone()
    }

    pub async fn execute_streamed(
        &self,
        req: ExecuteWithStreamRequest,
    ) -> Result<(StreamedOutput, mpsc::Receiver<OutputDeltaEvent>), AppError> {
        let (tx, rx) = mpsc::channel::<OutputDeltaEvent>(64);
        let mut mgr = self.manager.write().await;
        let result = mgr.execute_with_stream(req, tx).await?;
        Ok((result, rx))
    }

    pub async fn cancel_execution(&self, execution_id: &str) -> bool {
        let mut mgr = self.manager.write().await;
        mgr.cancel_execution(execution_id)
    }

    pub async fn kill_execution(&self, execution_id: &str) -> bool {
        let mut mgr = self.manager.write().await;
        mgr.kill_execution(execution_id).await
    }

    pub async fn send_stdin(&self, execution_id: &str, data: &str) -> bool {
        let mgr = self.manager.read().await;
        mgr.send_stdin(execution_id, data).await
    }

    pub async fn cancel_all(&self) -> usize {
        let mut mgr = self.manager.write().await;
        mgr.cancel_all()
    }
}