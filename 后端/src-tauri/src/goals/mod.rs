pub mod continuation;
pub mod tracker;

use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::goals::{
    CreateGoalRequest, GoalContinuationData, GoalListResponse, GoalSnapshot, GoalStatus,
    TokenConsumeRequest, UpdateGoalRequest, YuanGoal,
};

use self::continuation::{Checkpoint, ContinuationManager};
use self::tracker::{BudgetStatus, BudgetTracker};

pub struct GoalManager {
    pool: SqlitePool,
    budgets: BudgetTracker,
    continuations: ContinuationManager,
}

impl GoalManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            budgets: BudgetTracker::new(),
            continuations: ContinuationManager::new(),
        }
    }

    pub async fn create_goal(&mut self, req: CreateGoalRequest) -> Result<GoalSnapshot, AppError> {
        let now = chrono::Utc::now().timestamp_millis();
        let goal_uuid = uuid::Uuid::new_v4().to_string();
        let priority = req.priority.unwrap_or(0);

        let goal: YuanGoal = sqlx::query_as::<_, YuanGoal>(
            "INSERT INTO yuan_goals (goal_uuid, session_id, title, description, status, priority, progress_pct, parent_goal_id, agent_id, token_budget, tokens_used, thread_json, created_at, updated_at)
             VALUES (?, ?, ?, ?, 'pending', ?, 0, ?, ?, ?, 0, '{}', ?, ?)
             RETURNING *",
        )
        .bind(&goal_uuid)
        .bind(&req.session_id)
        .bind(&req.title)
        .bind(&req.description)
        .bind(priority)
        .bind(req.parent_goal_id)
        .bind(&req.agent_id)
        .bind(req.token_budget)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if let Some(budget) = req.token_budget {
            self.budgets.register(goal.id, budget, 0);
        }

        Ok(GoalSnapshot::from(goal))
    }

    pub async fn get_goal(&self, goal_id: i64) -> Result<GoalSnapshot, AppError> {
        let goal: YuanGoal = sqlx::query_as::<_, YuanGoal>(
            "SELECT * FROM yuan_goals WHERE id = ?",
        )
        .bind(goal_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

        Ok(GoalSnapshot::from(goal))
    }

    pub async fn get_goal_by_uuid(&self, goal_uuid: &str) -> Result<GoalSnapshot, AppError> {
        let goal: YuanGoal = sqlx::query_as::<_, YuanGoal>(
            "SELECT * FROM yuan_goals WHERE goal_uuid = ?",
        )
        .bind(goal_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

        Ok(GoalSnapshot::from(goal))
    }

    pub async fn list_goals(&self, session_id: &str) -> Result<GoalListResponse, AppError> {
        let goals: Vec<YuanGoal> = sqlx::query_as::<_, YuanGoal>(
            "SELECT * FROM yuan_goals WHERE session_id = ? ORDER BY priority DESC, created_at DESC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let total = goals.len();
        let active = goals
            .iter()
            .filter(|g| GoalStatus::from_str(&g.status).is_active())
            .count();
        let snapshots: Vec<GoalSnapshot> = goals.into_iter().map(GoalSnapshot::from).collect();

        Ok(GoalListResponse {
            session_id: session_id.to_string(),
            goals: snapshots,
            total,
            active,
        })
    }

    pub async fn update_goal(&mut self, req: UpdateGoalRequest) -> Result<GoalSnapshot, AppError> {
        let existing: YuanGoal = sqlx::query_as::<_, YuanGoal>(
            "SELECT * FROM yuan_goals WHERE id = ?",
        )
        .bind(req.goal_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

        let now = chrono::Utc::now().timestamp_millis();
        let title = req.title.unwrap_or(existing.title);
        let description = req.description.unwrap_or(existing.description);
        let priority = req.priority.unwrap_or(existing.priority);
        let token_budget = req.token_budget.or(existing.token_budget);
        let agent_id = req.agent_id.or(existing.agent_id);

        let goal: YuanGoal = sqlx::query_as::<_, YuanGoal>(
            "UPDATE yuan_goals SET title = ?, description = ?, priority = ?, token_budget = ?, agent_id = ?, updated_at = ? WHERE id = ? RETURNING *",
        )
        .bind(&title)
        .bind(&description)
        .bind(priority)
        .bind(token_budget)
        .bind(&agent_id)
        .bind(now)
        .bind(req.goal_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if let Some(budget) = token_budget {
            self.budgets.update_budget(req.goal_id, budget);
        }

        Ok(GoalSnapshot::from(goal))
    }

    pub async fn start_goal(&self, goal_id: i64) -> Result<GoalSnapshot, AppError> {
        let now = chrono::Utc::now().timestamp_millis();
        let goal: YuanGoal = sqlx::query_as::<_, YuanGoal>(
            "UPDATE yuan_goals SET status = 'in_progress', updated_at = ? WHERE id = ? AND status = 'pending' RETURNING *",
        )
        .bind(now)
        .bind(goal_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Conflict("目标不在pending状态".into()))?;

        Ok(GoalSnapshot::from(goal))
    }

    pub async fn pause_goal(&self, goal_id: i64) -> Result<GoalSnapshot, AppError> {
        let now = chrono::Utc::now().timestamp_millis();
        let goal: YuanGoal = sqlx::query_as::<_, YuanGoal>(
            "UPDATE yuan_goals SET status = 'paused', updated_at = ? WHERE id = ? AND status = 'in_progress' RETURNING *",
        )
        .bind(now)
        .bind(goal_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Conflict("目标不在in_progress状态".into()))?;

        Ok(GoalSnapshot::from(goal))
    }

    pub async fn complete_goal(&self, goal_id: i64, result_json: Option<String>) -> Result<GoalSnapshot, AppError> {
        let now = chrono::Utc::now().timestamp_millis();
        let goal: YuanGoal = sqlx::query_as::<_, YuanGoal>(
            "UPDATE yuan_goals SET status = 'completed', progress_pct = 100, result_json = ?, updated_at = ? WHERE id = ? RETURNING *",
        )
        .bind(&result_json)
        .bind(now)
        .bind(goal_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

        Ok(GoalSnapshot::from(goal))
    }

    pub async fn abort_goal(&self, goal_id: i64, reason: &str) -> Result<GoalSnapshot, AppError> {
        let now = chrono::Utc::now().timestamp_millis();
        let result = serde_json::json!({ "reason": reason });
        let goal: YuanGoal = sqlx::query_as::<_, YuanGoal>(
            "UPDATE yuan_goals SET status = 'aborted', result_json = ?, updated_at = ? WHERE id = ? RETURNING *",
        )
        .bind(serde_json::to_string(&result).ok())
        .bind(now)
        .bind(goal_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

        Ok(GoalSnapshot::from(goal))
    }

    pub async fn delete_goal(&self, goal_id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM yuan_goals WHERE id = ?")
            .bind(goal_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }

    pub async fn consume_tokens(&mut self, req: TokenConsumeRequest) -> Result<BudgetStatus, AppError> {
        let now = chrono::Utc::now().timestamp_millis();
        let _goal: YuanGoal = sqlx::query_as::<_, YuanGoal>(
            "UPDATE yuan_goals SET tokens_used = tokens_used + ?, updated_at = ? WHERE id = ? RETURNING *",
        )
        .bind(req.tokens)
        .bind(now)
        .bind(req.goal_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

        let status = self
            .budgets
            .consume(req.goal_id, req.tokens)
            .unwrap_or(BudgetStatus::Safe);

        if matches!(status, BudgetStatus::Exhausted) {
            sqlx::query("UPDATE yuan_goals SET status = 'paused', updated_at = ? WHERE id = ?")
                .bind(now)
                .bind(req.goal_id)
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;
        }

        Ok(status)
    }

    pub async fn update_progress(&self, goal_id: i64, progress_pct: i32) -> Result<GoalSnapshot, AppError> {
        let now = chrono::Utc::now().timestamp_millis();
        let pct = progress_pct.clamp(0, 100);

        let goal: YuanGoal = sqlx::query_as::<_, YuanGoal>(
            "UPDATE yuan_goals SET progress_pct = ?, updated_at = ? WHERE id = ? RETURNING *",
        )
        .bind(pct)
        .bind(now)
        .bind(goal_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

        Ok(GoalSnapshot::from(goal))
    }

    pub fn save_checkpoint(&mut self, goal_id: i64, checkpoint: Checkpoint) {
        self.continuations.save_checkpoint(goal_id, checkpoint);
    }

    pub fn build_continuation(&self, goal_id: i64) -> Result<GoalContinuationData, AppError> {
        let goal_parent = self
            .continuations
            .last_checkpoint(goal_id)
            .and_then(|cp| cp.agent_id.clone());

        self.continuations
            .build_continuation(goal_id, goal_parent)
    }

    pub fn checkpoints_count(&self, goal_id: i64) -> usize {
        self.continuations.checkpoints_count(goal_id)
    }
}