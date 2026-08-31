//! D4.6 游戏数据底层智能监测接入（非侵入式钩子）
//!
//! 设计依据：
//! - .trae/rules/项目核心设计意图.md §二（底层智能渗透所有模块，非侵入式，可关闭）
//! - .trae/rules/项目核心设计意图.md §8.2（底层智能必须支持一键开关，关闭后各模块核心功能仍可用）
//!
//! 核心能力：
//! - record_event()：将游戏事件写入底层智能 activity_logs 表（module='game'），
//!   供底层智能的行为分析/建议引擎消费。非侵入式：游戏核心流程不依赖此钩子。
//! - get_status()：查询钩子是否启用 + 近期游戏事件数。
//!
//! 边界（严格遵守项目核心设计意图 §二、§8.2）：
//! - 此钩子仅"喂数据给底层智能监测"，不替代游戏的 AI 调用（D4.4/D4.5/D4.6 走云端 API）
//! - 底层智能关闭（is_enabled() == false）时，钩子完全 no-op，但游戏核心功能仍可用
//! - 非侵入式：游戏服务不依赖此钩子即可工作；钩子失败不影响游戏流程（调用方忽略错误）

use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::services::intelligence_service::IntelligenceService;

/// 游戏事件类型（写入 activity_logs.operation）
#[derive(Debug, Clone)]
pub enum GameEventType {
    Build,
    Upgrade,
    Remove,
    Breakthrough,
    NpcChat,
    StoryAdvance,
    Custom(String),
}

impl GameEventType {
    fn as_str(&self) -> &str {
        match self {
            GameEventType::Build => "game_build",
            GameEventType::Upgrade => "game_upgrade",
            GameEventType::Remove => "game_remove",
            GameEventType::Breakthrough => "game_breakthrough",
            GameEventType::NpcChat => "game_npc_chat",
            GameEventType::StoryAdvance => "game_story_advance",
            GameEventType::Custom(s) => s.as_str(),
        }
    }
}

/// 钩子状态
#[derive(Debug, Clone, serde::Serialize)]
pub struct IntelligenceHookStatus {
    /// 底层智能全局开关是否启用
    pub enabled: bool,
    /// 近 7 天游戏事件数（module='game'）
    pub recent_event_count: i64,
}

/// 记录一条游戏事件供底层智能监测消费
///
/// 非侵入式 + 可关闭：
/// - 底层智能关闭时立即返回 Ok(())，不写入任何数据
/// - 写入失败仅记录日志，不向上抛错（游戏流程不应受监测钩子影响）
pub async fn record_event(
    pool: &SqlitePool,
    intelligence: &IntelligenceService,
    user_id: i64,
    event_type: GameEventType,
    detail: Option<&str>,
) -> Result<(), AppError> {
    // 可关闭性：底层智能关闭时，钩子完全 no-op
    if !intelligence.is_enabled().await {
        return Ok(());
    }

    let user_id_str = user_id.to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();

    let res = sqlx::query(
        r#"INSERT INTO activity_logs (user_id, timestamp, module, operation, detail, duration_secs)
           VALUES (?, ?, 'game', ?, ?, 0)"#,
    )
    .bind(&user_id_str)
    .bind(&timestamp)
    .bind(event_type.as_str())
    .bind(detail)
    .execute(pool)
    .await;

    if let Err(e) = res {
        // 非侵入式：监测写入失败不影响游戏流程，仅记录告警
        tracing::warn!(
            "[D4.6-hook] 游戏事件写入底层智能 activity_logs 失败（已忽略，不影响游戏）: {}",
            e
        );
    }

    Ok(())
}

/// 查询钩子状态（启用与否 + 近期游戏事件数）
pub async fn get_status(
    pool: &SqlitePool,
    intelligence: &IntelligenceService,
    user_id: i64,
) -> Result<IntelligenceHookStatus, AppError> {
    let enabled = intelligence.is_enabled().await;

    let user_id_str = user_id.to_string();
    let cutoff = {
        let now = chrono::Utc::now();
        let seven_days_ago = now - chrono::Duration::days(7);
        seven_days_ago.to_rfc3339()
    };

    let recent_event_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM activity_logs
           WHERE user_id = ? AND module = 'game' AND timestamp >= ?"#,
    )
    .bind(&user_id_str)
    .bind(&cutoff)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    Ok(IntelligenceHookStatus {
        enabled,
        recent_event_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_strings() {
        assert_eq!(GameEventType::Build.as_str(), "game_build");
        assert_eq!(GameEventType::StoryAdvance.as_str(), "game_story_advance");
        assert_eq!(
            GameEventType::Custom("game_custom".into()).as_str(),
            "game_custom"
        );
    }
}
