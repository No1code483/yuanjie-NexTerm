//! AI 考验机制深化模块（文档12 §6 模块清单）
//!
//! 实现考验机制的 15 项深化能力，作为 game_breakthrough_service 的扩展模块。
//! change-id: game-3d-rebuild-refactor
//!
//! 模块清单：
//! - MaterialExtractor 素材提取器（#1）
//! - QuestionDedup 题目去重（#2）
//! - QuestionValidator 题目校验器（#3/#4/#6）
//! - GradingValidator 批改校验器（#5/#14）
//! - HeavenlyExaminerPrompt 完整天道考官人设（#10）
//! - PersonalizedRecommender 个性化推荐器（#11）
//! - ImageGenPlaceholder 文生图占位（#12）
//! - GradingConsistency 批改一致性校验（#13）
//! - MultimodalUnderstanding 多模态理解占位（#15）
//!
//! 强制约束（项目核心设计意图 §五/§八）：
//! - 所有 AI 调用必须走云端 API（AiModelService），不接入本地底层智能模型
//! - 底层智能关闭后，本模块核心功能（校验/去重/素材提取）必须可用
//! - AI 失败时降级到 mock，不阻塞主流程

use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::services::ai_model_service::AiModelService;
use crate::services::game_breakthrough_service::{
    BreakthroughQuestion, GradingResult, WeaknessItem, evaluate_daoji_from_score,
};

// ============================================================================
// 1. 素材提取器（#1）— 从 KB + 领域映射 + 弱点历史提取出题素材
// ============================================================================

/// 单条 KB 素材摘要。
#[derive(Debug, Clone, serde::Serialize)]
pub struct KbMaterial {
    pub entry_id: String,
    pub title: String,
    /// 内容摘要（前 200 字）。
    pub content_excerpt: String,
    pub domain_id: String,
    pub category_name: String,
}

/// 突破出题素材上下文。
#[derive(Debug, Clone, serde::Serialize)]
pub struct BreakthroughContext {
    /// KB 素材列表。
    pub materials: Vec<KbMaterial>,
    /// 历史弱点列表（原样透传）。
    pub weakness: Vec<WeaknessItem>,
    /// 近 3 次本境界题目去重列表（知识点 + 题型）。
    pub recent_questions: Vec<RecentQuestionKey>,
    /// 各领域掌握情况（domain_id → level）。
    pub domain_levels: Vec<(String, i32)>,
}

/// 去重用题目键。
#[derive(Debug, Clone, serde::Serialize)]
pub struct RecentQuestionKey {
    pub knowledge_point: String,
    pub question_type: String,
}

/// 从 KB + 领域映射 + 弱点 + 历史记录提取出题素材。
///
/// 失败时返回空上下文（不阻塞出题，降级到无素材出题）。
///
/// 多用户隔离批次 3：新增 user_id 参数，KB 素材按用户隔离过滤。
pub async fn extract_breakthrough_materials(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    weakness: &[WeaknessItem],
) -> Result<BreakthroughContext, AppError> {
    // 1. 查询近 3 次本境界突破记录的 questions_json（字段为 Option<String>）
    let recent_records = game_repo::list_breakthrough_records(pool, world_id, 3).await?;
    let mut recent_questions: Vec<RecentQuestionKey> = Vec::new();
    for record in &recent_records {
        if let Some(json) = &record.questions_json {
            if let Ok(qs) = serde_json::from_str::<Vec<BreakthroughQuestion>>(json) {
                for q in qs {
                    recent_questions.push(RecentQuestionKey {
                        knowledge_point: q.knowledge_point,
                        question_type: q.question_type,
                    });
                }
            }
        }
    }

    // 2. 查询世界各领域掌握等级
    let progress = game_repo::list_progress_by_world(pool, world_id).await?;
    let domain_levels: Vec<(String, i32)> = progress
        .iter()
        .map(|p| (p.domain_id.clone(), p.level))
        .collect();

    // 3. 查询 KB 素材（复用 kb_repo::get_all_entries，按 updated_at 倒序取前 N 条）
    //    多用户隔离批次 3：传 user_id 仅取当前用户的 KB 条目
    let materials = load_kb_materials(pool, user_id, 20).await.unwrap_or_default();

    Ok(BreakthroughContext {
        materials,
        weakness: weakness.to_vec(),
        recent_questions,
        domain_levels,
    })
}

/// 加载 KB 素材（复用 kb_repo::get_all_entries，取前 N 条）。
///
/// 完整实现需 JOIN game_kb_category_mapping 反查领域；此处先做基础提取，
/// 后续可扩展为按领域过滤 + 关联映射。
///
/// 多用户隔离批次 3：新增 user_id 参数，仅加载当前用户的 KB 条目。
async fn load_kb_materials(pool: &SqlitePool, user_id: i64, limit: usize) -> Result<Vec<KbMaterial>, AppError> {
    let entries = kb_repo::get_all_entries(pool, user_id).await?;
    let materials = entries
        .into_iter()
        .take(limit)
        .map(|e| KbMaterial {
            entry_id: e.id.to_string(),
            title: e.name,
            content_excerpt: e.content.unwrap_or_default().chars().take(200).collect(),
            domain_id: "general".to_string(), // 后续按 category_id 反查领域映射
            category_name: e.category_id.to_string(),
        })
        .collect();
    Ok(materials)
}

// ============================================================================
// 2. 题目校验器（#3/#4/#6）— 校验 AI 返回题目是否符合规范
// ============================================================================

/// 校验结果。
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// 校验 AI 生成的题目集合。
///
/// 校验项：
/// - 数量 = question_count
/// - 题型分布符合预期（choice/short_answer/application/image_choice）
/// - 每个领域至少 1 题（领域覆盖强制校验 #3）
/// - high/medium 弱点是否实际出现（弱点强化 #4）
/// - question_id 连续（q1..qN）
/// - 选择题 options 为 4 个字符串（题目格式 #6）
/// - 不在去重列表中（题目去重 #2 后校验）
pub fn validate_generated_questions(
    questions: &[BreakthroughQuestion],
    expected_count: u8,
    expected_choice: u8,
    expected_short: u8,
    expected_app: u8,
    weakness: &[WeaknessItem],
    recent_questions: &[RecentQuestionKey],
) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // 1. 数量校验
    if questions.len() != expected_count as usize {
        errors.push(format!(
            "题目数量不匹配：期望 {} 道，实际 {} 道",
            expected_count,
            questions.len()
        ));
    }

    // 2. 题型分布校验
    // 注意：image_understanding 题型是选择题变体（携带 4 选项），但作为多模态追加题不计入
    // expected_choice（expected_choice 仅约束常规 choice/image_choice 数量）。
    // 这里统计 actual_choice 时也不含 image_understanding，确保与 expected_choice 口径一致。
    let actual_choice = questions
        .iter()
        .filter(|q| q.question_type == "choice" || q.question_type == "image_choice")
        .count() as u8;
    let actual_short = questions
        .iter()
        .filter(|q| q.question_type == "short_answer")
        .count() as u8;
    let actual_app = questions
        .iter()
        .filter(|q| q.question_type == "application")
        .count() as u8;

    if actual_choice != expected_choice {
        warnings.push(format!(
            "选择题数量不匹配：期望 {} 道，实际 {} 道",
            expected_choice, actual_choice
        ));
    }
    if actual_short != expected_short {
        warnings.push(format!(
            "简答题数量不匹配：期望 {} 道，实际 {} 道",
            expected_short, actual_short
        ));
    }
    if actual_app != expected_app {
        warnings.push(format!(
            "应用题数量不匹配：期望 {} 道，实际 {} 道",
            expected_app, actual_app
        ));
    }

    // 3. question_id 连续性校验
    // image_understanding 题型使用 q_img_<uuid> 格式（多模态追加题），不参与 q{idx} 连续性校验
    // Bug 修复 v1.52.19.1：原逻辑对 image_understanding 题必然失败（question_id 为 q_img_xxx 而非 q{N}）
    for (idx, q) in questions.iter().enumerate() {
        if q.question_type == "image_understanding" {
            continue;
        }
        let expected_id = format!("q{}", idx + 1);
        if q.question_id != expected_id {
            errors.push(format!(
                "question_id 不连续：第 {} 题应为 {}，实际为 {}",
                idx + 1,
                expected_id,
                q.question_id
            ));
        }
    }

    // 4. 选择题 options 校验
    // image_understanding 是选择题变体（携带 4 选项），纳入 options 校验
    for q in questions {
        if q.question_type == "choice"
            || q.question_type == "image_choice"
            || q.question_type == "image_understanding"
        {
            match &q.options {
                Some(opts) if opts.len() == 4 => {
                    if opts.iter().any(|o| o.trim().is_empty()) {
                        warnings.push(format!(
                            "题 {} 选项存在空字符串",
                            q.question_id
                        ));
                    }
                }
                Some(opts) => {
                    warnings.push(format!(
                        "题 {} 选择题选项应为 4 个，实际 {} 个",
                        q.question_id,
                        opts.len()
                    ));
                }
                None => {
                    errors.push(format!(
                        "题 {} 选择题缺少 options 字段",
                        q.question_id
                    ));
                }
            }
        } else {
            // 简答/应用题不应有 options
            if q.options.is_some() {
                warnings.push(format!(
                    "题 {} 非选择题不应携带 options",
                    q.question_id
                ));
            }
        }
    }

    // 5. 领域覆盖校验（每个有掌握等级的领域至少 1 题，若题目数 ≥ 领域数）
    let covered_domains: std::collections::HashSet<&str> =
        questions.iter().map(|q| q.domain_id.as_str()).collect();
    if covered_domains.len() < questions.len().min(3) {
        warnings.push(format!(
            "领域覆盖不足：仅覆盖 {} 个领域（建议至少 {} 个）",
            covered_domains.len(),
            questions.len().min(3)
        ));
    }

    // 6. 弱点强化校验（high 严重度弱点必须出现 ≥ 1 题）
    let high_weakness: Vec<&WeaknessItem> = weakness
        .iter()
        .filter(|w| w.severity == "high" && !w.resolved)
        .collect();
    let medium_weakness: Vec<&WeaknessItem> = weakness
        .iter()
        .filter(|w| w.severity == "medium" && !w.resolved)
        .collect();

    if !high_weakness.is_empty() {
        let high_covered = high_weakness.iter().any(|w| {
            questions
                .iter()
                .any(|q| q.knowledge_point.contains(&w.knowledge_point))
        });
        if !high_covered {
            warnings.push(format!(
                "high 严重度弱点未实际出现：建议至少 1 题考察「{}」",
                high_weakness
                    .iter()
                    .map(|w| w.knowledge_point.as_str())
                    .collect::<Vec<_>>()
                    .join("、")
            ));
        }
    }
    if !medium_weakness.is_empty() {
        let medium_covered = medium_weakness.iter().any(|w| {
            questions
                .iter()
                .any(|q| q.knowledge_point.contains(&w.knowledge_point))
        });
        if !medium_covered {
            warnings.push(format!(
                "medium 严重度弱点未实际出现：建议至少 1 题考察「{}」",
                medium_weakness
                    .iter()
                    .map(|w| w.knowledge_point.as_str())
                    .collect::<Vec<_>>()
                    .join("、")
            ));
        }
    }

    // 7. 去重校验（不在近 3 次题目列表中）
    let mut dedup_hits = Vec::new();
    for q in questions {
        for rq in recent_questions {
            if q.knowledge_point == rq.knowledge_point && q.question_type == rq.question_type {
                dedup_hits.push(format!(
                    "题 {}（知识点「{}」+ 题型「{}」）与近期题目重复",
                    q.question_id, q.knowledge_point, q.question_type
                ));
                break;
            }
        }
    }
    if !dedup_hits.is_empty() {
        warnings.push(format!("去重校验未通过：{}", dedup_hits.join("；")));
    }

    ValidationResult {
        is_valid: errors.is_empty(),
        errors,
        warnings,
    }
}

// ============================================================================
// 3. 批改校验器（#5/#14）— 校验 AI 批改结果
// ============================================================================

/// 校验 AI 批改结果。
///
/// 校验项：
/// - total_score 与 question_scores 平均值一致（容差 ±1，#5）
/// - daoji_level 符合分数区间映射（#5 道基等级复核）
/// - overall_comment 长度 100-300 字（#14）
/// - overall_comment 含至少 1 个 weakness_analysis 中的知识点名称（#14）
pub fn validate_grading_result(grading: &GradingResult) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // 1. 总分一致性校验（容差 ±1）
    if !grading.question_scores.is_empty() {
        let avg = (grading.question_scores.iter().map(|q| q.score as i64).sum::<i64>()
            / grading.question_scores.len() as i64) as i32;
        let diff = (grading.total_score - avg).abs();
        if diff > 1 {
            errors.push(format!(
                "总分与单题均值不一致：total_score={}, 平均={}(差值 {}>1)",
                grading.total_score, avg, diff
            ));
        }
    }

    // 2. 道基等级复核
    let expected_daoji = evaluate_daoji_from_score(grading.total_score).as_str();
    if grading.daoji_level != expected_daoji {
        errors.push(format!(
            "道基等级与分数不匹配：score={} 应为 {}，实际为 {}",
            grading.total_score, expected_daoji, grading.daoji_level
        ));
    }

    // 3. 评语长度校验（100-300 字）
    let comment_chars = grading.overall_comment.chars().count();
    if comment_chars < 100 {
        warnings.push(format!(
            "评语过短：{} 字（建议 100-300 字）",
            comment_chars
        ));
    } else if comment_chars > 300 {
        warnings.push(format!(
            "评语过长：{} 字（建议 100-300 字）",
            comment_chars
        ));
    }

    // 4. 评语含知识点引用校验
    if !grading.weakness_analysis.is_empty() {
        let has_kp_ref = grading.weakness_analysis.iter().any(|w| {
            grading.overall_comment.contains(&w.knowledge_point)
        });
        if !has_kp_ref {
            warnings.push("评语未引用任何弱点知识点名称（建议具体到知识点）".to_string());
        }
    }

    ValidationResult {
        is_valid: errors.is_empty(),
        errors,
        warnings,
    }
}

// ============================================================================
// 4. 完整天道考官人设 Prompt（#10）
// ============================================================================

/// 生成完整天道考官 system prompt。
///
/// 按 12_AI考验机制.md §3.1 实现：
/// - 角色：万年严苛考官
/// - 8 条行为准则
/// - 输出纪律（纯 JSON 输出）
/// - 世界观语境（修仙突破考验）
pub fn build_heavenly_examiner_system_prompt() -> &'static str {
    "你是天道考官，修仙世界万年以来负责裁定修士突破资格的严苛存在。\n\
     \n\
     ## 角色设定\n\
     你存在的意义是确保每一位修士的境界突破都建立在真实修行之上，杜绝虚假突破。\n\
     你不近人情，但绝对公正；你不为难题，只为检验真才实学。\n\
     \n\
     ## 行为准则（8 条）\n\
     1. 题目必须紧扣玩家实际学习的知识领域，绝不超纲\n\
     2. 每个知识领域至少 1 道题，确保覆盖面\n\
     3. high 严重度弱点必须出现 ≥ 1 题（优先应用题）\n\
     4. medium 严重度弱点必须出现 ≥ 1 题\n\
     5. 题目不可与近 3 次本境界突破考验重复\n\
     6. 难度系数严格遵循指定值（0.5 简单 ~ 1.5 困难）\n\
     7. 选择题必须 4 选项（A/B/C/D），简答/应用题不带 options\n\
     8. standard_answer 必须可批改（选择题填字母，简答/应用题用「；」分隔要点）\n\
     \n\
     ## 输出纪律\n\
     输出必须是合法 JSON 数组，不得包含任何解释性文字、markdown 代码块标记或前后缀。\n\
     每个元素格式严格遵守调用方指定的字段结构。\n\
     \n\
     ## 世界观语境\n\
     玩家即将突破到下一阶大境界，考验题目应贴合该境界的修行内涵：\n\
     - 低境界（凡人~元婴）：夯实基础，注重概念与简单应用\n\
     - 中境界（化神~合体）：考察综合应用与跨领域联想\n\
     - 高境界（大乘~仙）：考察深度思辨与系统性理解\n\
     \n\
     你只需输出 JSON，无需任何额外说明。"
}

/// 生成完整天道考官批改 system prompt。
pub fn build_heavenly_grader_system_prompt() -> &'static str {
    "你是天道考官，修仙世界万年以来负责裁定修士突破资格的严苛存在。\n\
     \n\
     ## 角色设定\n\
     你正在批改修士的突破考验答卷。你不近人情，但绝对公正。\n\
     每一分都关乎修士的修行前途，你必须严谨、客观、有理有据。\n\
     \n\
     ## 评分维度（4 维度，每项 0-100）\n\
     - correctness 正确性：答案是否正确\n\
     - completeness 完整性：要点是否覆盖完整\n\
     - expression 表达：表述是否清晰准确\n\
     - depth 深度：是否体现深入理解\n\
     \n\
     ## 评语硬性要求\n\
     - overall_comment 长度 100-300 字\n\
     - 必须包含至少 1 个暴露弱点的知识点名称\n\
     - 必须给出具体的行动建议（如\"建议复习 XXX\"）\n\
     - 禁止空泛评语（如\"还需努力\"\"有待提高\"）\n\
     \n\
     ## 弱点识别\n\
     - score < 60 的题目必须收集 weak_points\n\
     - weak_points 填入具体知识点名称\n\
     - score ≤ 30 → high 严重度；31-60 → medium；其余不收集\n\
     \n\
     ## 输出纪律\n\
     输出必须是合法 JSON 对象，不得包含任何解释性文字或 markdown 代码块标记。"
}

// ============================================================================
// 5. 个性化推荐器（#11）— 突破失败后生成复习路径
// ============================================================================

/// 复习建议项。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReviewSuggestion {
    pub domain_id: String,
    pub knowledge_point: String,
    pub reason: String,
    pub suggested_action: String,
    /// 关联的 KB 条目 ID（可选）。
    pub related_entry_id: Option<String>,
}

/// 突破失败后调用云端 API 生成个性化复习建议。
///
/// 输入：弱点列表 + 各领域掌握情况 + 可复习 KB 条目。
/// 输出：3-5 条具体复习建议。
/// 失败时返回空列表（不阻塞结果展示）。
pub async fn ai_generate_review_suggestions(
    ctx: &crate::services::game_breakthrough_service::AiContext,
    weakness: &[WeaknessItem],
    domain_levels: &[(String, i32)],
    materials: &[KbMaterial],
) -> Option<Vec<ReviewSuggestion>> {
    if weakness.is_empty() {
        return Some(Vec::new());
    }

    let weakness_str = weakness
        .iter()
        .map(|w| {
            format!(
                "- 领域 {} 知识点「{}」严重度 {} 出错 {} 次",
                w.domain_id, w.knowledge_point, w.severity, w.error_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let domain_str = domain_levels
        .iter()
        .map(|(d, l)| format!("- 领域 {} 当前等级 Lv.{}", d, l))
        .collect::<Vec<_>>()
        .join("\n");

    let material_str = if materials.is_empty() {
        "（KB 暂无可复习条目）".to_string()
    } else {
        materials
            .iter()
            .take(10)
            .map(|m| format!("- [{}] {}（领域 {}）", m.entry_id, m.title, m.domain_id))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let system_prompt = "你是天道考官，在修士突破失败后给出个性化复习建议。\
输出必须是合法 JSON 数组，不得包含任何解释性文字或 markdown 代码块标记。";

    let user_prompt = format!(
        "修士突破失败，请基于以下信息生成 3-5 条具体复习建议：\n\n\
## 历史弱点\n{}\n\n\
## 各领域掌握情况\n{}\n\n\
## 可复习 KB 条目\n{}\n\n\
输出 JSON 数组，每个元素格式：\n\
{{\"domain_id\":\"<领域>\",\"knowledge_point\":\"<知识点>\",\"reason\":\"<为什么需要复习>\",\"suggested_action\":\"<具体复习动作>\",\"related_entry_id\":\"<关联KB条目ID或null>\"}}",
        weakness_str, domain_str, material_str
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

    let suggestions = parsed
        .iter()
        .filter_map(|item| {
            Some(ReviewSuggestion {
                domain_id: item.get("domain_id")?.as_str()?.to_string(),
                knowledge_point: item.get("knowledge_point")?.as_str()?.to_string(),
                reason: item
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                suggested_action: item
                    .get("suggested_action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                related_entry_id: item
                    .get("related_entry_id")
                    .and_then(|v| v.as_str())
                    .map(String::from),
            })
        })
        .collect();

    Some(suggestions)
}

// ============================================================================
// 6. 批改一致性校验（#13）— 二次评分 + 置信度
// ============================================================================

/// 批改置信度。
#[derive(Debug, Clone, serde::Serialize)]
pub enum GradingConfidence {
    High,
    Medium,
    Low,
}

/// 对批改结果做一致性校验（可选二次评分）。
///
/// 实现策略：
/// - 当前为纯规则校验版本（不调用 AI 二次评分，节省成本）
/// - 若 batch 中存在 score 极差 > 50（最高分与最低分差距过大），标记 Low 置信度
/// - 若 overall_comment 长度 < 100，标记 Medium 置信度
/// - 其余情况标记 High
///
/// 远期可扩展为调用 AI 二次评分，差 > 15 分标记 Low。
pub fn evaluate_grading_confidence(grading: &GradingResult) -> GradingConfidence {
    if grading.question_scores.is_empty() {
        return GradingConfidence::Medium;
    }

    let scores: Vec<i32> = grading.question_scores.iter().map(|q| q.score).collect();
    let max_score = *scores.iter().max().unwrap_or(&0);
    let min_score = *scores.iter().min().unwrap_or(&0);
    let spread = max_score - min_score;

    let comment_len = grading.overall_comment.chars().count();

    if spread > 50 {
        return GradingConfidence::Low;
    }
    if comment_len < 100 || grading.weakness_analysis.len() > 5 {
        return GradingConfidence::Medium;
    }
    GradingConfidence::High
}

// ============================================================================
// 7. 文生图（#12）— 真实文生图接入（spec 阶段1 Task 1.3）
// ============================================================================

/// 为 image_choice 题生成真实图片 URL。
///
/// 实现说明（spec 阶段1 Task 1.3）：
/// - 调用云端 API（AiModelService::generate_image）走 OpenAI DALL-E 3 /images/generations
/// - provider 不支持文生图（ollama/anthropic）或调用失败时降级到 placeholder://
/// - 降级时记 `tracing::warn!`，不阻塞考验流程
///
/// 设计依据：
/// - .trae/rules/项目核心设计意图.md §五（游戏考验生成走云端 API）
/// - .trae/rules/项目核心设计意图.md §八.2（底层智能关闭后考验机制核心功能必须可用）
///
/// 接入契约：
/// - 输入：AiContext + 题目的 media_description（画面描述）
/// - 输出：图片 URL（http(s):// 或 data:image/... 或 placeholder:// 占位）
pub async fn generate_image_for_question(
    ctx: &crate::services::game_breakthrough_service::AiContext,
    media_description: &str,
) -> Option<String> {
    // media_description 为空时直接降级
    if media_description.trim().is_empty() {
        tracing::warn!("[文生图#12] media_description 为空，降级到 placeholder://");
        return None;
    }

    let ai_service = AiModelService::new();
    let size = crate::services::ai_model_service::ImageSize::Square;

    match ai_service
        .generate_image(
            &ctx.pool,
            &ctx.mek_manager,
            ctx.user_id,
            ctx.model_id,
            media_description,
            size,
        )
        .await
    {
        Ok(url) => {
            tracing::info!(
                url_len = url.len(),
                "[文生图#12] 调用成功，已生成真实图片 URL"
            );
            Some(url)
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "[文生图#12] 云端 API 调用失败，降级到 placeholder://（不阻塞考验流程）"
            );
            None
        }
    }
}

// ============================================================================
// 8. 多模态图片理解出题（#15）— 真实接入（spec 阶段1 Task 1.4）
// ============================================================================

/// 检查 KB 是否含图片素材（用于多模态出题）。
///
/// 实现说明（spec 阶段1 Task 1.4）：
/// - 查询 kb_entries 表中 entry_type='image' 的条目是否存在
/// - 仅返回是否存在（true/false），具体条目由 `list_kb_image_materials` 提供
///
/// 设计依据：
/// - KbEntry 模型 `entry_type` 字段支持 `image`（kb_service::add_entry 校验）
/// - 不依赖 kb_attachments 表（项目无此表）
pub async fn kb_has_image_materials(pool: &SqlitePool) -> bool {
    match sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM kb_entries WHERE entry_type = 'image' LIMIT 1",
    )
    .fetch_one(pool)
    .await
    {
        Ok(count) => count > 0,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "[多模态#15] 查询 kb_entries 图片素材失败，按无图片处理"
            );
            false
        }
    }
}

/// 列出 KB 中的图片素材（用于多模态出题）。
///
/// 返回 (entry_id, title, path_url) 列表，按 updated_at 倒序取前 N 条。
/// 失败时返回空 Vec（不阻塞出题，降级到文本素材）。
///
/// Bug 修复 v1.52.19.1：path_url 列在 schema 中允许 NULL，原解构为 (i64, String, String)
/// 会在 path_url 为 NULL 时 sqlx 反序列化失败，导致"有图片但 path_url 缺失"被误判为"无图片"。
/// 改为 Option<String> 解构并过滤 None，仅保留有 path_url 的图片素材。
pub async fn list_kb_image_materials(
    pool: &SqlitePool,
    limit: usize,
) -> Vec<(i64, String, String)> {
    let rows: Result<Vec<(i64, String, Option<String>)>, _> = sqlx::query_as(
        "SELECT id, name, path_url FROM kb_entries WHERE entry_type = 'image' ORDER BY updated_at DESC LIMIT ?",
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await;

    match rows {
        Ok(items) => items
            .into_iter()
            .filter_map(|(id, name, path_url)| path_url.map(|p| (id, name, p)))
            .collect(),
        Err(e) => {
            tracing::warn!(
                error = %e,
                "[多模态#15] 列出 KB 图片素材失败，返回空列表"
            );
            Vec::new()
        }
    }
}

/// spec 阶段1 Task 1.4：基于 KB 图片素材调用多模态 vision API 出题（#15 多模态图片理解出题）。
///
/// 流程：
/// 1. 调用 `AiModelService::understand_image`（vision API），传入图片 URL + 出题 prompt
/// 2. 解析 AI 返回的 JSON 为 `BreakthroughQuestion`（委托 `parse_image_understanding_response`）
/// 3. 失败时返回 `AppError::AiApi`，由调用方降级到文本素材出题
///
/// 设计依据：
/// - .trae/rules/项目核心设计意图.md §五（游戏考验生成走云端 API）
/// - .trae/rules/项目核心设计意图.md §八.2（vision API 失败时降级路径明确）
///
/// 参数：
/// - `ctx`：AI 调用上下文（含 model_id，需选用支持 vision 的模型如 glm-4v-plus / qwen-vl-max）
/// - `image_url`：图片 URL（http(s):// 或 data:image/...）
/// - `domain`：知识点所属领域 ID（如 "cs" / "math"）
pub async fn ai_understand_image_for_question(
    ctx: &crate::services::game_breakthrough_service::AiContext,
    image_url: &str,
    domain: &str,
) -> Result<BreakthroughQuestion, AppError> {
    let user_prompt = format!(
        "请基于以下图片生成 1 道 image_understanding 题型题目：\n\
- 图片 URL：{}\n\
- 目标领域：{}\n\
- 题型：image_understanding（图片理解题）\n\
- 要求：4 选项 A/B/C/D，standard_answer 填字母如 \"B\"\n\
- media_url 直接使用上方图片 URL\n\
- media_description 描述图片核心内容（用于无障碍访问）\n\n\
输出 JSON 对象（不要包裹在数组中）：\n\
{{\"question_type\":\"image_understanding\",\"domain_id\":\"{}\",\"knowledge_point\":\"<基于图片提取的知识点>\",\"content\":\"<题干：观察图片，以下哪项描述正确？>\",\"options\":[\"A. ...\",\"B. ...\",\"C. ...\",\"D. ...\"],\"standard_answer\":\"<字母>\",\"difficulty\":1.0,\"media_type\":\"image\",\"media_url\":\"{}\",\"media_description\":\"<图片画面描述>\"}}",
        image_url, domain, domain, image_url
    );

    let ai_service = AiModelService::new();
    let raw = ai_service
        .understand_image(
            &ctx.pool,
            &ctx.mek_manager,
            ctx.user_id,
            ctx.model_id,
            image_url,
            &user_prompt,
        )
        .await?;

    tracing::info!(
        raw_len = raw.len(),
        "[多模态#15] vision API 调用成功，开始解析为 BreakthroughQuestion"
    );

    parse_image_understanding_response(&raw, image_url, domain)
}

/// spec 阶段1 Task 1.4：解析 vision API 返回的 JSON 为 `BreakthroughQuestion`。
///
/// 抽出为独立 pub 函数便于单元测试（不依赖云端 API）。
///
/// 解析规则：
/// - 去除可能的 ```json ... ``` 代码块包裹
/// - 缺字段时使用合理默认值（question_type=image_understanding, media_type=image 等）
/// - media_url 缺失时回退到入参 image_url
/// - JSON 解析失败返回 `AppError::AiApi`
pub fn parse_image_understanding_response(
    raw: &str,
    image_url: &str,
    domain: &str,
) -> Result<BreakthroughQuestion, AppError> {
    let trimmed = strip_json_codeblock(raw);
    let parsed: serde_json::Value = serde_json::from_str(&trimmed).map_err(|e| {
        AppError::AiApi(format!(
            "vision API 响应解析 JSON 失败: {} (raw={})",
            e,
            &raw[..raw.len().min(200)]
        ))
    })?;

    let question_type = parsed
        .get("question_type")
        .and_then(|v| v.as_str())
        .unwrap_or("image_understanding")
        .to_string();
    let q_domain = parsed
        .get("domain_id")
        .and_then(|v| v.as_str())
        .unwrap_or(domain)
        .to_string();
    let knowledge_point = parsed
        .get("knowledge_point")
        .and_then(|v| v.as_str())
        .unwrap_or("图片理解")
        .to_string();
    let content = parsed
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("观察图片，以下哪项描述正确？")
        .to_string();
    let standard_answer = parsed
        .get("standard_answer")
        .and_then(|v| v.as_str())
        .unwrap_or("A")
        .to_string();
    let options: Option<Vec<String>> = parsed
        .get("options")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|o| o.as_str().map(|s| s.to_string()))
                .collect()
        });
    let difficulty = parsed
        .get("difficulty")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);
    let media_url = parsed
        .get("media_url")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| Some(image_url.to_string()));
    let media_type = parsed
        .get("media_type")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| Some("image".to_string()));
    let media_description = parsed
        .get("media_description")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(BreakthroughQuestion {
        question_id: format!("q_img_{}", uuid::Uuid::new_v4()),
        question_type,
        domain_id: q_domain,
        knowledge_point,
        content,
        options,
        standard_answer,
        difficulty,
        media_url,
        media_type,
        media_description,
    })
}

// ============================================================================
// 9. 辅助函数
// ============================================================================

/// 去除 LLM 输出可能的 ```json ... ``` 代码块包裹（与主模块同名函数保持一致）。
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

// 引用 game_repo + kb_repo（避免顶部 use 与循环依赖）
use crate::db::repositories::game_repo;
use crate::db::repositories::kb_repo;

// ============================================================================
// 10. 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    // 修复 spec 阶段1 Task 1.5：现有测试用例引用 QuestionScore 但未导入，补全 import
    use crate::services::game_breakthrough_service::QuestionScore;

    fn make_question(id: &str, q_type: &str, domain: &str, kp: &str) -> BreakthroughQuestion {
        BreakthroughQuestion {
            question_id: id.to_string(),
            question_type: q_type.to_string(),
            domain_id: domain.to_string(),
            knowledge_point: kp.to_string(),
            content: "test".to_string(),
            options: if q_type == "choice" || q_type == "image_choice" {
                Some(vec!["A".into(), "B".into(), "C".into(), "D".into()])
            } else {
                None
            },
            standard_answer: "A".to_string(),
            difficulty: 1.0,
            media_url: None,
            media_type: None,
            media_description: None,
        }
    }

    #[test]
    fn test_validate_questions_basic_pass() {
        let questions = vec![
            make_question("q1", "choice", "cs", "知识点1"),
            make_question("q2", "short_answer", "math", "知识点2"),
            make_question("q3", "application", "cs", "知识点3"),
        ];
        let result = validate_generated_questions(&questions, 3, 1, 1, 1, &[], &[]);
        assert!(result.is_valid, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_validate_questions_count_mismatch() {
        let questions = vec![make_question("q1", "choice", "cs", "知识点1")];
        let result = validate_generated_questions(&questions, 3, 1, 1, 1, &[], &[]);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("数量不匹配")));
    }

    #[test]
    fn test_validate_questions_id_not_continuous() {
        let questions = vec![
            make_question("q1", "choice", "cs", "知识点1"),
            make_question("q3", "short_answer", "math", "知识点2"), // q2 被跳过
        ];
        let result = validate_generated_questions(&questions, 2, 1, 1, 0, &[], &[]);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("不连续")));
    }

    #[test]
    fn test_validate_questions_choice_missing_options() {
        let mut q = make_question("q1", "choice", "cs", "知识点1");
        q.options = None;
        let questions = vec![q];
        let result = validate_generated_questions(&questions, 1, 1, 0, 0, &[], &[]);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("缺少 options")));
    }

    #[test]
    fn test_validate_questions_high_weakness_not_covered() {
        let weakness = vec![WeaknessItem {
            domain_id: "cs".to_string(),
            knowledge_point: "递归".to_string(),
            error_count: 3,
            severity: "high".to_string(),
            last_exposed_at: 0,
            resolved: false,
            resolved_at: None,
        }];
        // 题目不考察"递归"
        let questions = vec![make_question("q1", "choice", "cs", "排序算法")];
        let result = validate_generated_questions(&questions, 1, 1, 0, 0, &weakness, &[]);
        assert!(result.warnings.iter().any(|w| w.contains("high 严重度弱点未实际出现")));
    }

    #[test]
    fn test_validate_questions_dedup_hit() {
        let recent = vec![RecentQuestionKey {
            knowledge_point: "递归".to_string(),
            question_type: "choice".to_string(),
        }];
        let questions = vec![make_question("q1", "choice", "cs", "递归")];
        let result = validate_generated_questions(&questions, 1, 1, 0, 0, &[], &recent);
        assert!(result.warnings.iter().any(|w| w.contains("去重校验未通过")));
    }

    #[test]
    fn test_validate_grading_score_consistency() {
        let grading = GradingResult {
            total_score: 80, // 与均值 70 差 10
            daoji_level: "red".to_string(), // 60-74 应为 red，80 应为 purple
            question_scores: vec![QuestionScore {
                question_id: "q1".to_string(),
                score: 70,
                correctness: 70,
                completeness: 70,
                expression: 70,
                depth: 70,
                comment: "test".to_string(),
                weak_points: vec![],
            }],
            overall_comment: "这是一个长度刚好超过100字的评语用于测试批改校验器的评语长度校验功能，需要确保字符数在100到300之间才能通过校验，此处继续填充内容以达到下限要求。".to_string(),
            weakness_analysis: vec![],
        };
        let result = validate_grading_result(&grading);
        assert!(!result.is_valid);
        // 总分不一致 + 道基不匹配
        assert!(result.errors.iter().any(|e| e.contains("总分与单题均值不一致")));
        assert!(result.errors.iter().any(|e| e.contains("道基等级与分数不匹配")));
    }

    #[test]
    fn test_evaluate_grading_confidence_low() {
        let grading = GradingResult {
            total_score: 50,
            daoji_level: "blue".to_string(),
            question_scores: vec![
                QuestionScore {
                    question_id: "q1".to_string(),
                    score: 90,
                    correctness: 90,
                    completeness: 90,
                    expression: 90,
                    depth: 90,
                    comment: "good".to_string(),
                    weak_points: vec![],
                },
                QuestionScore {
                    question_id: "q2".to_string(),
                    score: 10, // 极差 80 > 50
                    correctness: 10,
                    completeness: 10,
                    expression: 10,
                    depth: 10,
                    comment: "bad".to_string(),
                    weak_points: vec![],
                },
            ],
            overall_comment: "test".to_string(),
            weakness_analysis: vec![],
        };
        assert!(matches!(
            evaluate_grading_confidence(&grading),
            GradingConfidence::Low
        ));
    }

    #[test]
    fn test_heavenly_examiner_prompt_not_empty() {
        let prompt = build_heavenly_examiner_system_prompt();
        assert!(prompt.contains("天道考官"));
        assert!(prompt.contains("8 条"));
        assert!(prompt.contains("JSON"));
    }

    // ========================================================================
    // spec 阶段1 Task 1.5：#15 多模态图片理解响应解析单元测试
    // ========================================================================

    /// 完整合法 JSON 应正确解析为 BreakthroughQuestion（所有字段齐全）。
    #[test]
    fn test_parse_image_understanding_response_valid_json() {
        let raw = r#"{
            "question_type": "image_understanding",
            "domain_id": "cs",
            "knowledge_point": "数据结构二叉树",
            "content": "观察图片，以下哪项描述正确？",
            "options": ["A. 根节点", "B. 叶子节点", "C. 内部节点", "D. 子树"],
            "standard_answer": "B",
            "difficulty": 1.2,
            "media_type": "image",
            "media_url": "https://example.com/img.png",
            "media_description": "一棵二叉树的示意图"
        }"#;
        let result = parse_image_understanding_response(raw, "https://fallback/img.png", "cs")
            .expect("完整 JSON 应解析成功");
        assert_eq!(result.question_type, "image_understanding");
        assert_eq!(result.domain_id, "cs");
        assert_eq!(result.knowledge_point, "数据结构二叉树");
        assert_eq!(result.content, "观察图片，以下哪项描述正确？");
        assert_eq!(result.standard_answer, "B");
        assert_eq!(result.difficulty, 1.2);
        assert_eq!(result.media_url.as_deref(), Some("https://example.com/img.png"));
        assert_eq!(result.media_type.as_deref(), Some("image"));
        assert_eq!(result.media_description.as_deref(), Some("一棵二叉树的示意图"));
        assert_eq!(
            result.options.as_ref().map(|o| o.len()),
            Some(4),
            "options 应解析为 4 个选项"
        );
        assert!(
            result.question_id.starts_with("q_img_"),
            "question_id 应以 q_img_ 前缀生成"
        );
    }

    /// 带 ```json``` 代码块包裹的响应应正确去除包裹后解析。
    #[test]
    fn test_parse_image_understanding_response_codeblock_wrapped() {
        let raw = "```json\n{\"question_type\":\"image_understanding\",\"domain_id\":\"math\",\"knowledge_point\":\"几何\",\"content\":\"看图\",\"options\":[\"A. x\",\"B. y\",\"C. z\",\"D. w\"],\"standard_answer\":\"A\",\"difficulty\":1.0,\"media_type\":\"image\",\"media_url\":\"https://e.com/x.png\",\"media_description\":\"三角形\"}\n```";
        let result = parse_image_understanding_response(raw, "https://fallback/x.png", "math")
            .expect("代码块包裹的 JSON 应解析成功");
        assert_eq!(result.domain_id, "math");
        assert_eq!(result.knowledge_point, "几何");
        assert_eq!(result.standard_answer, "A");
    }

    /// 非法 JSON 应返回 AppError::AiApi 错误，不 panic。
    #[test]
    fn test_parse_image_understanding_response_invalid_json_returns_err() {
        let raw = "this is not json";
        let result = parse_image_understanding_response(raw, "https://fallback/x.png", "cs");
        assert!(result.is_err(), "非法 JSON 应返回错误");
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("AiApi") || err_msg.contains("解析 JSON 失败"),
            "错误类型应为 AiApi，实际: {}",
            err_msg
        );
    }

    /// 缺失大部分字段时应使用合理默认值（不 panic、不返回错误）。
    #[test]
    fn test_parse_image_understanding_response_missing_fields_uses_defaults() {
        // 仅提供 question_type，其他字段全部缺失
        let raw = r#"{"question_type":"image_understanding"}"#;
        let result = parse_image_understanding_response(raw, "https://fallback/img.png", "cs")
            .expect("缺字段应使用默认值，不报错");
        assert_eq!(result.question_type, "image_understanding");
        assert_eq!(
            result.domain_id, "cs",
            "domain_id 缺失时应回退到入参 domain"
        );
        assert_eq!(
            result.knowledge_point, "图片理解",
            "knowledge_point 缺失时应使用默认值"
        );
        assert_eq!(
            result.content, "观察图片，以下哪项描述正确？",
            "content 缺失时应使用默认题干"
        );
        assert_eq!(
            result.standard_answer, "A",
            "standard_answer 缺失时应使用默认 A"
        );
        assert_eq!(result.difficulty, 1.0, "difficulty 缺失时应默认 1.0");
        assert_eq!(
            result.media_type.as_deref(),
            Some("image"),
            "media_type 缺失时应默认 image"
        );
        assert!(result.options.is_none(), "options 缺失时应为 None");
    }

    /// media_url 缺失时应回退到入参 image_url，确保题目始终有图源。
    #[test]
    fn test_parse_image_understanding_response_media_url_fallback() {
        let raw = r#"{"question_type":"image_understanding","knowledge_point":"测试"}"#;
        let fallback_url = "https://kb.example.com/materials/42.png";
        let result = parse_image_understanding_response(raw, fallback_url, "cs")
            .expect("应解析成功");
        assert_eq!(
            result.media_url.as_deref(),
            Some(fallback_url),
            "media_url 缺失时必须回退到入参 image_url"
        );
    }
}
