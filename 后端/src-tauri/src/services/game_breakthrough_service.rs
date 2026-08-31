//! 游戏 3D 重构 - 境界突破 AI 考验 Service
//!
//! Task 4.3：实现 AI 出题 / 批改 / 道基评定 / 结果处理 / 弱点记忆业务逻辑。
//! change-id: `game-3d-rebuild-refactor`
//!
//! 设计原则（12_AI考验机制.md）：
//! - **基础不牢地动山摇**：所有突破必须通过 AI 考验，杜绝虚假突破
//! - **题目紧扣学习**：基于用户实际学习的知识领域出题，不可超纲
//! - **弱点持续追踪**：暴露的弱点记录到 `weakness_json`，下次突破强化考察
//! - **失败有代价**：道基下降 + 24h 冷却；连续 3 次失败触发境界跌落
//!
//! D4.1 真实 AI 接入策略（2026-07-21）：
//! - `prepare_breakthrough` / `submit_breakthrough` 接受 `Option<&AiContext>` 参数
//! - 当传入 AiContext 时，尝试调用 AiModelService（云端 API）出题/批改
//! - AI 调用失败或解析错误时，自动降级到 mock（保证游戏可用性）
//! - 不传 AiContext 时直接走 mock（向后兼容）
//! - 遵循 `.trae/rules/项目核心设计意图.md` §五：游戏考验生成使用云端 API 模型

use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::crypto::mek_manager::MekManager;
use crate::db::repositories::game_repo;
use crate::error::app_error::AppError;
use crate::models::game::{DaoFoundation, GameWorld, RealmMajor, RealmMinor};
use crate::services::ai_model_service::AiModelService;

/// D4.1 真实 AI 调用上下文。
///
/// 由命令层构造，传入 `prepare_breakthrough` / `submit_breakthrough`。
/// `None` 时降级到 mock。
pub struct AiContext {
    pub pool: SqlitePool,
    pub mek_manager: Arc<RwLock<MekManager>>,
    pub user_id: i64,
    pub model_id: i64,
}

// ============================================================================
// 数据结构
// ============================================================================

/// 突破考验单题。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BreakthroughQuestion {
    pub question_id: String,
    /// `choice` / `short_answer` / `application` / `image_choice`（D4.5 多模态图片选择题）
    pub question_type: String,
    pub domain_id: String,
    pub knowledge_point: String,
    pub content: String,
    /// 选择题 4 选项，其他题型为 `None`。
    pub options: Option<Vec<String>>,
    /// 标准答案（选择题填字母如 `B`，简答/应用题填要点列表用 `；` 分隔）。
    pub standard_answer: String,
    /// 难度系数（0.5-1.5）。
    pub difficulty: f64,
    /// D4.5 多模态：媒体 URL（图片/音频）。`None` 表示纯文本题。
    /// image_choice 题型必填；当前无文生图能力时为占位 URL，后续接入文生图 API 后填充真实图源。
    #[serde(default)]
    pub media_url: Option<String>,
    /// D4.5 多模态：媒体类型 `"image"` / `"audio"` / `None`。
    #[serde(default)]
    pub media_type: Option<String>,
    /// D4.5 多模态：媒体内容描述（图片画面描述 / 音频内容说明）。
    /// 用于无障碍访问 + 无素材时前端占位展示"看图题"体验。
    #[serde(default)]
    pub media_description: Option<String>,
}

/// 用户答案。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BreakthroughAnswer {
    pub question_id: String,
    pub user_answer: String,
}

/// 弱点项（02 文档 §6.5.2 `WeaknessItem`）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WeaknessItem {
    pub domain_id: String,
    pub knowledge_point: String,
    pub error_count: u32,
    /// `low` / `medium` / `high`
    pub severity: String,
    pub last_exposed_at: i64,
    pub resolved: bool,
    pub resolved_at: Option<i64>,
}

/// 单题评分（4 维度加权）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuestionScore {
    pub question_id: String,
    pub score: i32,
    pub correctness: u8,
    pub completeness: u8,
    pub expression: u8,
    pub depth: u8,
    pub comment: String,
    pub weak_points: Vec<String>,
}

/// 批改结果。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GradingResult {
    pub total_score: i32,
    pub daoji_level: String,
    pub question_scores: Vec<QuestionScore>,
    pub overall_comment: String,
    pub weakness_analysis: Vec<WeaknessItem>,
}

/// 突破考验会话（prepare_breakthrough 返回，submit_breakthrough 输入）。
///
/// 不入库，由调用方临时保存（前端 zustand store 或后端内存缓存）。
/// 题目已固定，用户作答后用同一 session 提交。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BreakthroughSession {
    pub session_id: String,
    pub world_id: String,
    pub from_realm_major: String,
    pub from_realm_minor: String,
    pub target_realm_major: String,
    pub question_count: u8,
    pub difficulty_coefficient: f64,
    pub questions: Vec<BreakthroughQuestion>,
    pub created_at: i64,
}

/// 突破最终结果。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BreakthroughOutcome {
    /// `success` / `failed` / `dropped`
    pub result: String,
    pub score: i32,
    pub daoji_awarded: String,
    pub new_realm_major: String,
    pub new_realm_minor: String,
    pub new_dao_foundation: String,
    pub weakness_updated: bool,
    /// 失败/跌落时为 24h 后的时间戳；成功时为 `None`。
    pub cooldown_until: Option<i64>,
    pub ai_review: String,
    /// D4.6 深化#11：突破失败后的个性化复习建议（仅 result=failed/dropped 时可能有值）。
    /// AI 调用失败或无弱点时为 `None`。前端在结果页展示「天道建议」卡片。
    #[serde(default)]
    pub review_suggestions: Option<Vec<crate::services::game_breakthrough_deepening::ReviewSuggestion>>,
}

// ============================================================================
// 辅助函数
// ============================================================================

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn new_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// 24 小时毫秒数。
const COOLDOWN_MS: i64 = 24 * 3600 * 1000;

// ============================================================================
// 1. 题型分布与难度
// ============================================================================

/// 按 12 文档表 §2.2 返回 (choice, short_answer, application) 数量。
fn question_type_distribution(realm: RealmMajor) -> (u8, u8, u8) {
    match realm {
        RealmMajor::Mortal => (0, 0, 0),
        RealmMajor::QiRefining => (2, 1, 0),
        RealmMajor::FoundationBuilding => (2, 1, 1),
        RealmMajor::GoldenCore => (2, 2, 1),
        RealmMajor::NascentSoul => (2, 2, 2),
        RealmMajor::SpiritTransformation => (3, 3, 2),
        RealmMajor::Unity => (3, 4, 3),
        RealmMajor::Mahayana => (4, 4, 4),
        RealmMajor::Tribulation => (4, 5, 6),
        RealmMajor::Immortal => (0, 0, 15),
    }
}

/// 难度系数 = 0.5 + 序号 × 0.1，封顶 1.5。
fn difficulty_coefficient(realm: RealmMajor) -> f64 {
    let raw = 0.5 + realm.ordinal() as f64 * 0.1;
    raw.min(1.5)
}

/// D4.5 多模态：返回图片选择题数量（从 choice_count 中分出，总题数不变）。
///
/// 设计原则：
/// - 低境界（凡人~元婴）纯文本题，夯实基础
/// - 化神及以上引入图片题（观察图、结构图、流程图），贴合"看图悟道"修仙意象
/// - 数量 ≤ choice_count，避免 choice 被全部替换为 image_choice
fn image_question_count(realm: RealmMajor) -> u8 {
    match realm {
        RealmMajor::Mortal
        | RealmMajor::QiRefining
        | RealmMajor::FoundationBuilding
        | RealmMajor::GoldenCore
        | RealmMajor::NascentSoul => 0,
        RealmMajor::SpiritTransformation | RealmMajor::Unity => 1,
        RealmMajor::Mahayana | RealmMajor::Tribulation | RealmMajor::Immortal => 2,
    }
}

// ============================================================================
// 2. 道基评定（含保底规则）
// ============================================================================

/// 基于 AI 考验得分（0-100）评定道基（不含保底规则）。
///
/// 对应 02 文档 §5.1：
/// - 90-100 → 黑
/// - 75-89  → 紫
/// - 60-74  → 红（突破成功下限）
/// - 45-59  → 蓝（突破失败）
/// - 0-44   → 白（突破失败）
pub fn evaluate_daoji_from_score(score: i32) -> DaoFoundation {
    if score >= 90 {
        DaoFoundation::Black
    } else if score >= 75 {
        DaoFoundation::Purple
    } else if score >= 60 {
        DaoFoundation::Red
    } else if score >= 45 {
        DaoFoundation::Blue
    } else {
        DaoFoundation::White
    }
}

/// 应用上阶道基保底规则（仅突破成功时调用）。
///
/// 对应 02 文档 §4.4：
/// - 上阶黑 → 下阶保底红
/// - 上阶紫 → 下阶保底蓝
/// - 上阶白/蓝/红 → 无保底
pub fn apply_daoji_floor(base: DaoFoundation, prev: DaoFoundation) -> DaoFoundation {
    let floor = match prev {
        DaoFoundation::Black => DaoFoundation::Red,
        DaoFoundation::Purple => DaoFoundation::Blue,
        _ => return base,
    };
    if base.ordinal() < floor.ordinal() {
        floor
    } else {
        base
    }
}

/// 评定道基（含保底规则）。仅在突破成功时（score ≥ 60）应用保底。
pub fn evaluate_daoji_with_floor(score: i32, prev: DaoFoundation) -> DaoFoundation {
    let base = evaluate_daoji_from_score(score);
    if score >= 60 {
        apply_daoji_floor(base, prev)
    } else {
        base
    }
}

// ============================================================================
// 3. 弱点记忆
// ============================================================================

/// 合并弱点列表：旧弱点未解决 + 新暴露弱点合并、计数累加、严重度升级、最多保留 20 条。
///
/// 对应 02 文档 §6.5：
/// - 旧弱点未解决保留
/// - 已存在知识点：累加 error_count，严重度取更高级别，更新时间
/// - 按 last_exposed_at 倒序保留前 20 条
pub fn merge_weakness(prev: &[WeaknessItem], exposed: &[WeaknessItem]) -> Vec<WeaknessItem> {
    let mut result: Vec<WeaknessItem> = prev.iter().filter(|w| !w.resolved).cloned().collect();

    for new_w in exposed {
        if let Some(existing) = result
            .iter_mut()
            .find(|w| w.domain_id == new_w.domain_id && w.knowledge_point == new_w.knowledge_point)
        {
            existing.error_count = existing.error_count.saturating_add(new_w.error_count);
            existing.severity = severity_max(&existing.severity, &new_w.severity);
            existing.last_exposed_at = existing.last_exposed_at.max(new_w.last_exposed_at);
        } else {
            result.push(new_w.clone());
        }
    }

    // 按 last_exposed_at 倒序，保留前 20 条
    result.sort_by(|a, b| b.last_exposed_at.cmp(&a.last_exposed_at));
    result.truncate(20);
    result
}

/// 取两个严重度中更严重的一个（high > medium > low）。
fn severity_max(a: &str, b: &str) -> String {
    let rank = |s: &str| match s {
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    };
    if rank(a) >= rank(b) {
        a.to_string()
    } else {
        b.to_string()
    }
}

// ============================================================================
// 4. 业务函数
// ============================================================================

/// 准备突破考验：校验触发条件 + 查询历史弱点 + 出题（mock AI）。
///
/// 业务规则（02 文档 §6.1）：
/// 1. 当前境界非凡人/仙
/// 2. 修为达圆满（`realm_minor == complete`）
/// 3. 道基等级 ≥ 红
/// 4. 不在 24h 冷却期内
/// 5. 收集弱点历史 → 出题（D4.1: 优先 AI，失败降级 mock） → 返回会话
pub async fn prepare_breakthrough(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    ai_ctx: Option<&AiContext>,
) -> Result<BreakthroughSession, AppError> {
    // 多用户隔离（批次 6）：按 user_id 过滤
    let world = game_repo::get_world(pool, user_id, world_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let realm_major = RealmMajor::from_str(&world.realm_major)
        .ok_or_else(|| AppError::Internal(format!("无效的 realm_major: {}", world.realm_major)))?;

    // 1. 凡人无需突破，仙为终态
    if realm_major == RealmMajor::Mortal {
        return Err(AppError::Validation("凡人境界无需突破，请先积累修为至 100".into()));
    }
    if realm_major == RealmMajor::Immortal {
        return Err(AppError::Validation("仙境界为终态，无后续突破".into()));
    }

    // 2. 修为须达圆满
    let minor = RealmMinor::from_str(&world.realm_minor)
        .ok_or_else(|| AppError::Internal(format!("无效的 realm_minor: {}", world.realm_minor)))?;
    if minor != RealmMinor::Complete {
        return Err(AppError::Validation(format!(
            "修为未达圆满，当前 {}，需继续积累至 complete",
            minor.as_str()
        )));
    }

    // 3. 道基 ≥ 红
    let dao = DaoFoundation::from_str(&world.dao_foundation)
        .ok_or_else(|| AppError::Internal(format!("无效的 dao_foundation: {}", world.dao_foundation)))?;
    if dao.ordinal() < DaoFoundation::Red.ordinal() {
        return Err(AppError::Validation(format!(
            "道基不足以支撑突破，当前 {}，需 ≥ red",
            dao.as_str()
        )));
    }

    // 4. 冷却期检查（最近一次突破若为 failed/dropped，24h 内不可再突破）
    let recent_records = game_repo::list_breakthrough_records(pool, world_id, 1).await?;
    if let Some(latest) = recent_records.first() {
        if latest.result == "failed" || latest.result == "dropped" {
            let cooldown_until = latest.created_at + COOLDOWN_MS;
            let now = now_ms();
            if now < cooldown_until {
                let hours_left = (cooldown_until - now) / 3600000;
                return Err(AppError::Validation(format!(
                    "突破冷却中，剩余约 {} 小时",
                    hours_left.max(1)
                )));
            }
        }
    }

    // 5. 收集弱点历史
    let weakness_history = get_current_weakness(pool, world_id).await?;

    // 6. 出题（D4.1: 优先 AI，失败降级 mock）
    let target_realm = realm_major
        .next()
        .ok_or_else(|| AppError::Internal("无法获取下一阶大境界".into()))?;
    let (choice_n, short_n, app_n) = question_type_distribution(realm_major);
    // spec 阶段1 Task 1.4：question_count 不再使用 realm_major.breakthrough_question_count()，
    // 因为 Task 1.4 可能追加 image_understanding 题，实际题数以 questions.len() 为准。
    let base_difficulty = difficulty_coefficient(realm_major);

    // D4.3 自适应难度：根据玩家历史表现调整难度系数（心流理论：挑战略高于能力）
    let difficulty = match crate::services::game_difficulty_service::GameDifficultyService::get_or_create_skill(pool, world_id).await {
        Ok(skill) => {
            let d = crate::services::game_difficulty_service::GameDifficultyService::adjust_difficulty_coefficient(base_difficulty, &skill);
            tracing::info!(
                "D4.3 自适应难度: base={:.3} multiplier={:.3} → final={:.3} (streak={}, skill_score={:.1})",
                base_difficulty, skill.difficulty_multiplier, d, skill.streak, skill.skill_score
            );
            d
        }
        Err(e) => {
            tracing::warn!("D4.3: 读取玩家技能失败，降级到基础难度: {}", e);
            base_difficulty
        }
    };

    let questions = match ai_ctx {
        Some(ctx) => ai_generate_questions(
            ctx,
            world_id,
            target_realm,
            choice_n,
            short_n,
            app_n,
            difficulty,
            &weakness_history,
        )
        .await
        .unwrap_or_else(|| {
            tracing::warn!("D4.1: AI 出题失败，降级到 mock");
            mock_generate_questions(
                target_realm,
                choice_n,
                short_n,
                app_n,
                difficulty,
                &weakness_history,
            )
        }),
        None => mock_generate_questions(
            target_realm,
            choice_n,
            short_n,
            app_n,
            difficulty,
            &weakness_history,
        ),
    };

    Ok(BreakthroughSession {
        session_id: new_uuid(),
        world_id: world_id.to_string(),
        from_realm_major: world.realm_major.clone(),
        from_realm_minor: world.realm_minor.clone(),
        target_realm_major: target_realm.as_str().to_string(),
        // spec 阶段1 Task 1.4：实际题数可能含 image_understanding 追加题（KB 有图片时），
        // 因此以 questions.len() 为准，避免 question_count 与实际题目数不一致。
        question_count: questions.len() as u8,
        difficulty_coefficient: difficulty,
        questions,
        created_at: now_ms(),
    })
}

/// 提交突破考验：批改（D4.1: 优先 AI，失败降级 mock）+ 评定道基 + 结果处理 + 入库。
///
/// 返回 `BreakthroughOutcome`，包含新境界/道基/冷却等结果信息。
pub async fn submit_breakthrough(
    pool: &SqlitePool,
    user_id: i64,
    session: &BreakthroughSession,
    answers: &[BreakthroughAnswer],
    ai_ctx: Option<&AiContext>,
) -> Result<BreakthroughOutcome, AppError> {
    // 多用户隔离（批次 6）：按 user_id 过滤
    let world = game_repo::get_world(pool, user_id, &session.world_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let prev_dao = DaoFoundation::from_str(&world.dao_foundation)
        .ok_or_else(|| AppError::Internal(format!("无效的 dao_foundation: {}", world.dao_foundation)))?;
    let from_realm = RealmMajor::from_str(&session.from_realm_major)
        .ok_or_else(|| AppError::Internal(format!("无效的 from_realm_major: {}", session.from_realm_major)))?;

    // 1. 批改（D4.1: 优先 AI，失败降级 mock）
    let grading = match ai_ctx {
        Some(ctx) => ai_grade_answers(ctx, &session.questions, answers)
            .await
            .unwrap_or_else(|| {
                tracing::warn!("D4.1: AI 批改失败，降级到 mock");
                mock_grade_answers(&session.questions, answers)
            }),
        None => mock_grade_answers(&session.questions, answers),
    };

    // 2. 评定道基（含保底）
    let new_dao = evaluate_daoji_with_floor(grading.total_score, prev_dao);
    let is_success = grading.total_score >= 60;

    // 3. 查询连续失败次数（含本次即将记录的失败 → +1）
    let consecutive_failures = count_consecutive_failures(pool, &session.world_id).await?;

    // 4. D4.3 自适应难度：突破结束后更新玩家技能（不阻塞主流程）
    let result_str = if is_success {
        "success"
    } else if consecutive_failures + 1 >= 3 {
        "dropped"
    } else {
        "failed"
    };
    if let Err(e) = crate::services::game_difficulty_service::GameDifficultyService::update_skill_after_breakthrough(
        pool,
        &session.world_id,
        grading.total_score as i64,
        result_str,
    ).await {
        tracing::warn!("D4.3: 更新玩家技能失败，不影响突破结果: {}", e);
    }

    // 5. 结果处理
    if is_success {
        process_success(pool, session, &world, new_dao, &grading, from_realm).await
    } else if consecutive_failures + 1 >= 3 {
        // 连续 3 次失败 → 跌落
        process_drop(pool, session, &world, new_dao, &grading, from_realm, ai_ctx).await
    } else {
        process_failure(pool, session, &world, new_dao, &grading, ai_ctx).await
    }
}

// ============================================================================
// 5. 结果处理（成功 / 失败 / 跌落）
// ============================================================================

/// 突破成功：晋升下一大境界初期 + 应用保底 + 更新弱点（红/紫/黑时） + 记录。
async fn process_success(
    pool: &SqlitePool,
    session: &BreakthroughSession,
    world: &GameWorld,
    new_dao: DaoFoundation,
    grading: &GradingResult,
    from_realm: RealmMajor,
) -> Result<BreakthroughOutcome, AppError> {
    let target_realm = from_realm
        .next()
        .ok_or_else(|| AppError::Internal("已是最高境界".into()))?;
    let now = now_ms();

    // 弱点更新（仅红/紫/黑成功时；白/蓝不应进入此分支，但保险判断）
    let prev_weak = get_current_weakness(pool, &session.world_id).await?;
    let weakness_updated = matches!(
        new_dao,
        DaoFoundation::Red | DaoFoundation::Purple | DaoFoundation::Black
    );
    let new_weak = if weakness_updated {
        merge_weakness(&prev_weak, &grading.weakness_analysis)
    } else {
        prev_weak
    };
    let weakness_json = serde_json::to_string(&new_weak).ok();

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    // 更新世界：晋升下一大境界初期 + 新道基
    // 多用户隔离（批次 6）：WHERE 同时匹配 user_id 防止跨用户修改
    sqlx::query(
        "UPDATE game_worlds
         SET realm_major = ?, realm_minor = 'early', dao_foundation = ?, updated_at = ?
         WHERE id = ? AND user_id = ?",
    )
    .bind(target_realm.as_str())
    .bind(new_dao.as_str())
    .bind(now)
    .bind(&world.id)
    .bind(world.user_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    let from_realm_str = format!("{}:{}", session.from_realm_major, session.from_realm_minor);
    let to_realm_str = format!("{}:{}", target_realm.as_str(), RealmMinor::Early.as_str());
    let questions_json = serde_json::to_string(&session.questions).ok();
    let answers_json = serde_json::to_string(&grading.question_scores).ok();

    sqlx::query(
        "INSERT INTO game_breakthrough_records
         (id, world_id, from_realm, to_realm, score, dao_foundation_awarded, result,
          questions_json, answers_json, ai_review, weakness_json, created_at)
         VALUES (?, ?, ?, ?, ?, ?, 'success', ?, ?, ?, ?, ?)",
    )
    .bind(new_uuid())
    .bind(&session.world_id)
    .bind(&from_realm_str)
    .bind(&to_realm_str)
    .bind(grading.total_score)
    .bind(new_dao.as_str())
    .bind(&questions_json)
    .bind(&answers_json)
    .bind(&grading.overall_comment)
    .bind(weakness_json.as_deref())
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    Ok(BreakthroughOutcome {
        result: "success".to_string(),
        score: grading.total_score,
        daoji_awarded: new_dao.as_str().to_string(),
        new_realm_major: target_realm.as_str().to_string(),
        new_realm_minor: RealmMinor::Early.as_str().to_string(),
        new_dao_foundation: new_dao.as_str().to_string(),
        weakness_updated,
        cooldown_until: None,
        ai_review: grading.overall_comment.clone(),
        review_suggestions: None,
    })
}

/// 突破失败：停留原境界 + 道基更新（可能下降） + 24h 冷却 + 保留原弱点 + 个性化复习建议。
///
/// D4.6 深化#11：失败后调用云端 API 生成个性化复习路径，展示在结果页「天道建议」卡片。
async fn process_failure(
    pool: &SqlitePool,
    session: &BreakthroughSession,
    world: &GameWorld,
    new_dao: DaoFoundation,
    grading: &GradingResult,
    ai_ctx: Option<&AiContext>,
) -> Result<BreakthroughOutcome, AppError> {
    let now = now_ms();
    let cooldown_until = now + COOLDOWN_MS;

    // 失败不更新 weakness_json，保留原值
    let prev_weak = get_current_weakness(pool, &session.world_id).await?;
    let prev_weak_json = serde_json::to_string(&prev_weak).ok();

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    // 更新道基（可能下降）
    // 多用户隔离（批次 6）：WHERE 同时匹配 user_id 防止跨用户修改
    sqlx::query("UPDATE game_worlds SET dao_foundation = ?, updated_at = ? WHERE id = ? AND user_id = ?")
        .bind(new_dao.as_str())
        .bind(now)
        .bind(&world.id)
        .bind(world.user_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

    let from_realm_str = format!("{}:{}", session.from_realm_major, session.from_realm_minor);
    let questions_json = serde_json::to_string(&session.questions).ok();
    let answers_json = serde_json::to_string(&grading.question_scores).ok();

    sqlx::query(
        "INSERT INTO game_breakthrough_records
         (id, world_id, from_realm, to_realm, score, dao_foundation_awarded, result,
          questions_json, answers_json, ai_review, weakness_json, created_at)
         VALUES (?, ?, ?, NULL, ?, ?, 'failed', ?, ?, ?, ?, ?)",
    )
    .bind(new_uuid())
    .bind(&session.world_id)
    .bind(&from_realm_str)
    .bind(grading.total_score)
    .bind(new_dao.as_str())
    .bind(&questions_json)
    .bind(&answers_json)
    .bind(&grading.overall_comment)
    .bind(prev_weak_json.as_deref())
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    // D4.6 深化#11：生成个性化复习建议（不阻塞主流程，失败时为 None）
    let review_suggestions = generate_review_suggestions_safely(
        ai_ctx,
        &session.world_id,
        &grading.weakness_analysis,
    )
    .await;

    Ok(BreakthroughOutcome {
        result: "failed".to_string(),
        score: grading.total_score,
        daoji_awarded: new_dao.as_str().to_string(),
        new_realm_major: session.from_realm_major.clone(),
        new_realm_minor: session.from_realm_minor.clone(),
        new_dao_foundation: new_dao.as_str().to_string(),
        weakness_updated: false,
        cooldown_until: Some(cooldown_until),
        ai_review: grading.overall_comment.clone(),
        review_suggestions,
    })
}

/// 境界跌落：跌至前一阶大境界圆满 + 道基更新 + 24h 冷却 + 个性化复习建议。
///
/// 触发条件：连续 3 次突破失败。
async fn process_drop(
    pool: &SqlitePool,
    session: &BreakthroughSession,
    world: &GameWorld,
    new_dao: DaoFoundation,
    grading: &GradingResult,
    from_realm: RealmMajor,
    ai_ctx: Option<&AiContext>,
) -> Result<BreakthroughOutcome, AppError> {
    let now = now_ms();
    let cooldown_until = now + COOLDOWN_MS;

    let dropped_realm = previous_realm(from_realm);
    let (dropped_major, dropped_minor, dropped_xp) = match dropped_realm {
        Some(r) => (
            r.as_str().to_string(),
            RealmMinor::Complete.as_str().to_string(),
            r.xp_lower_bound(),
        ),
        None => (
            RealmMajor::Mortal.as_str().to_string(),
            RealmMinor::Complete.as_str().to_string(),
            0,
        ),
    };

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    // 跌落：realm_major 降一级，realm_minor 设为圆满，修为降至新境界下限
    // 多用户隔离（批次 6）：WHERE 同时匹配 user_id 防止跨用户修改
    sqlx::query(
        "UPDATE game_worlds
         SET realm_major = ?, realm_minor = ?, dao_foundation = ?,
             realm_xp = ?, total_xp = ?, updated_at = ?
         WHERE id = ? AND user_id = ?",
    )
    .bind(&dropped_major)
    .bind(&dropped_minor)
    .bind(new_dao.as_str())
    .bind(dropped_xp)
    .bind(dropped_xp)
    .bind(now)
    .bind(&world.id)
    .bind(world.user_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    let from_realm_str = format!("{}:{}", session.from_realm_major, session.from_realm_minor);
    let to_realm_str = format!("{}:{}", dropped_major, dropped_minor);
    let questions_json = serde_json::to_string(&session.questions).ok();
    let answers_json = serde_json::to_string(&grading.question_scores).ok();

    sqlx::query(
        "INSERT INTO game_breakthrough_records
         (id, world_id, from_realm, to_realm, score, dao_foundation_awarded, result,
          questions_json, answers_json, ai_review, weakness_json, created_at)
         VALUES (?, ?, ?, ?, ?, ?, 'dropped', ?, ?, ?, NULL, ?)",
    )
    .bind(new_uuid())
    .bind(&session.world_id)
    .bind(&from_realm_str)
    .bind(&to_realm_str)
    .bind(grading.total_score)
    .bind(new_dao.as_str())
    .bind(&questions_json)
    .bind(&answers_json)
    .bind(&grading.overall_comment)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    // D4.6 深化#11：跌落场景也生成复习建议（玩家更需要指导）
    let review_suggestions = generate_review_suggestions_safely(
        ai_ctx,
        &session.world_id,
        &grading.weakness_analysis,
    )
    .await;

    Ok(BreakthroughOutcome {
        result: "dropped".to_string(),
        score: grading.total_score,
        daoji_awarded: new_dao.as_str().to_string(),
        new_realm_major: dropped_major,
        new_realm_minor: dropped_minor,
        new_dao_foundation: new_dao.as_str().to_string(),
        weakness_updated: false,
        cooldown_until: Some(cooldown_until),
        ai_review: grading.overall_comment.clone(),
        review_suggestions,
    })
}

// ============================================================================
// 6. 辅助查询
// ============================================================================

/// 取前一阶大境界（凡人返回 `None`）。
fn previous_realm(realm: RealmMajor) -> Option<RealmMajor> {
    match realm {
        RealmMajor::Mortal => None,
        RealmMajor::QiRefining => Some(RealmMajor::Mortal),
        RealmMajor::FoundationBuilding => Some(RealmMajor::QiRefining),
        RealmMajor::GoldenCore => Some(RealmMajor::FoundationBuilding),
        RealmMajor::NascentSoul => Some(RealmMajor::GoldenCore),
        RealmMajor::SpiritTransformation => Some(RealmMajor::NascentSoul),
        RealmMajor::Unity => Some(RealmMajor::SpiritTransformation),
        RealmMajor::Mahayana => Some(RealmMajor::Unity),
        RealmMajor::Tribulation => Some(RealmMajor::Mahayana),
        RealmMajor::Immortal => Some(RealmMajor::Tribulation),
    }
}

/// 获取当前弱点列表（从最新一条突破记录的 weakness_json 反查）。
async fn get_current_weakness(
    pool: &SqlitePool,
    world_id: &str,
) -> Result<Vec<WeaknessItem>, AppError> {
    let records = game_repo::list_breakthrough_records(pool, world_id, 1).await?;
    if let Some(latest) = records.first() {
        if let Some(json) = &latest.weakness_json {
            return Ok(serde_json::from_str(json).unwrap_or_default());
        }
    }
    Ok(Vec::new())
}

/// D4.6 深化#11：安全调用个性化复习推荐（AI 失败/无 ctx/无弱点时返回 None）。
///
/// - 无 AiContext（mock 模式）→ None
/// - 弱点为空 → None（无需推荐）
/// - AI 调用失败 → None（不阻塞主流程，记 warn）
async fn generate_review_suggestions_safely(
    ai_ctx: Option<&AiContext>,
    world_id: &str,
    weakness: &[WeaknessItem],
) -> Option<Vec<crate::services::game_breakthrough_deepening::ReviewSuggestion>> {
    let ctx = ai_ctx?;
    if weakness.is_empty() {
        return None;
    }

    // 提取素材用于推荐（KB 条目 + 领域掌握情况）
    // 多用户隔离批次 3：传 ctx.user_id 仅取当前用户的 KB 素材
    let context = crate::services::game_breakthrough_deepening::extract_breakthrough_materials(
        &ctx.pool,
        ctx.user_id,
        world_id,
        weakness,
    )
    .await
    .unwrap_or_else(|e| {
        tracing::warn!("D4.6: 复习推荐素材提取失败: {}", e);
        crate::services::game_breakthrough_deepening::BreakthroughContext {
            materials: Vec::new(),
            weakness: weakness.to_vec(),
            recent_questions: Vec::new(),
            domain_levels: Vec::new(),
        }
    });

    match crate::services::game_breakthrough_deepening::ai_generate_review_suggestions(
        ctx,
        weakness,
        &context.domain_levels,
        &context.materials,
    )
    .await
    {
        Some(suggestions) if !suggestions.is_empty() => Some(suggestions),
        Some(_) => {
            tracing::info!("D4.6: AI 复习推荐返回空列表（可能无足够素材）");
            None
        }
        None => {
            tracing::warn!("D4.6: AI 复习推荐调用失败，跳过复习建议展示");
            None
        }
    }
}

/// 统计连续失败次数（result 为 `failed` 或 `dropped` 的最近记录数）。
async fn count_consecutive_failures(
    pool: &SqlitePool,
    world_id: &str,
) -> Result<u32, AppError> {
    let records = game_repo::list_breakthrough_records(pool, world_id, 50).await?;
    let mut count = 0u32;
    for r in records.iter() {
        if r.result == "failed" || r.result == "dropped" {
            count += 1;
        } else {
            break;
        }
    }
    Ok(count)
}

// ============================================================================
// 7. AI 出题 / 批改（D4.1）+ Mock 降级
// ============================================================================

/// D4.1: AI 出题 — 调用 AiModelService 生成结构化考验题目。
///
/// 返回 `None` 时调用方应降级到 `mock_generate_questions`。
/// Prompt 要求 AI 输出 JSON 数组，每项含 question_type/domain_id/knowledge_point/content/options/standard_answer/difficulty。
///
/// D4.6 深化集成（12_AI考验机制_未来展望.md）：
/// - 调用 `extract_breakthrough_materials` 提取 KB 素材 + 历史弱点 + 近 3 次去重列表
/// - 使用 `build_heavenly_examiner_system_prompt` 完整天道考官人设
/// - 解析后调用 `validate_generated_questions` 校验（错误记 warn 但不阻塞，warnings 记 info）
async fn ai_generate_questions(
    ctx: &AiContext,
    world_id: &str,
    target_realm: RealmMajor,
    choice_count: u8,
    short_count: u8,
    app_count: u8,
    difficulty: f64,
    weakness: &[WeaknessItem],
) -> Option<Vec<BreakthroughQuestion>> {
    let total = (choice_count as u32) + (short_count as u32) + (app_count as u32);
    if total == 0 {
        return None;
    }

    // D4.6 深化#1/#2：提取 KB 素材 + 历史 weaknesses + 近 3 次去重列表 + 领域掌握情况
    // 失败时返回空上下文，不阻塞出题
    // 多用户隔离批次 3：传 ctx.user_id 仅取当前用户的 KB 素材
    let context = crate::services::game_breakthrough_deepening::extract_breakthrough_materials(
        &ctx.pool,
        ctx.user_id,
        world_id,
        weakness,
    )
    .await
    .unwrap_or_else(|e| {
        tracing::warn!("D4.6: 素材提取失败，降级到无素材出题: {}", e);
        crate::services::game_breakthrough_deepening::BreakthroughContext {
            materials: Vec::new(),
            weakness: weakness.to_vec(),
            recent_questions: Vec::new(),
            domain_levels: Vec::new(),
        }
    });

    let weakness_hint = if context.weakness.is_empty() {
        "无历史弱点".to_string()
    } else {
        context
            .weakness
            .iter()
            .map(|w| format!("- 领域 {} 知识点「{}」严重度 {} 出错 {} 次", w.domain_id, w.knowledge_point, w.severity, w.error_count))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let materials_hint = if context.materials.is_empty() {
        "（KB 暂无可关联素材，请基于通用知识出题）".to_string()
    } else {
        context
            .materials
            .iter()
            .map(|m| format!("- [{}] {}（领域 {}）", m.entry_id, m.title, m.domain_id))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let dedup_hint = if context.recent_questions.is_empty() {
        "（无近期题目，无需去重）".to_string()
    } else {
        context
            .recent_questions
            .iter()
            .map(|rq| format!("- 知识点「{}」+ 题型「{}」", rq.knowledge_point, rq.question_type))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let domain_hint = if context.domain_levels.is_empty() {
        "（无领域掌握数据）".to_string()
    } else {
        context
            .domain_levels
            .iter()
            .map(|(d, l)| format!("- 领域 {} 等级 Lv.{}", d, l))
            .collect::<Vec<_>>()
            .join("\n")
    };

    // D4.6 深化#10：使用完整天道考官人设 system prompt
    let system_prompt =
        crate::services::game_breakthrough_deepening::build_heavenly_examiner_system_prompt();

    let image_count = image_question_count(target_realm);
    let user_prompt = format!(
        "玩家即将突破到「{}」境界，需要生成 {total} 道考验题：\n\
- 选择题 {choice_count} 道（4 选项 A/B/C/D，standard_answer 填字母如 \"B\"）\n\
  其中 {image_count} 道为图片选择题（question_type=\"image_choice\"），需带 media_type=\"image\"、media_description（画面描述，如\"一幅展示哈夫曼树结构的示意图，叶节点标注字符与频率\"）、media_url（占位填 \"placeholder://image/<序号>\"）\n\
- 简答题 {short_count} 道（standard_answer 用「；」分隔 3-5 个要点）\n\
- 应用题 {app_count} 道（standard_answer 用「；」分隔 4 个要点：分析；方案；实施；验证）\n\
难度系数：{difficulty}（0.5 简单 ~ 1.5 困难）\n\n\
## 玩家历史弱点（high 必出 ≥ 1 题应用题，medium 必出 ≥ 1 题）\n{weakness_hint}\n\n\
## 玩家可关联 KB 素材（题目应紧扣这些素材出题，绝不超纲）\n{materials_hint}\n\n\
## 玩家各领域掌握情况（基于此判断出题侧重）\n{domain_hint}\n\n\
## 近 3 次本境界题目去重列表（不可重复以下「知识点 + 题型」组合）\n{dedup_hint}\n\n\
输出 JSON 数组，每个元素格式：\n\
{{\"question_type\":\"choice|short_answer|application|image_choice\",\"domain_id\":\"<领域>\",\"knowledge_point\":\"<知识点>\",\"content\":\"<题干>\",\"options\":[\"A. ...\",\"B. ...\",\"C. ...\",\"D. ...\"],\"standard_answer\":\"<标准答案>\",\"difficulty\":{difficulty},\"media_type\":\"<image 或 null>\",\"media_url\":\"<图片URL或null>\",\"media_description\":\"<画面描述或null>\"}}\n\
注意：choice/image_choice 题必须带 options，short_answer/application 题 options 为 null。image_choice 题必须带 media_type/media_description，media_url 可为占位符。",
        target_realm.as_str()
    );

    let ai_service = AiModelService::new();
    let raw = ai_service
        .call_model(
            &ctx.pool,
            &ctx.mek_manager,
            ctx.user_id,
            ctx.model_id,
            &user_prompt,
            Some(system_prompt),
        )
        .await
        .ok()?;

    let trimmed = strip_json_codeblock(&raw);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&trimmed).ok()?;

    let mut questions = Vec::with_capacity(parsed.len());
    for (idx, item) in parsed.iter().enumerate() {
        let question_type = item.get("question_type")?.as_str()?.to_string();
        let domain_id = item
            .get("domain_id")
            .and_then(|v| v.as_str())
            .unwrap_or("general")
            .to_string();
        let knowledge_point = item
            .get("knowledge_point")
            .and_then(|v| v.as_str())
            .unwrap_or("综合")
            .to_string();
        let content = item.get("content")?.as_str()?.to_string();
        let standard_answer = item
            .get("standard_answer")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let options = item
            .get("options")
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|o| o.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                }
            });
        let q_difficulty = item
            .get("difficulty")
            .and_then(|v| v.as_f64())
            .unwrap_or(difficulty);

        questions.push(BreakthroughQuestion {
            question_id: format!("q{}", idx + 1),
            question_type,
            domain_id,
            knowledge_point,
            content,
            options,
            standard_answer,
            difficulty: q_difficulty,
            // D4.5 多模态：解析 AI 返回的 media_* 字段（image_choice 题型时携带）
            media_url: item
                .get("media_url")
                .and_then(|v| v.as_str())
                .map(String::from),
            media_type: item
                .get("media_type")
                .and_then(|v| v.as_str())
                .map(String::from),
            media_description: item
                .get("media_description")
                .and_then(|v| v.as_str())
                .map(String::from),
        });
    }

    if questions.is_empty() {
        None
    } else {
        // spec 阶段1 Task 1.3：image_choice 题型异步调用真实文生图 API
        // 失败降级到 placeholder:// + tracing::warn! 日志，不阻塞考验流程
        let mut questions = questions;
        for q in questions.iter_mut() {
            if q.question_type == "image_choice" {
                let url = ai_generate_image_for_question(ctx, q).await;
                q.media_url = Some(url);
            }
        }

        // spec 阶段1 Task 1.4：KB 含图片时调用 vision API 追加 1 道 image_understanding 题
        // 失败降级：跳过图片素材，使用文本素材（原题列表不变）
        if crate::services::game_breakthrough_deepening::kb_has_image_materials(&ctx.pool).await {
            match crate::services::game_breakthrough_deepening::list_kb_image_materials(
                &ctx.pool,
                5,
            )
            .await
            .first()
            {
                Some((_, title, path_url)) => {
                    let domain_hint = context
                        .domain_levels
                        .first()
                        .map(|(d, _)| d.as_str())
                        .unwrap_or("general");
                    match crate::services::game_breakthrough_deepening::ai_understand_image_for_question(
                        ctx,
                        path_url,
                        domain_hint,
                    )
                    .await
                    {
                        Ok(img_q) => {
                            tracing::info!(
                                image_title = %title,
                                "[多模态#15] 基于图片素材追加 1 道 image_understanding 题"
                            );
                            questions.push(img_q);
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                image_title = %title,
                                "[多模态#15降级] vision API 调用失败，跳过图片素材，使用文本素材出题"
                            );
                        }
                    }
                }
                None => {
                    tracing::info!("[多模态#15] kb_has_image_materials=true 但 list_kb_image_materials 返回空，跳过图片出题");
                }
            }
        } else {
            tracing::info!("[多模态#15] KB 不含图片素材，走常规文本出题流程");
        }

        // D4.6 深化#3/#4/#6：题目校验器（数量/题型/领域覆盖/弱点强化/去重）
        // 校验结果不阻塞返回（仅记日志），由调用方根据 None/Some 判断是否降级
        // Bug 修复 v1.52.19.1：expected_count 用 questions.len() 而非 total
        // （若追加了 image_understanding 题，total 不含追加题，会导致"题目数量不匹配"误报）
        let validation = crate::services::game_breakthrough_deepening::validate_generated_questions(
            &questions,
            questions.len() as u8,
            choice_count,
            short_count,
            app_count,
            &context.weakness,
            &context.recent_questions,
        );
        if !validation.is_valid {
            tracing::warn!(
                "D4.6: AI 出题校验失败（仍返回题目，但标记降级）：errors={:?}",
                validation.errors
            );
        }
        if !validation.warnings.is_empty() {
            tracing::info!(
                "D4.6: AI 出题校验警告：warnings={:?}",
                validation.warnings
            );
        }
        Some(questions)
    }
}

/// spec 阶段1 Task 1.3：为 image_choice 题生成真实图片 URL（#12 真实文生图接入）。
///
/// 流程：
/// 1. 从 question.media_description 提取画面描述（必填）
/// 2. 调用 `game_breakthrough_deepening::generate_image_for_question`（走云端 API）
/// 3. 成功 → 写入真实 URL 到 media_url
/// 4. 失败/空描述 → 降级到 `placeholder://image/<question_id>` + tracing::warn! 日志
///
/// 设计依据：
/// - .trae/rules/项目核心设计意图.md §五（游戏考验生成走云端 API）
/// - .trae/rules/项目核心设计意图.md §八.2（底层智能关闭后考验机制核心功能可用，降级到 placeholder://）
///
/// 返回：图片 URL（http(s):// 或 data:image/... 或 placeholder:// 占位）。
async fn ai_generate_image_for_question(
    ctx: &AiContext,
    question: &BreakthroughQuestion,
) -> String {
    let description = question.media_description.as_deref().unwrap_or("");

    match crate::services::game_breakthrough_deepening::generate_image_for_question(ctx, description)
        .await
    {
        Some(url) => url,
        None => {
            // 降级：保留 placeholder:// 占位（确保题目可用）
            tracing::warn!(
                question_id = %question.question_id,
                "[文生图#12降级] image_choice 题使用 placeholder:// 占位（AI 不可用或调用失败）"
            );
            format!("placeholder://image/{}", question.question_id)
        }
    }
}

/// D4.1: AI 批改 — 调用 AiModelService 对玩家答案进行 4 维度评分。
///
/// 返回 `None` 时调用方应降级到 `mock_grade_answers`。
async fn ai_grade_answers(
    ctx: &AiContext,
    questions: &[BreakthroughQuestion],
    answers: &[BreakthroughAnswer],
) -> Option<GradingResult> {
    if questions.is_empty() {
        return None;
    }

    let qa_pairs: Vec<String> = questions
        .iter()
        .map(|q| {
            let ans = answers
                .iter()
                .find(|a| a.question_id == q.question_id)
                .map(|a| a.user_answer.as_str())
                .unwrap_or("(未作答)");
            format!(
                "题号 {} [{}] 知识点「{}」题干：{}\n标准答案：{}\n玩家答案：{}",
                q.question_id,
                q.question_type,
                q.knowledge_point,
                q.content,
                q.standard_answer,
                ans
            )
        })
        .collect();

    let system_prompt =
        crate::services::game_breakthrough_deepening::build_heavenly_grader_system_prompt();

    let user_prompt = format!(
        "请批改以下 {} 道题：\n\n{}\n\n\
输出 JSON 格式：\n\
{{\"question_scores\":[{{\"question_id\":\"q1\",\"correctness\":80,\"completeness\":70,\"expression\":85,\"depth\":60,\"comment\":\"评语\",\"weak_points\":[\"暴露的弱点知识点\"]}}],\"overall_comment\":\"总体评语\"}}\n\
score 字段 = (correctness+completeness+expression+depth)/4 取整。\n\
weak_points 仅在得分 < 60 时填入知识点；否则为空数组。",
        questions.len(),
        qa_pairs.join("\n\n")
    );

    let ai_service = AiModelService::new();
    let raw = ai_service
        .call_model(
            &ctx.pool,
            &ctx.mek_manager,
            ctx.user_id,
            ctx.model_id,
            &user_prompt,
            Some(system_prompt),
        )
        .await
        .ok()?;

    let trimmed = strip_json_codeblock(&raw);
    let parsed: serde_json::Value = serde_json::from_str(&trimmed).ok()?;

    let question_scores_arr = parsed.get("question_scores")?.as_array()?;
    let mut question_scores = Vec::with_capacity(question_scores_arr.len());
    let mut weakness_analysis = Vec::new();
    let mut total_score = 0i64;

    for (idx, q) in questions.iter().enumerate() {
        let item = question_scores_arr.get(idx)?;
        let correctness = item.get("correctness").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        let completeness = item.get("completeness").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        let expression = item.get("expression").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        let depth = item.get("depth").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        let score = ((correctness as u32 + completeness as u32 + expression as u32 + depth as u32) / 4) as i32;
        let comment = item
            .get("comment")
            .and_then(|v| v.as_str())
            .unwrap_or("AI 评语")
            .to_string();
        let weak_points: Vec<String> = item
            .get("weak_points")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|w| w.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        if !weak_points.is_empty() {
            let severity = if score <= 30 {
                "high"
            } else if score <= 60 {
                "medium"
            } else {
                "low"
            };
            for wp in &weak_points {
                weakness_analysis.push(WeaknessItem {
                    domain_id: q.domain_id.clone(),
                    knowledge_point: wp.clone(),
                    error_count: 1,
                    severity: severity.to_string(),
                    last_exposed_at: now_ms(),
                    resolved: false,
                    resolved_at: None,
                });
            }
        }

        total_score += score as i64;
        question_scores.push(QuestionScore {
            question_id: q.question_id.clone(),
            score,
            correctness,
            completeness,
            expression,
            depth,
            comment,
            weak_points,
        });
    }

    let avg_score = if !questions.is_empty() {
        (total_score / questions.len() as i64) as i32
    } else {
        0
    };
    let daoji = evaluate_daoji_from_score(avg_score);
    let overall_comment = parsed
        .get("overall_comment")
        .and_then(|v| v.as_str())
        .unwrap_or("AI 批改完成")
        .to_string();

    let grading = GradingResult {
        total_score: avg_score,
        daoji_level: daoji.as_str().to_string(),
        question_scores,
        overall_comment,
        weakness_analysis,
    };

    // D4.6 深化#5/#14：批改结果后校验（总分一致性 + 道基等级复核 + 评语质量）
    // 校验失败记 warn，但不阻塞返回（避免 AI 评分因小偏差导致整体降级）
    let grading_validation =
        crate::services::game_breakthrough_deepening::validate_grading_result(&grading);
    if !grading_validation.is_valid {
        tracing::warn!(
            "D4.6: 批改结果校验失败（仍返回批改结果）：errors={:?}",
            grading_validation.errors
        );
    }
    if !grading_validation.warnings.is_empty() {
        tracing::info!(
            "D4.6: 批改结果校验警告：warnings={:?}",
            grading_validation.warnings
        );
    }

    // D4.6 深化#13：批改置信度评估（极差过大或评语过短 → Low/Medium）
    let confidence = crate::services::game_breakthrough_deepening::evaluate_grading_confidence(&grading);
    match confidence {
        crate::services::game_breakthrough_deepening::GradingConfidence::Low => {
            tracing::warn!("D4.6: 批改置信度 Low（单题极差 > 50 或评语异常），建议人工复核");
        }
        crate::services::game_breakthrough_deepening::GradingConfidence::Medium => {
            tracing::info!("D4.6: 批改置信度 Medium");
        }
        crate::services::game_breakthrough_deepening::GradingConfidence::High => {
            tracing::debug!("D4.6: 批改置信度 High");
        }
    }

    Some(grading)
}

/// D4.1: 去除 LLM 输出可能的 ```json ... ``` 代码块包裹。
fn strip_json_codeblock(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("```json") {
        return rest.trim_start().trim_end_matches("```").trim().to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("```") {
        return rest.trim_start().trim_end_matches("```").trim().to_string();
    }
    trimmed.to_string()
}

// ============================================================================
// 8. Mock AI 实现（出题 / 批改）— 作为 AI 降级兜底路径
// ============================================================================

/// Mock 出题：按题型分布生成固定模板题目。
///
/// 题目内容为占位文本（`[Mock ...]` 前缀），仅保证题型/数量/难度符合规范。
/// 真实 AI 接入时由 `ai_generate_questions` 调用，失败时降级到此函数。
fn mock_generate_questions(
    target_realm: RealmMajor,
    choice_count: u8,
    short_count: u8,
    app_count: u8,
    difficulty: f64,
    weakness: &[WeaknessItem],
) -> Vec<BreakthroughQuestion> {
    let mut questions = Vec::new();
    let mut idx = 1u8;

    // 选择题（D4.5: 前 image_count 道升级为 image_choice 图片选择题）
    let img_count = image_question_count(target_realm).min(choice_count);
    for i in 0..choice_count {
        let is_image = i < img_count;
        let q_type = if is_image { "image_choice" } else { "choice" };
        let content = if is_image {
            format!(
                "[Mock 图片选择题 {} - 目标境界 {}] 观察下方示意图，以下哪项描述正确？",
                idx,
                target_realm.as_str()
            )
        } else {
            format!(
                "[Mock 选择题 {} - 目标境界 {}] 以下哪项描述正确？",
                idx,
                target_realm.as_str()
            )
        };
        questions.push(BreakthroughQuestion {
            question_id: format!("q{}", idx),
            question_type: q_type.to_string(),
            domain_id: "cs".to_string(),
            knowledge_point: "基础概念".to_string(),
            content,
            options: Some(vec![
                "A. 选项 A".into(),
                "B. 选项 B".into(),
                "C. 选项 C".into(),
                "D. 选项 D".into(),
            ]),
            standard_answer: "A".to_string(),
            difficulty,
            // D4.5 多模态：image_choice 题型携带媒体字段
            media_url: if is_image {
                Some(format!("placeholder://image/{}", idx))
            } else {
                None
            },
            media_type: if is_image { Some("image".to_string()) } else { None },
            media_description: if is_image {
                Some(format!(
                    "目标境界 {} 的知识点示意图：展示某核心结构的组成与关联",
                    target_realm.as_str()
                ))
            } else {
                None
            },
        });
        idx += 1;
    }

    // 简答题
    for _ in 0..short_count {
        questions.push(BreakthroughQuestion {
            question_id: format!("q{}", idx),
            question_type: "short_answer".to_string(),
            domain_id: "math".to_string(),
            knowledge_point: "核心概念".to_string(),
            content: format!(
                "[Mock 简答题 {} - 目标境界 {}] 简述某核心概念的关键要点（至少 3 点）。",
                idx,
                target_realm.as_str()
            ),
            options: None,
            standard_answer: "要点1；要点2；要点3".to_string(),
            difficulty,
            media_url: None,
            media_type: None,
            media_description: None,
        });
        idx += 1;
    }

    // 应用题（优先使用弱点知识点）
    for i in 0..app_count {
        let weak_point = weakness
            .get(i as usize)
            .map(|w| w.knowledge_point.as_str())
            .unwrap_or("综合应用");
        questions.push(BreakthroughQuestion {
            question_id: format!("q{}", idx),
            question_type: "application".to_string(),
            domain_id: weakness
                .get(i as usize)
                .map(|w| w.domain_id.clone())
                .unwrap_or_else(|| "engineering".to_string()),
            knowledge_point: weak_point.to_string(),
            content: format!(
                "[Mock 应用题 {} - 目标境界 {}] 在实际场景中应用「{}」知识解决问题，给出分析与方案。",
                idx,
                target_realm.as_str(),
                weak_point
            ),
            options: None,
            standard_answer: "分析；方案；实施；验证".to_string(),
            difficulty,
            media_url: None,
            media_type: None,
            media_description: None,
        });
        idx += 1;
    }

    questions
}

/// Mock 批改：
/// - 选择题：答案完全匹配 → 100 分，否则 0 分
/// - 简答/应用题：按用户答案要点数 / 标准答案要点数 × 70（最高 70 分）
///
/// 总分 = 全部题目得分的平均值（0-100）。
/// 弱点：每道错题（score < 60）暴露对应知识点，按分数评定严重度。
fn mock_grade_answers(
    questions: &[BreakthroughQuestion],
    answers: &[BreakthroughAnswer],
) -> GradingResult {
    let mut question_scores = Vec::new();
    let mut total_score = 0i64;
    let mut weakness_analysis = Vec::new();

    for q in questions {
        let ans = answers.iter().find(|a| a.question_id == q.question_id);
        let (score, weak_points) = match ans {
            Some(a) => {
                // Bug 修复 v1.52.19.1：image_understanding 是选择题变体，应走二值判定
                // （原逻辑只识别 choice/image_choice，导致 image_understanding 题答对只得 70 分）
                if q.question_type == "choice"
                    || q.question_type == "image_choice"
                    || q.question_type == "image_understanding"
                {
                    // 选择题 / 图片选择题 / 图片理解题：二值判定
                    if a.user_answer.trim().eq_ignore_ascii_case(&q.standard_answer) {
                        (100_i32, Vec::new())
                    } else {
                        (0_i32, vec![q.knowledge_point.clone()])
                    }
                } else {
                    // 简答/应用题：按要点覆盖率评分（上限 70）
                    let expected_count = q.standard_answer.split('；').count().max(1);
                    let user_count = a.user_answer.split('；').count().max(1);
                    let coverage = (user_count as f64 / expected_count as f64).min(1.0);
                    let score = (70.0 * coverage) as i32;
                    if score < 60 {
                        (score, vec![q.knowledge_point.clone()])
                    } else {
                        (score, Vec::new())
                    }
                }
            }
            None => (0, vec![q.knowledge_point.clone()]),
        };

        total_score += score as i64;

        // 弱点收集
        if !weak_points.is_empty() {
            let severity = if score <= 30 {
                "high"
            } else if score <= 60 {
                "medium"
            } else {
                "low"
            };
            for wp in &weak_points {
                weakness_analysis.push(WeaknessItem {
                    domain_id: q.domain_id.clone(),
                    knowledge_point: wp.clone(),
                    error_count: 1,
                    severity: severity.to_string(),
                    last_exposed_at: now_ms(),
                    resolved: false,
                    resolved_at: None,
                });
            }
        }

        question_scores.push(QuestionScore {
            question_id: q.question_id.clone(),
            score,
            correctness: 0,
            completeness: 0,
            expression: 0,
            depth: 0,
            comment: format!("Mock 评语：得分 {}（题号 {}）", score, q.question_id),
            weak_points,
        });
    }

    let avg_score = if !questions.is_empty() {
        (total_score / questions.len() as i64) as i32
    } else {
        0
    };
    let daoji = evaluate_daoji_from_score(avg_score);

    GradingResult {
        total_score: avg_score,
        daoji_level: daoji.as_str().to_string(),
        question_scores,
        overall_comment: format!(
            "Mock 批改：总分 {} 分，评定道基 {}。本次共暴露 {} 个弱点。",
            avg_score,
            daoji.as_str(),
            weakness_analysis.len()
        ),
        weakness_analysis,
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------- 道基评定 --------

    #[test]
    fn test_evaluate_daoji_from_score() {
        assert_eq!(evaluate_daoji_from_score(100), DaoFoundation::Black);
        assert_eq!(evaluate_daoji_from_score(95), DaoFoundation::Black);
        assert_eq!(evaluate_daoji_from_score(90), DaoFoundation::Black);
        assert_eq!(evaluate_daoji_from_score(89), DaoFoundation::Purple);
        assert_eq!(evaluate_daoji_from_score(75), DaoFoundation::Purple);
        assert_eq!(evaluate_daoji_from_score(74), DaoFoundation::Red);
        assert_eq!(evaluate_daoji_from_score(60), DaoFoundation::Red);
        assert_eq!(evaluate_daoji_from_score(59), DaoFoundation::Blue);
        assert_eq!(evaluate_daoji_from_score(45), DaoFoundation::Blue);
        assert_eq!(evaluate_daoji_from_score(44), DaoFoundation::White);
        assert_eq!(evaluate_daoji_from_score(0), DaoFoundation::White);
    }

    #[test]
    fn test_apply_daoji_floor_black_prev() {
        // 上阶黑 → 保底红
        assert_eq!(
            apply_daoji_floor(DaoFoundation::White, DaoFoundation::Black),
            DaoFoundation::Red
        );
        assert_eq!(
            apply_daoji_floor(DaoFoundation::Blue, DaoFoundation::Black),
            DaoFoundation::Red
        );
        assert_eq!(
            apply_daoji_floor(DaoFoundation::Red, DaoFoundation::Black),
            DaoFoundation::Red
        );
        // 紫和黑本身 ≥ 红，保持不变
        assert_eq!(
            apply_daoji_floor(DaoFoundation::Purple, DaoFoundation::Black),
            DaoFoundation::Purple
        );
        assert_eq!(
            apply_daoji_floor(DaoFoundation::Black, DaoFoundation::Black),
            DaoFoundation::Black
        );
    }

    #[test]
    fn test_apply_daoji_floor_purple_prev() {
        // 上阶紫 → 保底蓝
        assert_eq!(
            apply_daoji_floor(DaoFoundation::White, DaoFoundation::Purple),
            DaoFoundation::Blue
        );
        assert_eq!(
            apply_daoji_floor(DaoFoundation::Blue, DaoFoundation::Purple),
            DaoFoundation::Blue
        );
        // 红/紫/黑本身 ≥ 蓝，保持不变
        assert_eq!(
            apply_daoji_floor(DaoFoundation::Red, DaoFoundation::Purple),
            DaoFoundation::Red
        );
        assert_eq!(
            apply_daoji_floor(DaoFoundation::Purple, DaoFoundation::Purple),
            DaoFoundation::Purple
        );
    }

    #[test]
    fn test_apply_daoji_floor_no_floor() {
        // 上阶白/蓝/红无保底
        assert_eq!(
            apply_daoji_floor(DaoFoundation::White, DaoFoundation::Red),
            DaoFoundation::White
        );
        assert_eq!(
            apply_daoji_floor(DaoFoundation::White, DaoFoundation::Blue),
            DaoFoundation::White
        );
        assert_eq!(
            apply_daoji_floor(DaoFoundation::White, DaoFoundation::White),
            DaoFoundation::White
        );
    }

    #[test]
    fn test_evaluate_daoji_with_floor_success() {
        // 突破成功（≥60）：应用保底
        // 上阶黑，得分 60 → 红（保底红）
        assert_eq!(
            evaluate_daoji_with_floor(60, DaoFoundation::Black),
            DaoFoundation::Red
        );
        // 上阶紫，得分 60 → 红（红 ≥ 蓝，保持红）
        assert_eq!(
            evaluate_daoji_with_floor(60, DaoFoundation::Purple),
            DaoFoundation::Red
        );
    }

    #[test]
    fn test_evaluate_daoji_with_floor_failure() {
        // 突破失败（<60）：不应用保底，直接按分数评定
        assert_eq!(
            evaluate_daoji_with_floor(50, DaoFoundation::Black),
            DaoFoundation::Blue
        );
        assert_eq!(
            evaluate_daoji_with_floor(30, DaoFoundation::Purple),
            DaoFoundation::White
        );
    }

    // -------- 题型分布 --------

    #[test]
    fn test_question_type_distribution_all_realms() {
        assert_eq!(question_type_distribution(RealmMajor::Mortal), (0, 0, 0));
        assert_eq!(
            question_type_distribution(RealmMajor::QiRefining),
            (2, 1, 0)
        );
        assert_eq!(
            question_type_distribution(RealmMajor::FoundationBuilding),
            (2, 1, 1)
        );
        assert_eq!(
            question_type_distribution(RealmMajor::GoldenCore),
            (2, 2, 1)
        );
        assert_eq!(
            question_type_distribution(RealmMajor::NascentSoul),
            (2, 2, 2)
        );
        assert_eq!(
            question_type_distribution(RealmMajor::SpiritTransformation),
            (3, 3, 2)
        );
        assert_eq!(question_type_distribution(RealmMajor::Unity), (3, 4, 3));
        assert_eq!(question_type_distribution(RealmMajor::Mahayana), (4, 4, 4));
        assert_eq!(
            question_type_distribution(RealmMajor::Tribulation),
            (4, 5, 6)
        );
        assert_eq!(
            question_type_distribution(RealmMajor::Immortal),
            (0, 0, 15)
        );
    }

    #[test]
    fn test_question_type_distribution_sum_matches_question_count() {
        // 总题数应等于 RealmMajor::breakthrough_question_count
        // 注：Mortal/Immortal 为终态，breakthrough_question_count 返回 0，不参与突破流程
        for realm in [
            RealmMajor::QiRefining,
            RealmMajor::FoundationBuilding,
            RealmMajor::GoldenCore,
            RealmMajor::NascentSoul,
            RealmMajor::SpiritTransformation,
            RealmMajor::Unity,
            RealmMajor::Mahayana,
            RealmMajor::Tribulation,
        ] {
            let (c, s, a) = question_type_distribution(realm);
            let total = c + s + a;
            assert_eq!(
                total,
                realm.breakthrough_question_count(),
                "境界 {:?} 题数不匹配：{} vs {}",
                realm,
                total,
                realm.breakthrough_question_count()
            );
        }
    }

    // -------- 难度系数 --------

    #[test]
    fn test_difficulty_coefficient() {
        // 难度系数 = 0.5 + 序号 × 0.1，序号从 1 开始（Mortal=1）
        // 注：浮点累加存在精度误差，用容差比较（abs < 1e-9）
        let approx_eq = |a: f64, b: f64| (a - b).abs() < 1e-9;
        assert!(approx_eq(difficulty_coefficient(RealmMajor::Mortal), 0.6));
        assert!(approx_eq(difficulty_coefficient(RealmMajor::QiRefining), 0.7));
        assert!(approx_eq(
            difficulty_coefficient(RealmMajor::FoundationBuilding),
            0.8
        ));
        assert!(approx_eq(difficulty_coefficient(RealmMajor::GoldenCore), 0.9));
        assert!(approx_eq(difficulty_coefficient(RealmMajor::NascentSoul), 1.0));
        assert!(approx_eq(
            difficulty_coefficient(RealmMajor::SpiritTransformation),
            1.1
        ));
        assert!(approx_eq(difficulty_coefficient(RealmMajor::Unity), 1.2));
        assert!(approx_eq(difficulty_coefficient(RealmMajor::Mahayana), 1.3));
        assert!(approx_eq(difficulty_coefficient(RealmMajor::Tribulation), 1.4));
        assert!(approx_eq(difficulty_coefficient(RealmMajor::Immortal), 1.5));
    }

    // -------- 前一阶大境界 --------

    #[test]
    fn test_previous_realm() {
        assert_eq!(previous_realm(RealmMajor::Mortal), None);
        assert_eq!(
            previous_realm(RealmMajor::QiRefining),
            Some(RealmMajor::Mortal)
        );
        assert_eq!(
            previous_realm(RealmMajor::FoundationBuilding),
            Some(RealmMajor::QiRefining)
        );
        assert_eq!(
            previous_realm(RealmMajor::Immortal),
            Some(RealmMajor::Tribulation)
        );
    }

    // -------- 弱点合并 --------

    #[test]
    fn test_merge_weakness_empty_prev() {
        let prev = vec![];
        let exposed = vec![WeaknessItem {
            domain_id: "cs".to_string(),
            knowledge_point: "智能指针".to_string(),
            error_count: 1,
            severity: "medium".to_string(),
            last_exposed_at: 1000,
            resolved: false,
            resolved_at: None,
        }];
        let result = merge_weakness(&prev, &exposed);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].knowledge_point, "智能指针");
        assert_eq!(result[0].error_count, 1);
    }

    #[test]
    fn test_merge_weakness_existing() {
        let prev = vec![WeaknessItem {
            domain_id: "cs".to_string(),
            knowledge_point: "智能指针".to_string(),
            error_count: 1,
            severity: "low".to_string(),
            last_exposed_at: 1000,
            resolved: false,
            resolved_at: None,
        }];
        let exposed = vec![WeaknessItem {
            domain_id: "cs".to_string(),
            knowledge_point: "智能指针".to_string(),
            error_count: 1,
            severity: "high".to_string(),
            last_exposed_at: 2000,
            resolved: false,
            resolved_at: None,
        }];
        let result = merge_weakness(&prev, &exposed);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].error_count, 2);
        assert_eq!(result[0].severity, "high");
        assert_eq!(result[0].last_exposed_at, 2000);
    }

    #[test]
    fn test_merge_weakness_skip_resolved() {
        let prev = vec![WeaknessItem {
            domain_id: "cs".to_string(),
            knowledge_point: "已解决".to_string(),
            error_count: 1,
            severity: "low".to_string(),
            last_exposed_at: 1000,
            resolved: true,
            resolved_at: Some(1500),
        }];
        let exposed = vec![];
        let result = merge_weakness(&prev, &exposed);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_merge_weakness_truncate_to_20() {
        let prev: Vec<WeaknessItem> = (0..25)
            .map(|i| WeaknessItem {
                domain_id: format!("d{}", i),
                knowledge_point: format!("k{}", i),
                error_count: 1,
                severity: "low".to_string(),
                last_exposed_at: i,
                resolved: false,
                resolved_at: None,
            })
            .collect();
        let exposed = vec![];
        let result = merge_weakness(&prev, &exposed);
        assert_eq!(result.len(), 20);
        // 保留 last_exposed_at 最大的 20 条（24, 23, ..., 5）
        assert_eq!(result[0].last_exposed_at, 24);
        assert_eq!(result[19].last_exposed_at, 5);
    }

    #[test]
    fn test_severity_max() {
        assert_eq!(severity_max("high", "medium"), "high");
        assert_eq!(severity_max("low", "medium"), "medium");
        assert_eq!(severity_max("high", "high"), "high");
        assert_eq!(severity_max("low", "low"), "low");
        assert_eq!(severity_max("medium", "high"), "high");
    }

    // -------- Mock AI 出题 --------

    #[test]
    fn test_mock_generate_questions_count() {
        let questions = mock_generate_questions(
            RealmMajor::FoundationBuilding,
            2,  // choice
            1,  // short
            1,  // app
            0.8,
            &[],
        );
        assert_eq!(questions.len(), 4);
        assert_eq!(
            questions.iter().filter(|q| q.question_type == "choice").count(),
            2
        );
        assert_eq!(
            questions
                .iter()
                .filter(|q| q.question_type == "short_answer")
                .count(),
            1
        );
        assert_eq!(
            questions
                .iter()
                .filter(|q| q.question_type == "application")
                .count(),
            1
        );
        // 题号递增
        assert_eq!(questions[0].question_id, "q1");
        assert_eq!(questions[3].question_id, "q4");
    }

    #[test]
    fn test_mock_generate_questions_with_weakness() {
        let weakness = vec![WeaknessItem {
            domain_id: "rust".to_string(),
            knowledge_point: "生命周期".to_string(),
            error_count: 2,
            severity: "high".to_string(),
            last_exposed_at: 1000,
            resolved: false,
            resolved_at: None,
        }];
        let questions = mock_generate_questions(
            RealmMajor::GoldenCore,
            2,
            2,
            1, // 1 应用题
            0.9,
            &weakness,
        );
        // 应用题应使用弱点知识点
        let app_q = questions
            .iter()
            .find(|q| q.question_type == "application")
            .unwrap();
        assert_eq!(app_q.knowledge_point, "生命周期");
        assert_eq!(app_q.domain_id, "rust");
    }

    // -------- Mock AI 批改 --------

    #[test]
    fn test_mock_grade_answers_choice_perfect() {
        let questions = vec![
            BreakthroughQuestion {
                question_id: "q1".to_string(),
                question_type: "choice".to_string(),
                domain_id: "cs".to_string(),
                knowledge_point: "测试1".to_string(),
                content: "题干1".to_string(),
                options: Some(vec!["A".into(), "B".into(), "C".into(), "D".into()]),
                standard_answer: "A".to_string(),
                difficulty: 0.7,
                media_url: None,
                media_type: None,
                media_description: None,
            },
            BreakthroughQuestion {
                question_id: "q2".to_string(),
                question_type: "choice".to_string(),
                domain_id: "cs".to_string(),
                knowledge_point: "测试2".to_string(),
                content: "题干2".to_string(),
                options: Some(vec!["A".into(), "B".into(), "C".into(), "D".into()]),
                standard_answer: "B".to_string(),
                difficulty: 0.7,
                media_url: None,
                media_type: None,
                media_description: None,
            },
        ];
        let answers = vec![
            BreakthroughAnswer {
                question_id: "q1".to_string(),
                user_answer: "A".to_string(),
            },
            BreakthroughAnswer {
                question_id: "q2".to_string(),
                user_answer: "B".to_string(),
            },
        ];
        let result = mock_grade_answers(&questions, &answers);
        assert_eq!(result.total_score, 100);
        assert_eq!(result.daoji_level, "black");
        assert!(result.weakness_analysis.is_empty());
    }

    #[test]
    fn test_mock_grade_answers_choice_wrong() {
        let questions = vec![BreakthroughQuestion {
            question_id: "q1".to_string(),
            question_type: "choice".to_string(),
            domain_id: "cs".to_string(),
            knowledge_point: "所有权".to_string(),
            content: "题干".to_string(),
            options: Some(vec!["A".into(), "B".into(), "C".into(), "D".into()]),
            standard_answer: "A".to_string(),
            difficulty: 0.7,
            media_url: None,
            media_type: None,
            media_description: None,
        }];
        let answers = vec![BreakthroughAnswer {
            question_id: "q1".to_string(),
            user_answer: "B".to_string(),
        }];
        let result = mock_grade_answers(&questions, &answers);
        assert_eq!(result.total_score, 0);
        assert_eq!(result.daoji_level, "white");
        assert_eq!(result.weakness_analysis.len(), 1);
        assert_eq!(result.weakness_analysis[0].knowledge_point, "所有权");
        assert_eq!(result.weakness_analysis[0].severity, "high");
    }

    #[test]
    fn test_mock_grade_answers_short_partial() {
        // 简答题：3 个标准要点，用户答 2 个 → 70 × (2/3) ≈ 46 分 → 蓝
        let questions = vec![BreakthroughQuestion {
            question_id: "q1".to_string(),
            question_type: "short_answer".to_string(),
            domain_id: "math".to_string(),
            knowledge_point: "导数".to_string(),
            content: "题干".to_string(),
            options: None,
            standard_answer: "要点1；要点2；要点3".to_string(),
            difficulty: 0.8,
            media_url: None,
            media_type: None,
            media_description: None,
        }];
        let answers = vec![BreakthroughAnswer {
            question_id: "q1".to_string(),
            user_answer: "答案1；答案2".to_string(),
        }];
        let result = mock_grade_answers(&questions, &answers);
        // 70 × (2/3) = 46（i32 截断）
        assert_eq!(result.total_score, 46);
        assert_eq!(result.daoji_level, "blue");
        // 46 < 60 → 暴露弱点
        assert_eq!(result.weakness_analysis.len(), 1);
        assert_eq!(result.weakness_analysis[0].severity, "medium");
    }

    #[test]
    fn test_mock_grade_answers_missing_answer() {
        let questions = vec![BreakthroughQuestion {
            question_id: "q1".to_string(),
            question_type: "choice".to_string(),
            domain_id: "cs".to_string(),
            knowledge_point: "测试".to_string(),
            content: "题干".to_string(),
            options: Some(vec!["A".into(), "B".into(), "C".into(), "D".into()]),
            standard_answer: "A".to_string(),
            difficulty: 0.7,
            media_url: None,
            media_type: None,
            media_description: None,
        }];
        let answers: Vec<BreakthroughAnswer> = vec![];
        let result = mock_grade_answers(&questions, &answers);
        assert_eq!(result.total_score, 0);
        assert_eq!(result.daoji_level, "white");
        assert_eq!(result.weakness_analysis.len(), 1);
    }

    #[test]
    fn test_mock_grade_answers_empty_questions() {
        let questions: Vec<BreakthroughQuestion> = vec![];
        let answers: Vec<BreakthroughAnswer> = vec![];
        let result = mock_grade_answers(&questions, &answers);
        assert_eq!(result.total_score, 0);
        assert!(result.question_scores.is_empty());
        assert!(result.weakness_analysis.is_empty());
    }

    // -------- D4.5 多模态图片选择题 --------

    #[test]
    fn test_image_question_count_per_realm() {
        // 低境界（凡人~元婴）无图片题
        assert_eq!(image_question_count(RealmMajor::Mortal), 0);
        assert_eq!(image_question_count(RealmMajor::QiRefining), 0);
        assert_eq!(image_question_count(RealmMajor::FoundationBuilding), 0);
        assert_eq!(image_question_count(RealmMajor::GoldenCore), 0);
        assert_eq!(image_question_count(RealmMajor::NascentSoul), 0);
        // 化神 / 合体：1 道
        assert_eq!(image_question_count(RealmMajor::SpiritTransformation), 1);
        assert_eq!(image_question_count(RealmMajor::Unity), 1);
        // 大乘 / 渡劫 / 仙：2 道
        assert_eq!(image_question_count(RealmMajor::Mahayana), 2);
        assert_eq!(image_question_count(RealmMajor::Tribulation), 2);
        assert_eq!(image_question_count(RealmMajor::Immortal), 2);
    }

    #[test]
    fn test_mock_generate_questions_image_choice_spirit_transformation() {
        // 化神境界：choice_count=3，image_question_count=1
        // 预期：1 道 image_choice + 2 道 choice + 3 道 short_answer + 2 道 application = 8 道
        let questions = mock_generate_questions(
            RealmMajor::SpiritTransformation,
            3, // choice
            3, // short
            2, // app
            1.0,
            &[],
        );
        assert_eq!(questions.len(), 8);

        let image_count = questions
            .iter()
            .filter(|q| q.question_type == "image_choice")
            .count();
        let choice_count = questions
            .iter()
            .filter(|q| q.question_type == "choice")
            .count();
        assert_eq!(image_count, 1, "化神境界应有 1 道 image_choice");
        assert_eq!(choice_count, 2, "剩余 2 道为纯文本 choice");

        // image_choice 题应携带媒体字段
        let img_q = questions
            .iter()
            .find(|q| q.question_type == "image_choice")
            .expect("应存在 image_choice 题目");
        assert!(img_q.media_url.is_some(), "media_url 必填");
        assert_eq!(img_q.media_type.as_deref(), Some("image"));
        assert!(img_q.media_description.is_some(), "media_description 必填");

        // 纯文本 choice 题不应携带媒体字段
        let text_q = questions
            .iter()
            .find(|q| q.question_type == "choice")
            .expect("应存在 choice 题目");
        assert!(text_q.media_url.is_none());
        assert!(text_q.media_type.is_none());
        assert!(text_q.media_description.is_none());
    }

    #[test]
    fn test_mock_generate_questions_image_choice_mahayana() {
        // 大乘境界：image_question_count=2，choice_count=4
        // 预期：2 道 image_choice + 2 道 choice
        let questions = mock_generate_questions(
            RealmMajor::Mahayana,
            4, // choice
            4, // short
            4, // app
            1.5,
            &[],
        );
        let image_count = questions
            .iter()
            .filter(|q| q.question_type == "image_choice")
            .count();
        assert_eq!(image_count, 2, "大乘境界应有 2 道 image_choice");

        // image_choice 题号应为 q1 / q2（前 image_count 道）
        assert_eq!(questions[0].question_type, "image_choice");
        assert_eq!(questions[0].question_id, "q1");
        assert_eq!(questions[1].question_type, "image_choice");
        assert_eq!(questions[1].question_id, "q2");
        // 第 3 道起恢复为 choice
        assert_eq!(questions[2].question_type, "choice");
    }

    #[test]
    fn test_mock_grade_answers_image_choice_correct() {
        // image_choice 答对 → 100 分，无弱点
        let questions = vec![BreakthroughQuestion {
            question_id: "q1".to_string(),
            question_type: "image_choice".to_string(),
            domain_id: "cs".to_string(),
            knowledge_point: "图结构识别".to_string(),
            content: "观察示意图，以下哪项描述正确？".to_string(),
            options: Some(vec!["A".into(), "B".into(), "C".into(), "D".into()]),
            standard_answer: "A".to_string(),
            difficulty: 1.0,
            media_url: Some("placeholder://image/1".to_string()),
            media_type: Some("image".to_string()),
            media_description: Some("知识点示意图".to_string()),
        }];
        let answers = vec![BreakthroughAnswer {
            question_id: "q1".to_string(),
            user_answer: "a".to_string(), // 大小写不敏感
        }];
        let result = mock_grade_answers(&questions, &answers);
        assert_eq!(result.total_score, 100);
        assert_eq!(result.daoji_level, "black");
        assert!(result.weakness_analysis.is_empty());
    }

    #[test]
    fn test_mock_grade_answers_image_choice_wrong() {
        // image_choice 答错 → 0 分 + 暴露弱点
        let questions = vec![BreakthroughQuestion {
            question_id: "q1".to_string(),
            question_type: "image_choice".to_string(),
            domain_id: "cs".to_string(),
            knowledge_point: "图结构识别".to_string(),
            content: "观察示意图，以下哪项描述正确？".to_string(),
            options: Some(vec!["A".into(), "B".into(), "C".into(), "D".into()]),
            standard_answer: "A".to_string(),
            difficulty: 1.0,
            media_url: Some("placeholder://image/1".to_string()),
            media_type: Some("image".to_string()),
            media_description: Some("知识点示意图".to_string()),
        }];
        let answers = vec![BreakthroughAnswer {
            question_id: "q1".to_string(),
            user_answer: "B".to_string(),
        }];
        let result = mock_grade_answers(&questions, &answers);
        assert_eq!(result.total_score, 0);
        assert_eq!(result.daoji_level, "white");
        assert_eq!(result.weakness_analysis.len(), 1);
        assert_eq!(result.weakness_analysis[0].knowledge_point, "图结构识别");
        assert_eq!(result.weakness_analysis[0].severity, "high");
    }
}
