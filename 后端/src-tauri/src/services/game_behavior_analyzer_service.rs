//! D4.6 游戏数据分析 AI 洞察服务
//!
//! 设计依据：
//! - 功能展望/模块深化/04_游戏_真实AI接入_深度.md §D4.6 数据分析 AI 洞察
//! - .trae/rules/项目核心设计意图.md §五（游戏使用云端 API 模型，非底层智能）
//!
//! 核心能力：
//! - analyze_behavior()：聚合玩家行为快照 → 调用云端 API 生成个性化洞察
//!   （学习风格、薄弱领域、突破策略建议、节奏建议等）
//!
//! 边界：
//! - 走云端 API（AiModelService::call_model），不使用底层智能模型
//! - AI 失败时降级到规则化基础洞察（基于 skill_score / streak 等纯 DB 指标）
//! - 纯只读分析，不修改任何游戏数据

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::crypto::mek_manager::MekManager;
use crate::error::app_error::AppError;
use crate::services::ai_model_service::AiModelService;

// ============================================================================
// 数据结构
// ============================================================================

/// 洞察类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightType {
    /// 学习风格
    LearningStyle,
    /// 薄弱领域
    WeakArea,
    /// 突破策略
    BreakthroughStrategy,
    /// 节奏建议
    PaceAdvice,
    /// 综合评价
    Overall,
}

impl Default for InsightType {
    fn default() -> Self {
        InsightType::Overall
    }
}

/// 优先级
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightSeverity {
    Info,
    Warning,
    Critical,
}

impl Default for InsightSeverity {
    fn default() -> Self {
        InsightSeverity::Info
    }
}

/// 单条洞察
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInsight {
    pub insight_type: InsightType,
    pub severity: InsightSeverity,
    pub title: String,
    pub content: String,
}

/// 玩家行为快照（聚合自多张游戏表，喂给 AI 分析）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BehaviorSnapshot {
    player_name: String,
    realm: String,
    build_total: i64,
    upgrade_total: i64,
    remove_total: i64,
    breakthrough_total: i64,
    breakthrough_success: i64,
    breakthrough_fail: i64,
    npc_chat_count: i64,
    skill_score: f64,
    streak: i64,
    difficulty_multiplier: f64,
    avg_score: f64,
}

/// 分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorAnalysis {
    /// 洞察列表
    pub insights: Vec<GameInsight>,
    /// 行为快照摘要（前端可展示关键指标）
    pub snapshot: BehaviorSnapshotSummary,
    /// 是否走了 AI
    pub used_ai: bool,
}

/// 行为快照摘要（脱敏给前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSnapshotSummary {
    pub build_total: i64,
    pub breakthrough_total: i64,
    pub breakthrough_success: i64,
    pub npc_chat_count: i64,
    pub skill_score: f64,
    pub streak: i64,
}

/// 分析请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeBehaviorRequest {
    pub world_id: String,
    pub player_name: String,
    pub realm: String,
    pub model_id: i64,
    pub user_id: i64,
}

// ============================================================================
// 服务实现
// ============================================================================

pub struct GameBehaviorAnalyzerService;

impl GameBehaviorAnalyzerService {
    /// 分析玩家行为，调用云端 API 生成洞察
    ///
    /// A5 Phase 3 Task 2: 新增 `is_online` 参数，离线时直接走规则化基础洞察降级
    /// （设计依据：.trae/rules/项目核心设计意图.md §五 — 游戏 AI 走云端 API，
    /// 离线时不切换本地 ollama；规则化洞察基于纯 DB 指标，不依赖网络）。
    pub async fn analyze_behavior(
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        req: AnalyzeBehaviorRequest,
        is_online: bool,
    ) -> Result<BehaviorAnalysis, AppError> {
        let snapshot = Self::collect_snapshot(pool, &req).await;

        // A5 Phase 3 Task 2: 离线时直接降级到规则化洞察（不调用云端 AI）
        let (insights, used_ai) = if !is_online {
            tracing::info!("[D4.6] 网络离线，行为分析直接降级到规则化洞察");
            (Self::fallback_insights(&snapshot), false)
        } else {
            let ai = AiModelService::new();
            let snapshot_json = serde_json::to_string_pretty(&snapshot)
                .unwrap_or_else(|_| "{}".to_string());

            let prompt = format!(
                r#"你是修仙世界游戏数据分析师。基于以下玩家行为快照，输出 3-5 条个性化洞察。

## 玩家行为快照
{snapshot}

## 洞察维度
- learning_style：学习风格（偏好建造 / 偏好突破 / 偏好社交）
- weak_area：薄弱领域（突破成功率低、连败等）
- breakthrough_strategy：突破策略建议
- pace_advice：节奏建议（是否过度专注 / 是否需要平衡）
- overall：综合评价

## 响应格式（严格 JSON 数组，不要其他文字）
[
  {{
    "insight_type": "weak_area",
    "severity": "warning",
    "title": "突破成功率偏低",
    "content": "您的突破成功率仅 30%，建议先巩固基础知识再挑战。"
  }}
]"#,
                snapshot = snapshot_json,
            );

            let system = "你是游戏数据分析师，只输出严格 JSON 数组格式，不要任何额外文字。";

            match ai
                .call_model(pool, mek_manager, req.user_id, req.model_id, &prompt, Some(system))
                .await
            {
                Ok(raw) => match serde_json::from_str::<Vec<GameInsight>>(&raw) {
                    Ok(list) if !list.is_empty() => {
                        tracing::info!("[D4.6] 行为分析 AI 生成 {} 条洞察", list.len());
                        (list, true)
                    }
                    Ok(_) => {
                        tracing::warn!("[D4.6] AI 返回空洞察列表，降级到规则分析");
                        (Self::fallback_insights(&snapshot), false)
                    }
                    Err(e) => {
                        tracing::warn!("[D4.6] AI 响应 JSON 解析失败，降级到规则分析: {}", e);
                        (Self::fallback_insights(&snapshot), false)
                    }
                },
                Err(e) => {
                    tracing::warn!("[D4.6] AI 调用失败，降级到规则分析: {}", e);
                    (Self::fallback_insights(&snapshot), false)
                }
            }
        };

        Ok(BehaviorAnalysis {
            insights,
            snapshot: BehaviorSnapshotSummary {
                build_total: snapshot.build_total,
                breakthrough_total: snapshot.breakthrough_total,
                breakthrough_success: snapshot.breakthrough_success,
                npc_chat_count: snapshot.npc_chat_count,
                skill_score: snapshot.skill_score,
                streak: snapshot.streak,
            },
            used_ai,
        })
    }

    /// 聚合玩家行为快照（只读 COUNT 查询，失败按 0 处理）
    async fn collect_snapshot(pool: &SqlitePool, req: &AnalyzeBehaviorRequest) -> BehaviorSnapshot {
        let world_id = &req.world_id;

        let build_total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_build_history WHERE world_id = ? AND event_type = 'build'",
        )
        .bind(world_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let upgrade_total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_build_history WHERE world_id = ? AND event_type = 'upgrade'",
        )
        .bind(world_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let remove_total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_build_history WHERE world_id = ? AND event_type = 'remove'",
        )
        .bind(world_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let breakthrough_total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_breakthrough_records WHERE world_id = ?",
        )
        .bind(world_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let breakthrough_success: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_breakthrough_records WHERE world_id = ? AND result = 'success'",
        )
        .bind(world_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let breakthrough_fail: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_breakthrough_records WHERE world_id = ? AND result IN ('failed', 'dropped')",
        )
        .bind(world_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let npc_chat_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_npc_conversations WHERE world_id = ?",
        )
        .bind(world_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let skill_row: Option<(f64, i64, f64, f64)> = sqlx::query_as(
            "SELECT skill_score, streak, difficulty_multiplier, avg_score FROM game_player_skill WHERE world_id = ?",
        )
        .bind(world_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        let (skill_score, streak, difficulty_multiplier, avg_score) =
            skill_row.unwrap_or((50.0, 0, 1.0, 0.0));

        BehaviorSnapshot {
            player_name: req.player_name.clone(),
            realm: req.realm.clone(),
            build_total,
            upgrade_total,
            remove_total,
            breakthrough_total,
            breakthrough_success,
            breakthrough_fail,
            npc_chat_count,
            skill_score,
            streak,
            difficulty_multiplier,
            avg_score,
        }
    }

    /// 规则化降级洞察（基于纯 DB 指标）
    fn fallback_insights(s: &BehaviorSnapshot) -> Vec<GameInsight> {
        let mut list = Vec::new();

        // 学习风格
        let style = if s.build_total >= s.breakthrough_total && s.build_total >= s.npc_chat_count {
            "建造型玩家"
        } else if s.breakthrough_total >= s.npc_chat_count {
            "突破型玩家"
        } else {
            "社交型玩家"
        };
        list.push(GameInsight {
            insight_type: InsightType::LearningStyle,
            severity: InsightSeverity::Info,
            title: style.to_string(),
            content: format!(
                "建造 {} 次、突破 {} 次、NPC 对话 {} 次，您偏好 {}。",
                s.build_total, s.breakthrough_total, s.npc_chat_count,
                if style == "建造型玩家" {
                    "发展建筑"
                } else if style == "突破型玩家" {
                    "挑战突破"
                } else {
                    "与 NPC 交流"
                }
            ),
        });

        // 突破成功率
        if s.breakthrough_total > 0 {
            let success_rate = s.breakthrough_success as f64 / s.breakthrough_total as f64;
            if success_rate < 0.5 {
                list.push(GameInsight {
                    insight_type: InsightType::WeakArea,
                    severity: InsightSeverity::Warning,
                    title: "突破成功率偏低".to_string(),
                    content: format!(
                        "突破成功率仅 {:.0}%，建议先巩固知识领域积分再挑战。",
                        success_rate * 100.0
                    ),
                });
            }
        }

        // 连胜/连败
        if s.streak >= 3 {
            list.push(GameInsight {
                insight_type: InsightType::BreakthroughStrategy,
                severity: InsightSeverity::Info,
                title: "连胜势头".to_string(),
                content: format!("已连胜 {} 次，状态正佳，可尝试更高难度。", s.streak),
            });
        } else if s.streak <= -3 {
            list.push(GameInsight {
                insight_type: InsightType::BreakthroughStrategy,
                severity: InsightSeverity::Critical,
                title: "连败预警".to_string(),
                content: format!(
                    "已连败 {} 次，难度已自动下调，建议先复习薄弱知识点。",
                    s.streak.abs()
                ),
            });
        }

        // 平衡建议
        if s.build_total > 0 && s.breakthrough_total == 0 && s.npc_chat_count == 0 {
            list.push(GameInsight {
                insight_type: InsightType::PaceAdvice,
                severity: InsightSeverity::Info,
                title: "建议平衡发展".to_string(),
                content: "您只建造未突破，可尝试突破考验或与 NPC 交流获取更多乐趣。".to_string(),
            });
        }

        // 综合评价
        if s.build_total == 0 && s.breakthrough_total == 0 && s.npc_chat_count == 0 {
            list.push(GameInsight {
                insight_type: InsightType::Overall,
                severity: InsightSeverity::Info,
                title: "新手起步".to_string(),
                content: "尚无足够行为数据，多建造、突破或与 NPC 对话后将获得更精准的洞察。".to_string(),
            });
        }

        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap() -> BehaviorSnapshot {
        BehaviorSnapshot {
            player_name: "测试".into(),
            realm: "炼气期".into(),
            build_total: 5,
            upgrade_total: 2,
            remove_total: 1,
            breakthrough_total: 10,
            breakthrough_success: 3,
            breakthrough_fail: 7,
            npc_chat_count: 1,
            skill_score: 40.0,
            streak: -3,
            difficulty_multiplier: 0.8,
            avg_score: 45.0,
        }
    }

    #[test]
    fn fallback_emits_weak_area_and_streak() {
        let insights = GameBehaviorAnalyzerService::fallback_insights(&snap());
        assert!(insights.iter().any(|i| i.title == "突破成功率偏低"));
        assert!(insights.iter().any(|i| i.title == "连败预警"));
    }

    #[test]
    fn fallback_empty_snapshot() {
        let s = BehaviorSnapshot {
            player_name: "新".into(),
            realm: "炼气期".into(),
            build_total: 0,
            upgrade_total: 0,
            remove_total: 0,
            breakthrough_total: 0,
            breakthrough_success: 0,
            breakthrough_fail: 0,
            npc_chat_count: 0,
            skill_score: 50.0,
            streak: 0,
            difficulty_multiplier: 1.0,
            avg_score: 0.0,
        };
        let insights = GameBehaviorAnalyzerService::fallback_insights(&s);
        assert!(insights.iter().any(|i| i.title == "新手起步"));
    }
}
