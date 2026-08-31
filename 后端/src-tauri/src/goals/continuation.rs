use serde::{Deserialize, Serialize};

use crate::error::app_error::AppError;
use crate::models::goals::GoalContinuationData;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub agent_id: Option<String>,
    pub last_message: String,
    pub turn_count: u32,
    pub tokens_used: i64,
    pub sandbox_id: Option<String>,
    pub file_snapshot_json: Option<String>,
    pub timestamp: i64,
}

impl Checkpoint {
    pub fn new(
        agent_id: Option<String>,
        last_message: String,
        turn_count: u32,
        tokens_used: i64,
    ) -> Self {
        Self {
            agent_id,
            last_message,
            turn_count,
            tokens_used,
            sandbox_id: None,
            file_snapshot_json: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn with_sandbox(mut self, sandbox_id: String) -> Self {
        self.sandbox_id = Some(sandbox_id);
        self
    }
}

pub struct ContinuationManager {
    checkpoints: std::collections::HashMap<i64, Vec<Checkpoint>>,
}

impl ContinuationManager {
    pub fn new() -> Self {
        Self {
            checkpoints: std::collections::HashMap::new(),
        }
    }

    pub fn save_checkpoint(&mut self, goal_id: i64, checkpoint: Checkpoint) {
        self.checkpoints
            .entry(goal_id)
            .or_default()
            .push(checkpoint);
    }

    pub fn last_checkpoint(&self, goal_id: i64) -> Option<&Checkpoint> {
        self.checkpoints
            .get(&goal_id)
            .and_then(|cps| cps.last())
    }

    pub fn checkpoints_count(&self, goal_id: i64) -> usize {
        self.checkpoints
            .get(&goal_id)
            .map(|cps| cps.len())
            .unwrap_or(0)
    }

    pub fn build_continuation(
        &self,
        goal_id: i64,
        parent_agent_id: Option<String>,
    ) -> Result<GoalContinuationData, AppError> {
        let last_cp = self
            .last_checkpoint(goal_id)
            .ok_or_else(|| AppError::NotFound)?;

        let last_context = format!(
            "恢复执行 Goal #{}: 已执行 {} 轮, 已消耗 {} tokens. 最后消息: {}",
            goal_id, last_cp.turn_count, last_cp.tokens_used, last_cp.last_message
        );

        Ok(GoalContinuationData {
            goal_id,
            last_context,
            parent_agent_id: parent_agent_id.or(last_cp.agent_id.clone()),
            checkpoint_data: None,
        })
    }

    pub fn remove_goal(&mut self, goal_id: i64) {
        self.checkpoints.remove(&goal_id);
    }

    pub fn cleanup_old(&mut self, older_than_ms: i64) -> usize {
        let now = chrono::Utc::now().timestamp_millis();
        let mut removed = 0;

        self.checkpoints.retain(|_, cps| {
            cps.retain(|cp| {
                let keep = now - cp.timestamp < older_than_ms;
                if !keep {
                    removed += 1;
                }
                keep
            });
            !cps.is_empty()
        });

        removed
    }
}