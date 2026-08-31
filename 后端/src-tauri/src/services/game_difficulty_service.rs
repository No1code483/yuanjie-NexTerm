//! D4.3 自适应难度系统
//!
//! 设计目标（04_游戏_真实AI接入_深度.md §2.3）：
//! - 玩家能力评估：基于突破历史（成功率 + 平均分 + 连胜）综合评分 0-100
//! - 难度调整算法：心流理论 — 挑战略高于能力（×1.1 基础偏移）
//! - 动态调整：连胜升难度（×1.05^streak），连败降难度（×0.95^|streak|）
//! - 集成到突破考验：prepare 时读取技能调整 difficulty，submit 后更新技能
//!
//! 不使用 AI 模型 — 纯规则算法，零成本，不影响可关闭性。

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::app_error::AppError;

/// D4.3 玩家能力评估记录（game_player_skill 表，单玩家世界一行）。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PlayerSkill {
    pub world_id: String,
    /// 综合能力评分 0.0-100.0（avg_score * 0.5 + success_rate * 50 + streak_bonus）。
    pub skill_score: f64,
    pub attempt_count: i64,
    pub success_count: i64,
    pub fail_count: i64,
    pub total_score: i64,
    pub avg_score: f64,
    pub last_score: i64,
    /// `success` / `failed` / `dropped` / `''`（首次）。
    pub last_result: String,
    /// 连胜（正数）/ 连败（负数）/ 0（无）。
    pub streak: i64,
    /// 难度乘数 0.6-1.5，应用到 `difficulty_coefficient` 上。
    pub difficulty_multiplier: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// D4.3 玩家技能评级标签（由 skill_score 派生）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillLabel {
    Novice,      // 0-20 新手
    Beginner,    // 21-40 初学
    Proficient,  // 41-60 熟练
    Expert,      // 61-80 精通
    Master,      // 81-100 大师
}

impl SkillLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillLabel::Novice => "novice",
            SkillLabel::Beginner => "beginner",
            SkillLabel::Proficient => "proficient",
            SkillLabel::Expert => "expert",
            SkillLabel::Master => "master",
        }
    }

    pub fn display_zh(&self) -> &'static str {
        match self {
            SkillLabel::Novice => "新手",
            SkillLabel::Beginner => "初学",
            SkillLabel::Proficient => "熟练",
            SkillLabel::Expert => "精通",
            SkillLabel::Master => "大师",
        }
    }
}

pub struct GameDifficultyService;

/// D4.3 难度乘数边界。
const MULTIPLIER_MIN: f64 = 0.6;
const MULTIPLIER_MAX: f64 = 1.5;
/// D4.3 最终难度系数边界（difficulty_coefficient * multiplier 后的 clamp 范围）。
const DIFFICULTY_MIN: f64 = 0.3;
const DIFFICULTY_MAX: f64 = 2.0;

impl GameDifficultyService {
    /// 获取或创建玩家技能记录（首次访问自动创建默认记录：skill_score=50, multiplier=1.0）。
    pub async fn get_or_create_skill(
        pool: &SqlitePool,
        world_id: &str,
    ) -> Result<PlayerSkill, AppError> {
        // 先尝试读取
        if let Some(skill) = sqlx::query_as::<_, PlayerSkill>(
            "SELECT world_id, skill_score, attempt_count, success_count, fail_count, \
             total_score, avg_score, last_score, last_result, streak, difficulty_multiplier, \
             created_at, updated_at FROM game_player_skill WHERE world_id = ?",
        )
        .bind(world_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?
        {
            return Ok(skill);
        }

        // 不存在则创建默认记录
        let now = chrono::Utc::now().timestamp_millis();
        sqlx::query(
            "INSERT INTO game_player_skill \
             (world_id, skill_score, attempt_count, success_count, fail_count, total_score, \
             avg_score, last_score, last_result, streak, difficulty_multiplier, \
             created_at, updated_at) \
             VALUES (?, 50.0, 0, 0, 0, 0, 0.0, 0, '', 0, 1.0, ?, ?)",
        )
        .bind(world_id)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        // 重新读取返回
        sqlx::query_as::<_, PlayerSkill>(
            "SELECT world_id, skill_score, attempt_count, success_count, fail_count, \
             total_score, avg_score, last_score, last_result, streak, difficulty_multiplier, \
             created_at, updated_at FROM game_player_skill WHERE world_id = ?",
        )
        .bind(world_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
    }

    /// D4.3 调整难度系数（应用到 `difficulty_coefficient(realm)` 上）。
    ///
    /// 算法：`final = clamp(base * multiplier, [0.3, 2.0])`
    ///
    /// - `base`：境界基础难度系数（来自 `difficulty_coefficient(realm)`）
    /// - `multiplier`：玩家难度乘数（0.6-1.5，连胜升/连败降）
    pub fn adjust_difficulty_coefficient(base: f64, skill: &PlayerSkill) -> f64 {
        let adjusted = base * skill.difficulty_multiplier;
        adjusted.clamp(DIFFICULTY_MIN, DIFFICULTY_MAX)
    }

    /// D4.3 突破结束后更新玩家技能记录。
    ///
    /// - `score`：本次突破得分 0-100
    /// - `result`：`success` / `failed` / `dropped`
    pub async fn update_skill_after_breakthrough(
        pool: &SqlitePool,
        world_id: &str,
        score: i64,
        result: &str,
    ) -> Result<PlayerSkill, AppError> {
        let mut skill = Self::get_or_create_skill(pool, world_id).await?;

        // 1. 更新统计
        skill.attempt_count += 1;
        skill.total_score += score;
        skill.avg_score = skill.total_score as f64 / skill.attempt_count as f64;
        skill.last_score = score;
        skill.last_result = result.to_string();

        // 2. 更新连胜/连败
        let is_success = result == "success";
        if is_success {
            skill.success_count += 1;
            skill.streak = if skill.streak > 0 { skill.streak + 1 } else { 1 };
        } else {
            skill.fail_count += 1;
            skill.streak = if skill.streak < 0 { skill.streak - 1 } else { -1 };
        }

        // 3. 重新计算难度乘数（连胜升/连败降）
        skill.difficulty_multiplier = Self::recalc_multiplier(skill.streak);

        // 4. 重新计算综合评分
        skill.skill_score = Self::recalc_skill_score(&skill);

        // 5. 持久化
        let now = chrono::Utc::now().timestamp_millis();
        sqlx::query(
            "UPDATE game_player_skill SET \
             skill_score = ?, attempt_count = ?, success_count = ?, fail_count = ?, \
             total_score = ?, avg_score = ?, last_score = ?, last_result = ?, streak = ?, \
             difficulty_multiplier = ?, updated_at = ? WHERE world_id = ?",
        )
        .bind(skill.skill_score)
        .bind(skill.attempt_count)
        .bind(skill.success_count)
        .bind(skill.fail_count)
        .bind(skill.total_score)
        .bind(skill.avg_score)
        .bind(skill.last_score)
        .bind(&skill.last_result)
        .bind(skill.streak)
        .bind(skill.difficulty_multiplier)
        .bind(now)
        .bind(world_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(skill)
    }

    /// D4.3 重新计算难度乘数（连胜 ×1.05^streak，连败 ×0.95^|streak|，clamp 0.6-1.5）。
    fn recalc_multiplier(streak: i64) -> f64 {
        if streak == 0 {
            return 1.0;
        }
        let raw = if streak > 0 {
            // 连胜：每连胜 +5% 难度
            1.0_f64 * 1.05_f64.powi(streak as i32)
        } else {
            // 连败：每连败 -5% 难度
            1.0_f64 * 0.95_f64.powi((-streak) as i32)
        };
        raw.clamp(MULTIPLIER_MIN, MULTIPLIER_MAX)
    }

    /// D4.3 重新计算综合能力评分（0-100）。
    ///
    /// 公式：`avg_score * 0.5 + success_rate * 50 + streak_bonus`
    /// - `avg_score * 0.5`：平均分占 50%（0-50 分）
    /// - `success_rate * 50`：成功率占 50%（0-50 分）
    /// - `streak_bonus`：连胜加成（每连胜 +2，上限 +10）/ 连败扣分（每连败 -2，下限 -10）
    fn recalc_skill_score(skill: &PlayerSkill) -> f64 {
        if skill.attempt_count == 0 {
            return 50.0; // 默认中立
        }

        let avg_part = skill.avg_score * 0.5; // 0-50
        let success_rate = skill.success_count as f64 / skill.attempt_count as f64;
        let success_part = success_rate * 50.0; // 0-50

        // streak_bonus：连胜 +2/次（上限 +10），连败 -2/次（下限 -10）
        let streak_bonus = if skill.streak > 0 {
            (skill.streak * 2).min(10) as f64
        } else if skill.streak < 0 {
            (skill.streak * 2).max(-10) as f64
        } else {
            0.0
        };

        (avg_part + success_part + streak_bonus).clamp(0.0, 100.0)
    }

    /// D4.3 由 skill_score 派生评级标签。
    pub fn skill_label(skill_score: f64) -> SkillLabel {
        if skill_score <= 20.0 {
            SkillLabel::Novice
        } else if skill_score <= 40.0 {
            SkillLabel::Beginner
        } else if skill_score <= 60.0 {
            SkillLabel::Proficient
        } else if skill_score <= 80.0 {
            SkillLabel::Expert
        } else {
            SkillLabel::Master
        }
    }
}

// ============================================================================
// 单元测试
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    fn mock_skill(streak: i64, multiplier: f64) -> PlayerSkill {
        PlayerSkill {
            world_id: "test".into(),
            skill_score: 50.0,
            attempt_count: 0,
            success_count: 0,
            fail_count: 0,
            total_score: 0,
            avg_score: 0.0,
            last_score: 0,
            last_result: "".into(),
            streak,
            difficulty_multiplier: multiplier,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn mock_skill_with_stats(
        attempt: i64,
        success: i64,
        avg: f64,
        streak: i64,
    ) -> PlayerSkill {
        PlayerSkill {
            world_id: "test".into(),
            skill_score: 50.0,
            attempt_count: attempt,
            success_count: success,
            fail_count: attempt - success,
            total_score: (avg * attempt as f64) as i64,
            avg_score: avg,
            last_score: 0,
            last_result: "".into(),
            streak,
            difficulty_multiplier: 1.0,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn test_adjust_difficulty_coefficient_default() {
        let skill = mock_skill(0, 1.0);
        // base 0.8 * 1.0 = 0.8
        assert!((GameDifficultyService::adjust_difficulty_coefficient(0.8, &skill) - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_adjust_difficulty_coefficient_high_multiplier() {
        let skill = mock_skill(3, 1.15);
        // base 0.8 * 1.15 = 0.92
        let result = GameDifficultyService::adjust_difficulty_coefficient(0.8, &skill);
        assert!((result - 0.92).abs() < 0.001);
    }

    #[test]
    fn test_adjust_difficulty_coefficient_low_multiplier() {
        let skill = mock_skill(-3, 0.85);
        // base 0.8 * 0.85 = 0.68
        let result = GameDifficultyService::adjust_difficulty_coefficient(0.8, &skill);
        assert!((result - 0.68).abs() < 0.001);
    }

    #[test]
    fn test_adjust_difficulty_coefficient_clamp_high() {
        let skill = mock_skill(10, 1.5);
        // base 1.5 * 1.5 = 2.25 → clamp to 2.0
        let result = GameDifficultyService::adjust_difficulty_coefficient(1.5, &skill);
        assert!((result - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_adjust_difficulty_coefficient_clamp_low() {
        let skill = mock_skill(-10, 0.6);
        // base 0.4 * 0.6 = 0.24 → clamp to 0.3
        let result = GameDifficultyService::adjust_difficulty_coefficient(0.4, &skill);
        assert!((result - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_recalc_multiplier_no_streak() {
        assert!((GameDifficultyService::recalc_multiplier(0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_recalc_multiplier_win_streak() {
        // 3 连胜：1.0 * 1.05^3 ≈ 1.1576
        let m = GameDifficultyService::recalc_multiplier(3);
        assert!(m > 1.15 && m < 1.16);
    }

    #[test]
    fn test_recalc_multiplier_lose_streak() {
        // 3 连败：1.0 * 0.95^3 ≈ 0.857
        let m = GameDifficultyService::recalc_multiplier(-3);
        assert!(m > 0.85 && m < 0.86);
    }

    #[test]
    fn test_recalc_multiplier_clamp_max() {
        // 100 连胜应 clamp 到 1.5
        let m = GameDifficultyService::recalc_multiplier(100);
        assert!((m - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_recalc_multiplier_clamp_min() {
        // 100 连败应 clamp 到 0.6
        let m = GameDifficultyService::recalc_multiplier(-100);
        assert!((m - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_recalc_skill_score_no_attempts() {
        let skill = mock_skill_with_stats(0, 0, 0.0, 0);
        let score = GameDifficultyService::recalc_skill_score(&skill);
        assert!((score - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_recalc_skill_score_perfect() {
        // 10 次 100 分全成功：50 + 50 + 10(streak bonus, capped) = 100
        let skill = mock_skill_with_stats(10, 10, 100.0, 10);
        let score = GameDifficultyService::recalc_skill_score(&skill);
        assert!((score - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_recalc_skill_score_all_fail() {
        // 10 次 0 分全失败：0 + 0 + (-10 streak penalty) = -10 → clamp 0
        let skill = mock_skill_with_stats(10, 0, 0.0, -10);
        let score = GameDifficultyService::recalc_skill_score(&skill);
        assert!((score - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_recalc_skill_score_average() {
        // 10 次 60 分 5 成功：30 + 25 + 0 = 55
        let skill = mock_skill_with_stats(10, 5, 60.0, 0);
        let score = GameDifficultyService::recalc_skill_score(&skill);
        assert!((score - 55.0).abs() < 0.001);
    }

    #[test]
    fn test_skill_label_boundaries() {
        assert_eq!(GameDifficultyService::skill_label(0.0), SkillLabel::Novice);
        assert_eq!(GameDifficultyService::skill_label(20.0), SkillLabel::Novice);
        assert_eq!(GameDifficultyService::skill_label(21.0), SkillLabel::Beginner);
        assert_eq!(GameDifficultyService::skill_label(40.0), SkillLabel::Beginner);
        assert_eq!(GameDifficultyService::skill_label(41.0), SkillLabel::Proficient);
        assert_eq!(GameDifficultyService::skill_label(60.0), SkillLabel::Proficient);
        assert_eq!(GameDifficultyService::skill_label(61.0), SkillLabel::Expert);
        assert_eq!(GameDifficultyService::skill_label(80.0), SkillLabel::Expert);
        assert_eq!(GameDifficultyService::skill_label(81.0), SkillLabel::Master);
        assert_eq!(GameDifficultyService::skill_label(100.0), SkillLabel::Master);
    }

    #[test]
    fn test_skill_label_display_zh() {
        assert_eq!(SkillLabel::Novice.display_zh(), "新手");
        assert_eq!(SkillLabel::Beginner.display_zh(), "初学");
        assert_eq!(SkillLabel::Proficient.display_zh(), "熟练");
        assert_eq!(SkillLabel::Expert.display_zh(), "精通");
        assert_eq!(SkillLabel::Master.display_zh(), "大师");
    }
}
