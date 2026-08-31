//! D4.3 游戏对手 AI 决策服务
//!
//! 设计目标（04_游戏_真实AI接入_深度.md §2.2）：
//! - 基于 LLM 云端 API 的对手 AI 决策（替代原设计 DQN 训练，训练不在 Phase 0 范围）
//! - 4 种难度：Novice / Normal / Veteran / Master
//! - 4 种风格：Aggressive / Defensive / Economic / Balanced
//! - AI 调用走云端 API（AiModelService），不使用底层智能模型
//! - AI 失败时降级到启发式 mock 决策，保证游戏可用性
//!
//! 关键边界（.trae/rules/项目核心设计意图.md §五）：
//! - 游戏对手 AI 使用云端 API 模型（非底层智能模型）
//! - 游戏数据可由底层智能监测、记录

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::crypto::mek_manager::MekManager;
use crate::error::app_error::AppError;
use crate::services::ai_model_service::AiModelService;

/// 对手难度
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OpponentDifficulty {
    Novice,
    Normal,
    Veteran,
    Master,
}

impl OpponentDifficulty {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Novice => "novice",
            Self::Normal => "normal",
            Self::Veteran => "veteran",
            Self::Master => "master",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Novice => "新手：反应慢、决策保守、不学习玩家模式、有较高失误率",
            Self::Normal => "普通：平衡型决策，偶尔学习玩家模式",
            Self::Veteran => "资深：反应快、决策激进、主动学习并针对玩家弱点",
            Self::Master => "大师：极佳决策、主动反制玩家策略、几乎不失误",
        }
    }

    /// 启发式 fallback 中的次优概率（Novice 最高）
    pub fn suboptimal_rate(self) -> f64 {
        match self {
            Self::Novice => 0.35,
            Self::Normal => 0.15,
            Self::Veteran => 0.05,
            Self::Master => 0.0,
        }
    }
}

/// 对手风格
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OpponentStyle {
    Aggressive,
    Defensive,
    Economic,
    Balanced,
}

impl OpponentStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aggressive => "aggressive",
            Self::Defensive => "defensive",
            Self::Economic => "economic",
            Self::Balanced => "balanced",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Aggressive => "激进型：优先攻击/挑战，偏好高风险高回报",
            Self::Defensive => "防守型：优先建造防御、稳扎稳打",
            Self::Economic => "经济型：优先发展资源、积累道基",
            Self::Balanced => "平衡型：综合发展，根据局势灵活调整",
        }
    }
}

/// 决策时前端传入的当前游戏状态快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpponentGameState {
    pub player_realm: String,
    pub player_dao_foundation: i64,
    pub player_buildings: Vec<String>,
    pub player_points: i64,
    pub opponent_realm: String,
    pub opponent_dao_foundation: i64,
    pub opponent_buildings: Vec<String>,
    pub opponent_points: i64,
    pub round: i64,
    pub phase: String,
}

/// 历史决策条目（用于上下文，让 Master/Veteran 难度能"学习"玩家）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpponentHistoryEntry {
    pub round: i64,
    pub actor: String,      // "player" | "opponent"
    pub action: String,
    pub target: String,
    pub outcome: String,   // "success" | "failed" | "neutral"
}

/// 决策请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpponentDecisionRequest {
    pub difficulty: OpponentDifficulty,
    pub style: OpponentStyle,
    pub game_state: OpponentGameState,
    #[serde(default)]
    pub recent_history: Vec<OpponentHistoryEntry>,
    pub user_id: i64,
    pub model_id: i64,
}

/// AI 决策结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpponentDecision {
    pub action: String,
    pub target: String,
    #[serde(default)]
    pub params: serde_json::Value,
    pub reasoning: String,
    pub taunt: Option<String>,
    /// true 表示走的是 AI 云端 API；false 表示降级到启发式 mock
    pub used_ai: bool,
}

pub struct GameOpponentService;

impl GameOpponentService {
    /// 生成对手本回合决策：调用云端 API，失败时降级到启发式 mock
    pub async fn make_decision(
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        request: OpponentDecisionRequest,
    ) -> Result<OpponentDecision, AppError> {
        let system_prompt = Self::build_system_prompt(request.difficulty, request.style);
        let user_prompt = Self::build_user_prompt(&request);

        let ai_service = AiModelService::new();
        match ai_service
            .call_model(
                pool,
                mek_manager,
                request.user_id,
                request.model_id,
                &user_prompt,
                Some(&system_prompt),
            )
            .await
        {
            Ok(raw) => match Self::parse_decision(&raw) {
                Some(mut decision) => {
                    decision.used_ai = true;
                    Ok(decision)
                }
                None => {
                    tracing::warn!(
                        raw = %raw,
                        "对手 AI 返回内容无法解析为决策 JSON，降级到启发式 mock"
                    );
                    Ok(Self::fallback_decision(request.difficulty, request.style, &request.game_state))
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "对手 AI 调用失败，降级到启发式 mock");
                Ok(Self::fallback_decision(request.difficulty, request.style, &request.game_state))
            }
        }
    }

    fn build_system_prompt(difficulty: OpponentDifficulty, style: OpponentStyle) -> String {
        format!(
            "你是一个修仙游戏中的对手 AI，控制一个与玩家竞争的修仙者角色。\n\
             你的性格风格：{style_desc}\n\
             你的难度等级：{diff_desc}\n\n\
             你必须基于当前游戏状态做出本回合的一个决策，返回严格 JSON（不要任何额外文字、不要 markdown 代码块）。\n\n\
             决策类型：\n\
             - build: 建造新建筑（target=建筑类型，如 academy/scripture_pavilion/alchemy_lab 等）\n\
             - upgrade: 升级已有建筑（target=建筑类型）\n\
             - destroy: 拆除建筑（target=建筑类型）\n\
             - defend: 进入防御态势（target=self）\n\
             - cultivate: 闭关修炼积累资源与道基（target=self）\n\
             - challenge: 主动挑战玩家（target=player）\n\n\
             输出格式（严格 JSON）：\n\
             {{\"action\": \"...\", \"target\": \"...\", \"reasoning\": \"决策理由(30字内)\", \"taunt\": \"挑衅台词(20字内)\"}}\n\n\
             注意：\n\
             1. taunt 可为空字符串\n\
             2. 决策必须符合你的风格与难度\n\
             3. 不要输出 JSON 以外的任何内容",
            style_desc = style.description(),
            diff_desc = difficulty.description(),
        )
    }

    fn build_user_prompt(request: &OpponentDecisionRequest) -> String {
        let s = &request.game_state;
        let history_text = if request.recent_history.is_empty() {
            "（无历史记录，第一回合）".to_string()
        } else {
            request
                .recent_history
                .iter()
                .map(|h| format!("- 回合{} [{}]: {} {} → {}", h.round, h.actor, h.action, h.target, h.outcome))
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            "当前回合：{round}\n\
             当前阶段：{phase}\n\n\
             【玩家状态】\n\
             - 境界：{player_realm}\n\
             - 道基：{player_dao}\n\
             - 建筑：{player_buildings}\n\
             - 积分：{player_points}\n\n\
             【你的状态（对手）】\n\
             - 境界：{opp_realm}\n\
             - 道基：{opp_dao}\n\
             - 建筑：{opp_buildings}\n\
             - 积分：{opp_points}\n\n\
             【最近决策历史】\n\
             {history}\n\n\
             请基于你的性格与难度，做出本回合的决策。只输出 JSON。",
            round = s.round,
            phase = s.phase,
            player_realm = s.player_realm,
            player_dao = s.player_dao_foundation,
            player_buildings = if s.player_buildings.is_empty() {
                "（无）".to_string()
            } else {
                s.player_buildings.join(", ")
            },
            player_points = s.player_points,
            opp_realm = s.opponent_realm,
            opp_dao = s.opponent_dao_foundation,
            opp_buildings = if s.opponent_buildings.is_empty() {
                "（无）".to_string()
            } else {
                s.opponent_buildings.join(", ")
            },
            opp_points = s.opponent_points,
            history = history_text,
        )
    }

    /// 解析 AI 返回的 JSON 决策（容错：去 markdown 代码块、提取首个 JSON 对象）
    fn parse_decision(raw: &str) -> Option<OpponentDecision> {
        let trimmed = raw.trim();

        // 去除 ```json ... ``` 包裹
        let cleaned = if trimmed.starts_with("```") {
            let inner = trimmed
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();
            inner
        } else {
            trimmed
        };

        // 尝试直接解析
        if let Ok(d) = serde_json::from_str::<OpponentDecision>(cleaned) {
            return Some(d);
        }

        // 容错：提取首个 {...} 块
        if let (Some(start), Some(end)) = (cleaned.find('{'), cleaned.rfind('}')) {
            if end > start {
                if let Ok(d) = serde_json::from_str::<OpponentDecision>(&cleaned[start..=end]) {
                    return Some(d);
                }
            }
        }

        None
    }

    /// 启发式 fallback 决策（AI 不可用时使用）
    /// 根据风格选 action，根据难度决定是否做次优选择
    fn fallback_decision(
        difficulty: OpponentDifficulty,
        style: OpponentStyle,
        state: &OpponentGameState,
    ) -> OpponentDecision {
        // 次优概率判定：若命中，则做 cultivate（保守积累）
        let roll: f64 = {
            // 简单哈希伪随机，基于回合数避免纯确定性
            let seed = (state.round as u64).wrapping_mul(2654435761);
            ((seed >> 32) as f64) / (u32::MAX as f64)
        };

        if roll < difficulty.suboptimal_rate() {
            return OpponentDecision {
                action: "cultivate".into(),
                target: "self".into(),
                params: json!({}),
                reasoning: "（保守积累，等待时机）".into(),
                taunt: Some("且让你一程。".into()),
                used_ai: false,
            };
        }

        let (action, target, taunt) = match style {
            OpponentStyle::Aggressive => {
                if state.round > 3 && state.opponent_dao_foundation >= state.player_dao_foundation {
                    ("challenge", "player", "受死吧！")
                } else {
                    ("build", "alchemy_lab", "且先蓄势。")
                }
            }
            OpponentStyle::Defensive => ("build", "scripture_pavilion", "稳守为上。"),
            OpponentStyle::Economic => ("cultivate", "self", "道基为先。"),
            OpponentStyle::Balanced => {
                if state.opponent_buildings.len() < 3 {
                    ("build", "academy", "徐图发展。")
                } else {
                    ("upgrade", "academy", "精进不休。")
                }
            }
        };

        OpponentDecision {
            action: action.into(),
            target: target.into(),
            params: json!({}),
            reasoning: format!("（启发式决策：{}风格）", style.as_str()),
            taunt: Some(taunt.into()),
            used_ai: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_state() -> OpponentGameState {
        OpponentGameState {
            player_realm: "foundation_building".into(),
            player_dao_foundation: 60,
            player_buildings: vec!["academy".into()],
            player_points: 120,
            opponent_realm: "foundation_building".into(),
            opponent_dao_foundation: 55,
            opponent_buildings: vec![],
            opponent_points: 100,
            round: 1,
            phase: "build".into(),
        }
    }

    #[test]
    fn test_parse_decision_plain_json() {
        let raw = r#"{"action":"build","target":"academy","reasoning":"发展","taunt":"来战"}"#;
        let d = GameOpponentService::parse_decision(raw).expect("应解析成功");
        assert_eq!(d.action, "build");
        assert_eq!(d.target, "academy");
        assert_eq!(d.taunt.as_deref(), Some("来战"));
    }

    #[test]
    fn test_parse_decision_with_codeblock() {
        let raw = "```json\n{\"action\":\"challenge\",\"target\":\"player\",\"reasoning\":\"进攻\",\"taunt\":\"\"}\n```";
        let d = GameOpponentService::parse_decision(raw).expect("应解析成功");
        assert_eq!(d.action, "challenge");
        assert_eq!(d.target, "player");
    }

    #[test]
    fn test_parse_decision_with_surrounding_text() {
        let raw = "好的，我的决策是：\n{\"action\":\"defend\",\"target\":\"self\",\"reasoning\":\"防守\",\"taunt\":\"哼\"}\n以上。";
        let d = GameOpponentService::parse_decision(raw).expect("应解析成功");
        assert_eq!(d.action, "defend");
    }

    #[test]
    fn test_parse_decision_invalid_returns_none() {
        assert!(GameOpponentService::parse_decision("不是 JSON").is_none());
    }

    #[test]
    fn test_fallback_aggressive_builds_or_challenges() {
        let state = sample_state();
        let d = GameOpponentService::fallback_decision(
            OpponentDifficulty::Master,
            OpponentStyle::Aggressive,
            &state,
        );
        // 第一回合、对手建筑不足 → 应为 build
        assert!(d.action == "build" || d.action == "challenge");
        assert!(!d.used_ai);
    }

    #[test]
    fn test_fallback_economic_cultivates() {
        let state = sample_state();
        let d = GameOpponentService::fallback_decision(
            OpponentDifficulty::Normal,
            OpponentStyle::Economic,
            &state,
        );
        // Economic 风格在非次优情况下应 cultivate（除非命中次优概率，仍是 cultivate）
        assert_eq!(d.action, "cultivate");
    }

    #[test]
    fn test_difficulty_suboptimal_rate_decreasing() {
        assert!(OpponentDifficulty::Novice.suboptimal_rate() > OpponentDifficulty::Normal.suboptimal_rate());
        assert!(OpponentDifficulty::Normal.suboptimal_rate() > OpponentDifficulty::Veteran.suboptimal_rate());
        assert_eq!(OpponentDifficulty::Master.suboptimal_rate(), 0.0);
    }
}
