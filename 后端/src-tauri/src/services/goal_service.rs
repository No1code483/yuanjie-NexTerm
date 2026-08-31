use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::goals::continuation::Checkpoint;
use crate::goals::GoalManager;
use crate::models::goals::{
    CreateGoalRequest, GoalContinuationData, GoalListResponse, GoalSnapshot,
    TokenConsumeRequest, UpdateGoalRequest,
};
use crate::goals::tracker::BudgetStatus;

pub struct GoalService {
    manager: Arc<RwLock<Option<GoalManager>>>,
}

impl GoalService {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn initialize(&self, pool: sqlx::SqlitePool) {
        let mut mgr = self.manager.write().await;
        *mgr = Some(GoalManager::new(pool));
    }

    async fn with_manager<F, T>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&GoalManager) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, AppError>> + Send + '_>>,
    {
        let mgr = self.manager.read().await;
        let mgr_ref = mgr.as_ref().ok_or(AppError::Internal("GoalManager未初始化".into()))?;
        f(mgr_ref).await
    }

    async fn with_manager_mut<F, T>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&mut GoalManager) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, AppError>> + Send + '_>>,
    {
        let mut mgr = self.manager.write().await;
        let mgr_mut = mgr.as_mut().ok_or(AppError::Internal("GoalManager未初始化".into()))?;
        f(mgr_mut).await
    }

    pub async fn create_goal(&self, req: CreateGoalRequest) -> Result<GoalSnapshot, AppError> {
        self.with_manager_mut(|mgr| Box::pin(mgr.create_goal(req)))
            .await
    }

    pub async fn get_goal(&self, goal_id: i64) -> Result<GoalSnapshot, AppError> {
        self.with_manager(|mgr| Box::pin(mgr.get_goal(goal_id)))
            .await
    }

    pub async fn list_goals(&self, session_id: &str) -> Result<GoalListResponse, AppError> {
        self.with_manager(|mgr| {
            let sid = session_id.to_string();
            Box::pin(async move { mgr.list_goals(&sid).await })
        })
        .await
    }

    pub async fn update_goal(&self, req: UpdateGoalRequest) -> Result<GoalSnapshot, AppError> {
        self.with_manager_mut(|mgr| Box::pin(mgr.update_goal(req)))
            .await
    }

    pub async fn start_goal(&self, goal_id: i64) -> Result<GoalSnapshot, AppError> {
        self.with_manager(|mgr| Box::pin(mgr.start_goal(goal_id)))
            .await
    }

    pub async fn pause_goal(&self, goal_id: i64) -> Result<GoalSnapshot, AppError> {
        self.with_manager(|mgr| Box::pin(mgr.pause_goal(goal_id)))
            .await
    }

    pub async fn complete_goal(&self, goal_id: i64, result_json: Option<String>) -> Result<GoalSnapshot, AppError> {
        self.with_manager(|mgr| Box::pin(mgr.complete_goal(goal_id, result_json)))
            .await
    }

    pub async fn abort_goal(&self, goal_id: i64, reason: &str) -> Result<GoalSnapshot, AppError> {
        let r = reason.to_string();
        self.with_manager(move |mgr| Box::pin(async move { mgr.abort_goal(goal_id, &r).await }))
            .await
    }

    pub async fn delete_goal(&self, goal_id: i64) -> Result<(), AppError> {
        self.with_manager(|mgr| Box::pin(mgr.delete_goal(goal_id)))
            .await
    }

    pub async fn consume_tokens(&self, req: TokenConsumeRequest) -> Result<BudgetStatus, AppError> {
        self.with_manager_mut(|mgr| Box::pin(mgr.consume_tokens(req)))
            .await
    }

    pub async fn update_progress(&self, goal_id: i64, pct: i32) -> Result<GoalSnapshot, AppError> {
        self.with_manager(|mgr| Box::pin(mgr.update_progress(goal_id, pct)))
            .await
    }

    pub async fn save_checkpoint(&self, goal_id: i64, checkpoint: Checkpoint) -> Result<(), AppError> {
        let mut mgr = self.manager.write().await;
        let mgr_mut = mgr.as_mut().ok_or(AppError::Internal("GoalManager未初始化".into()))?;
        mgr_mut.save_checkpoint(goal_id, checkpoint);
        Ok(())
    }

    pub async fn build_continuation(&self, goal_id: i64) -> Result<GoalContinuationData, AppError> {
        self.with_manager(|mgr| Box::pin(async move { mgr.build_continuation(goal_id) }))
            .await
    }

    pub async fn checkpoints_count(&self, goal_id: i64) -> Result<usize, AppError> {
        self.with_manager(|mgr| Box::pin(async move { Ok(mgr.checkpoints_count(goal_id)) }))
            .await
    }
}