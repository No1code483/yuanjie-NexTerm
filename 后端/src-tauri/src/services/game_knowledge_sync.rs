//! 游戏 3D 重构 - 知识积分同步 Service
//!
//! Task 4.2：实现学习/工作/科研行为 → 积分入账的业务逻辑。
//! change-id: `game-3d-rebuild-refactor`
//!
//! 设计原则（03_积分系统设计.md）：
//! - **领域绑定**：所有积分归属 12 领域之一，不存在"无主积分"
//! - **幂等可重放**：积分事件带 `event_id`，重复事件不重复入账
//! - **每日上限保护**：高频行为（AI 会话/YuanCode）设每日上限
//! - **变更可审计**：每次变动记录到 `game_points_log`
//! - **领域不可负**：积分下限为 0
//!
//! 核心数据流：
//! 1. 外部行为触发 → 调用 `award_points`
//! 2. 校验每日上限（`game_daily_limit_counter`）
//! 3. 幂等检查（`game_points_log.event_id` UNIQUE）
//! 4. 累加积分（`game_knowledge_progress`）
//! 5. 推进建造进度（同领域建造中建筑）
//! 6. 转换修为（积分 × 道基系数 → `game_worlds.realm_xp`）

use sqlx::SqlitePool;

use crate::db::repositories::game_repo;
use crate::error::app_error::AppError;
use crate::models::game::{
    building_required_points, dao_foundation_multiplier, domain_level_from_earned, DaoFoundation,
    GameBuilding, PointsLogResult,
};

// ============================================================================
// 辅助函数
// ============================================================================

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn new_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// 当前日期（Asia/Shanghai 时区，`'YYYY-MM-DD'`）。
fn today_shanghai() -> String {
    let now = chrono::Utc::now();
    let beijing = now + chrono::Duration::hours(8);
    beijing.format("%Y-%m-%d").to_string()
}

// ============================================================================
// 1. 积分入账核心函数
// ============================================================================

/// 积分入账请求。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AwardPointsRequest {
    pub world_id: String,
    /// 积分来源类型（11 种，见 `models::game::source_type`）。
    pub source_type: String,
    /// 目标领域 id（如 `cs`/`math`/...）。
    pub domain_id: String,
    /// 业务事件唯一 ID（幂等键，重复事件不重复入账）。
    pub event_id: String,
    /// 积分变动值（正数入账，负数扣除）。
    pub points_delta: i64,
    /// 附加元数据（JSON 字符串，如条目 ID、会话 ID 等）。
    pub metadata_json: Option<String>,
}

/// 积分入账结果。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AwardPointsResult {
    /// 入账结果：`accepted` / `rejected_daily_limit` / `rejected_negative`。
    pub result: String,
    /// 入账后该领域可用积分。
    pub points_after: i64,
    /// 实际入账积分（被拒绝时为 0）。
    pub points_credited: i64,
    /// 本次事件是否为重复事件（幂等命中）。
    pub is_duplicate: bool,
    /// 被推进进度的建筑列表（仅 `accepted` 且 points_delta > 0 时非空）。
    pub build_progress_updated: Vec<BuildProgressUpdate>,
}

/// 建筑进度推进结果（service 层 DTO，与 commands 层 `BuildProgressDelta` 解耦）。
///
/// 注：`progress_before` 为读取快照，并发场景下可能与 DB 实际值略有差异；
///     `delta` 为绝对增量（基于 `actual_share / required × 100` 计算），不受快照准确性影响。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BuildProgressUpdate {
    pub building_id: String,
    pub progress_before: f64,
    pub progress_after: f64,
    pub delta: f64,
    pub completed: bool,
}

/// 积分入账主函数（核心业务逻辑）。
///
/// 流程：
/// 1. 幂等检查：`event_id` 已存在则返回原结果（不重复入账）
/// 2. 每日上限校验：查 `game_points_source_config.daily_limit` + `game_daily_limit_counter`
/// 3. 余额校验：负数扣除时检查领域积分是否足够
/// 4. 累加积分到 `game_knowledge_progress`（更新 points/total_earned/level）
/// 5. 记录 `game_points_log` 审计日志
/// 6. 推进同领域建造中建筑进度（按比例分配）
/// 7. 转换修为：`points_delta × 道基系数` 累加到 `game_worlds.realm_xp/total_xp`
///
/// 事务保证 1-5 步原子性；6-7 步在事务外异步推进（失败不影响入账）。
pub async fn award_points(
    pool: &SqlitePool,
    user_id: i64,
    req: &AwardPointsRequest,
) -> Result<AwardPointsResult, AppError> {
    // 1. 幂等检查
    if let Some(existing_log) = game_repo::find_points_log_by_event(pool, &req.event_id).await? {
        return Ok(AwardPointsResult {
            result: existing_log.result.clone(),
            points_after: existing_log.points_after,
            points_credited: 0,
            is_duplicate: true,
            // 幂等命中：重复事件不再推进建造进度
            build_progress_updated: Vec::new(),
        });
    }

    // 2. 查询积分来源配置（含每日上限）
    let source_config = game_repo::get_source_config(pool, &req.source_type)
        .await?
        .ok_or_else(|| AppError::Validation(format!("未知积分来源: {}", req.source_type)))?;

    if !source_config.enabled {
        return Ok(rejected(
            PointsLogResult::RejectedDailyLimit,
            req,
            "积分来源已禁用",
        ));
    }

    // 3. 每日上限校验
    if let Some(daily_limit) = source_config.daily_limit {
        let counter = game_repo::get_daily_counter(pool, &req.world_id, &req.domain_id, &req.source_type).await?;
        let current_count = counter.map(|c| c.current_count).unwrap_or(0);
        if current_count >= daily_limit {
            return Ok(rejected(
                PointsLogResult::RejectedDailyLimit,
                req,
                &format!("每日上限已达：{}/{}", current_count, daily_limit),
            ));
        }
    }

    // 4. 负数扣除时余额校验
    if req.points_delta < 0 {
        let progress = game_repo::get_progress(pool, &req.world_id, &req.domain_id)
            .await?
            .ok_or_else(|| AppError::Validation(format!(
                "领域 {} 未初始化进度记录", req.domain_id
            )))?;
        if progress.points + req.points_delta < 0 {
            return Ok(rejected(
                PointsLogResult::RejectedNegative,
                req,
                &format!("余额不足：当前 {}，需扣除 {}", progress.points, -req.points_delta),
            ));
        }
    }

    // 5. 事务：累加积分 + 记日志 + 计数器
    let now = now_ms();
    let log_id = new_uuid();
    let counter_id = new_uuid();
    let today = today_shanghai();

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    // 查询当前 progress（事务内锁定行）
    let progress: Option<(i64, i64, i64)> = sqlx::query_as(
        "SELECT points, total_earned, total_consumed FROM game_knowledge_progress
         WHERE world_id = ? AND domain_id = ?",
    )
    .bind(&req.world_id)
    .bind(&req.domain_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    let (old_points, old_earned, old_consumed) = progress
        .ok_or_else(|| AppError::Validation(format!("领域 {} 未初始化进度记录", req.domain_id)))?;

    let new_points = old_points + req.points_delta;
    let new_earned = old_earned + req.points_delta.max(0); // 仅正数累加到 total_earned
    let new_consumed = old_consumed + (-req.points_delta).max(0); // 仅负数的绝对值累加到 total_consumed
    let new_level = domain_level_from_earned(new_earned) as i32;

    // 更新 progress
    sqlx::query(
        "UPDATE game_knowledge_progress
         SET points = ?, total_earned = ?, total_consumed = ?, level = ?, updated_at = ?
         WHERE world_id = ? AND domain_id = ?",
    )
    .bind(new_points)
    .bind(new_earned)
    .bind(new_consumed)
    .bind(new_level)
    .bind(now)
    .bind(&req.world_id)
    .bind(&req.domain_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // 记积分日志（accepted）
    sqlx::query(
        "INSERT INTO game_points_log
         (id, world_id, source_type, domain_id, points_delta, points_after,
          result, event_id, metadata_json, created_at)
         VALUES (?, ?, ?, ?, ?, ?, 'accepted', ?, ?, ?)",
    )
    .bind(&log_id)
    .bind(&req.world_id)
    .bind(&req.source_type)
    .bind(&req.domain_id)
    .bind(req.points_delta)
    .bind(new_points)
    .bind(&req.event_id)
    .bind(&req.metadata_json)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // 更新每日计数器（仅正数入账时计数）
    if req.points_delta > 0 {
        sqlx::query(
            "INSERT INTO game_daily_limit_counter
             (id, world_id, counter_date, domain_id, source_type, current_count, updated_at)
             VALUES (?, ?, ?, ?, ?, 1, ?)
             ON CONFLICT(world_id, counter_date, domain_id, source_type) DO UPDATE SET
                 current_count = current_count + 1,
                 updated_at = excluded.updated_at",
        )
        .bind(&counter_id)
        .bind(&req.world_id)
        .bind(&today)
        .bind(&req.domain_id)
        .bind(&req.source_type)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;
    }

    tx.commit().await.map_err(AppError::Database)?;

    // 6. 推进同领域建造中建筑进度（事务外，失败不影响入账）
    let build_progress_updated = if req.points_delta > 0 {
        match update_build_progress(pool, user_id, &req.world_id, &req.domain_id, req.points_delta).await {
            Ok(deltas) => deltas,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    world_id = %req.world_id,
                    domain_id = %req.domain_id,
                    "建造进度推进失败（不影响积分入账）"
                );
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    // 7. 修为转换（事务外，失败不影响入账）
    if req.points_delta > 0 {
        if let Err(e) = update_realm_xp(pool, user_id, &req.world_id, req.points_delta).await {
            tracing::warn!(error = %e, world_id = %req.world_id, "修为更新失败（不影响积分入账）");
        }
    }

    Ok(AwardPointsResult {
        result: PointsLogResult::Accepted.as_str().to_string(),
        points_after: new_points,
        points_credited: req.points_delta,
        is_duplicate: false,
        build_progress_updated,
    })
}

/// 构造拒绝结果（不入账，仅记日志）。
fn rejected(result: PointsLogResult, req: &AwardPointsRequest, reason: &str) -> AwardPointsResult {
    tracing::info!(
        world_id = %req.world_id,
        domain_id = %req.domain_id,
        source_type = %req.source_type,
        reason = %reason,
        "积分入账被拒绝"
    );
    AwardPointsResult {
        result: result.as_str().to_string(),
        points_after: 0,
        points_credited: 0,
        is_duplicate: false,
        // 拒绝事件不入账积分，自然不推进建造进度
        build_progress_updated: Vec::new(),
    }
}

// ============================================================================
// 2. 修为转换（积分 → 修为）
// ============================================================================

/// 将积分增量转换为修为，累加到世界主表。
///
/// 公式（02 文档 §2.2）：`修为增量 = 积分 × 道基系数`
///
/// - 道基系数：白 0.5 / 蓝 0.7 / 红 1.0 / 紫 1.3 / 黑 1.6
/// - 修为累加到 `game_worlds.realm_xp`（当前大境界内）和 `total_xp`（累计总修为）
/// - 若 `realm_xp` 超过当前大境界上限，自动停留在上限（突破由 AI 考验触发，不自动晋升）
async fn update_realm_xp(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    points_delta: i64,
) -> Result<(), AppError> {
    use crate::models::game::RealmMajor;

    // 多用户隔离（批次 6）：按 user_id 过滤
    let world = game_repo::get_world(pool, user_id, world_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let dao = DaoFoundation::from_str(&world.dao_foundation)
        .ok_or_else(|| AppError::Internal(format!("无效的 dao_foundation: {}", world.dao_foundation)))?;
    let multiplier = dao_foundation_multiplier(dao);

    // 修为增量 = 积分 × 道基系数（向下取整）
    let xp_delta = (points_delta as f64 * multiplier) as i64;

    let realm_major = RealmMajor::from_str(&world.realm_major)
        .ok_or_else(|| AppError::Internal(format!("无效的 realm_major: {}", world.realm_major)))?;

    // realm_xp 不超过当前大境界上限（突破由 AI 考验触发）
    let realm_xp_upper = realm_major.xp_upper_bound();
    let total_xp_upper = realm_major.xp_upper_bound();
    let _ = total_xp_upper; // total_xp 实际无上限，但 realm_xp 有上限

    let new_realm_xp = (world.realm_xp + xp_delta).min(realm_xp_upper);
    let new_total_xp = world.total_xp + xp_delta;
    let now = now_ms();

    sqlx::query(
        "UPDATE game_worlds SET realm_xp = ?, total_xp = ?, updated_at = ? WHERE id = ? AND user_id = ?",
    )
    .bind(new_realm_xp)
    .bind(new_total_xp)
    .bind(now)
    .bind(world_id)
    .bind(world.user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(())
}

// ============================================================================
// 3. 建造进度推进（积分 → 建筑进度）
// ============================================================================

/// 计算各建筑的进度增量（纯算法，无 DB 副作用）。
///
/// 算法（03 文档 §6.2 多建筑分摊）：
/// 1. 各建筑 `required_i = building_required_points(category, subtype, level) as f64`
/// 2. 各建筑 `contributed_i = required × (progress_before / 100)`
/// 3. 各建筑 `remaining_i = max(0.0, required - contributed)`
/// 4. `total_remaining = Σ remaining`；为 0 → 返回空 vec
/// 5. `share_i = points_delta × (remaining_i / total_remaining)`
/// 6. `actual_share_i = min(share_i, remaining_i)`  // 不溢出
/// 7. `ΔProgress_i = (actual_share_i / required_i) × 100`
/// 8. `progress_after_i = min(100.0, progress_before + ΔProgress)`
///    浮点兜底：`progress_after > 99.9999 → 100.0`
/// 9. `completed = progress_after >= 100.0`
///
/// 跳过条件（不返回这些条目）：
/// - `required <= 0`（异常数据，warn 日志）
/// - `progress_before >= 100.0`（数据异常，warn 日志）
fn compute_build_progress_deltas(
    buildings: &[GameBuilding],
    points_delta: i64,
) -> Vec<BuildProgressUpdate> {
    // 0. 早返回：负数/零积分或空建筑列表不推进
    if points_delta <= 0 || buildings.is_empty() {
        return Vec::new();
    }

    let points = points_delta as f64;

    // 1-3. 计算各建筑 required / contributed / remaining，跳过异常数据
    struct Ctx<'a> {
        building: &'a GameBuilding,
        required: f64,
        remaining: f64,
    }

    let mut contexts: Vec<Ctx<'_>> = Vec::with_capacity(buildings.len());
    for b in buildings {
        let required =
            building_required_points(&b.building_category, &b.building_subtype, b.level) as f64;

        if required <= 0.0 {
            tracing::warn!(
                building_id = %b.id,
                category = %b.building_category,
                subtype = %b.building_subtype,
                level = b.level,
                "建筑所需积分为非正数，跳过进度推进"
            );
            continue;
        }

        if b.build_progress >= 100.0 {
            tracing::warn!(
                building_id = %b.id,
                progress = b.build_progress,
                "建筑进度已达 100% 但仍处于 building 状态，跳过进度推进"
            );
            continue;
        }

        let contributed = required * (b.build_progress / 100.0);
        let remaining = (required - contributed).max(0.0);

        contexts.push(Ctx {
            building: b,
            required,
            remaining,
        });
    }

    if contexts.is_empty() {
        return Vec::new();
    }

    // 4. total_remaining
    let total_remaining: f64 = contexts.iter().map(|c| c.remaining).sum();
    if total_remaining <= 0.0 {
        return Vec::new();
    }

    // 5-9. 按比例分摊 + capping + 浮点兜底
    let mut updates: Vec<BuildProgressUpdate> = Vec::with_capacity(contexts.len());
    for c in contexts {
        // 5. share_i = points × (remaining_i / total_remaining)
        let share = points * (c.remaining / total_remaining);
        // 6. actual_share_i = min(share_i, remaining_i)
        let actual_share = share.min(c.remaining);
        // 7. ΔProgress_i = (actual_share_i / required_i) × 100
        let delta = (actual_share / c.required) * 100.0;
        // 8. progress_after_i = min(100.0, progress_before + ΔProgress)
        let mut progress_after = (c.building.build_progress + delta).min(100.0);
        //    浮点兜底：> 99.9999 → 100.0
        if progress_after > 99.9999 {
            progress_after = 100.0;
        }
        // 9. completed
        let completed = progress_after >= 100.0;

        updates.push(BuildProgressUpdate {
            building_id: c.building.id.clone(),
            progress_before: c.building.build_progress,
            progress_after,
            delta,
            completed,
        });
    }

    updates
}

/// 推进同领域建造中建筑进度（事务外调用，失败不影响积分入账）。
///
/// 流程：
/// 1. `points_delta <= 0` → 返回空 vec
/// 2. 查询同领域建造中建筑：`list_buildings_by_world_domain_status(pool, world, domain, "building")`
/// 3. 调用 `compute_build_progress_deltas` 算法
/// 4. 事务内逐个相对更新（避免读改写竞态）：
///    - 未完成：`UPDATE ... SET build_progress = MIN(100.0, build_progress + ?) WHERE id = ?`
///    - 已完成：`UPDATE ... SET build_progress=100.0, status='completed', completed_at=? WHERE id=?`
///    - 已完成时同步插入 `event_type='complete'` 的 `game_build_history` 记录（同事务）
/// 5. 事务提交，返回 deltas
async fn update_build_progress(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    domain_id: &str,
    points_delta: i64,
) -> Result<Vec<BuildProgressUpdate>, AppError> {
    use crate::models::game::BuildHistoryEventType;

    // 1. 负数/零积分不推进
    if points_delta <= 0 {
        return Ok(Vec::new());
    }

    // 2. 查询同领域建造中建筑
    let buildings =
        game_repo::list_buildings_by_world_domain_status(pool, world_id, domain_id, "building")
            .await?;

    if buildings.is_empty() {
        return Ok(Vec::new());
    }

    // 3. 计算各建筑进度增量
    let deltas = compute_build_progress_deltas(&buildings, points_delta);
    if deltas.is_empty() {
        return Ok(Vec::new());
    }

    // 4. 事务内逐个更新
    let mut tx = pool.begin().await.map_err(AppError::Database)?;
    let now = now_ms();

    // 收集本次事务中产生的 complete 事件，事务提交后调用快照钩子（11_时间轴回放.md §5.1）
    let mut completed_history: Vec<(String, String)> = Vec::new();

    for d in &deltas {
        // 查找对应建筑（用于历史快照）
        let b_opt = buildings.iter().find(|b| b.id == d.building_id);

        if d.completed {
            // 已完成：更新 status + completed_at
            sqlx::query(
                "UPDATE game_buildings
                 SET build_progress = 100.0, status = 'completed', completed_at = ?
                 WHERE id = ?",
            )
            .bind(now)
            .bind(&d.building_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

            // 同步插入 event_type='complete' 的建造历史（直接 SQL，避免跨事务调用 game_repo）
            if let Some(b) = b_opt {
                let history_id = new_uuid();
                sqlx::query(
                    "INSERT INTO game_build_history
                     (id, world_id, event_type, building_id, building_name,
                      pos_x, pos_y, pos_z, level, progress, snapshot_json, created_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&history_id)
                .bind(&b.world_id)
                .bind(BuildHistoryEventType::Complete.as_str())
                .bind(&b.id)
                .bind(&b.name)
                .bind(b.pos_x)
                .bind(b.pos_y)
                .bind(b.pos_z)
                .bind(b.level as i32)
                .bind(100.0_f64)
                .bind(None::<String>)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(AppError::Database)?;

                // 收集 complete 事件，事务提交后触发快照
                completed_history.push((b.world_id.clone(), history_id));
            }
        } else {
            // 未完成：相对更新避免竞态
            sqlx::query(
                "UPDATE game_buildings
                 SET build_progress = MIN(100.0, build_progress + ?)
                 WHERE id = ?",
            )
            .bind(d.delta)
            .bind(&d.building_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        }
    }

    tx.commit().await.map_err(AppError::Database)?;

    // T1.1 时间轴回放：complete 事件快照钩子（11_时间轴回放.md §5.1）
    for (wid, hid) in &completed_history {
        let _ =
            crate::services::snapshot_service::maybe_write_snapshot_after_event(pool, user_id, wid, hid, "complete")
                .await;
    }

    Ok(deltas)
}

// ============================================================================
// 4. 知识库联动便利函数
// ============================================================================

/// 知识库条目新增 → 积分入账（+10，无每日上限）。
///
/// 领域由条目分类映射决定（调用方查询 `game_kb_category_mapping` 后传入）。
pub async fn sync_kb_entry_added(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    domain_id: &str,
    event_id: &str,
    entry_id: i64,
) -> Result<AwardPointsResult, AppError> {
    let req = AwardPointsRequest {
        world_id: world_id.to_string(),
        source_type: crate::models::game::source_type::KB_ENTRY_CREATE.to_string(),
        domain_id: domain_id.to_string(),
        event_id: event_id.to_string(),
        points_delta: 10,
        metadata_json: Some(format!(r#"{{"entry_id":{}}}"#, entry_id)),
    };
    award_points(pool, user_id, &req).await
}

/// 知识库分类新增 → 积分入账（+50，无每日上限）。
pub async fn sync_kb_category_added(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    domain_id: &str,
    event_id: &str,
    category_id: i64,
) -> Result<AwardPointsResult, AppError> {
    let req = AwardPointsRequest {
        world_id: world_id.to_string(),
        source_type: crate::models::game::source_type::KB_CATEGORY_CREATE.to_string(),
        domain_id: domain_id.to_string(),
        event_id: event_id.to_string(),
        points_delta: 50,
        metadata_json: Some(format!(r#"{{"category_id":{}}}"#, category_id)),
    };
    award_points(pool, user_id, &req).await
}

/// 知识库条目更新 → 积分入账（+2，每日上限 20）。
pub async fn sync_kb_entry_updated(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    domain_id: &str,
    event_id: &str,
    entry_id: i64,
) -> Result<AwardPointsResult, AppError> {
    let req = AwardPointsRequest {
        world_id: world_id.to_string(),
        source_type: crate::models::game::source_type::KB_ENTRY_UPDATE.to_string(),
        domain_id: domain_id.to_string(),
        event_id: event_id.to_string(),
        points_delta: 2,
        metadata_json: Some(format!(r#"{{"entry_id":{}}}"#, entry_id)),
    };
    award_points(pool, user_id, &req).await
}

/// 待办完成 → 积分入账（+5，engineering 领域，每日上限 50）。
pub async fn sync_todo_completed(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    event_id: &str,
    todo_id: i64,
) -> Result<AwardPointsResult, AppError> {
    let req = AwardPointsRequest {
        world_id: world_id.to_string(),
        source_type: crate::models::game::source_type::TODO_COMPLETE.to_string(),
        domain_id: crate::models::game::domain::ENGINEERING.to_string(),
        event_id: event_id.to_string(),
        points_delta: 5,
        metadata_json: Some(format!(r#"{{"todo_id":{}}}"#, todo_id)),
    };
    award_points(pool, user_id, &req).await
}

/// 计时器完成（短，<25min）→ 积分入账（engineering +3, philosophy +1）。
///
/// 短计时器触发两个领域的入账，各自独立计数。
pub async fn sync_timer_short_completed(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    event_id: &str,
    timer_id: i64,
) -> Result<Vec<AwardPointsResult>, AppError> {
    let mut results = Vec::with_capacity(2);

    // engineering +3
    let req1 = AwardPointsRequest {
        world_id: world_id.to_string(),
        source_type: crate::models::game::source_type::TIMER_SHORT_COMPLETE.to_string(),
        domain_id: crate::models::game::domain::ENGINEERING.to_string(),
        event_id: format!("{}:eng", event_id),
        points_delta: 3,
        metadata_json: Some(format!(r#"{{"timer_id":{},"type":"short"}}"#, timer_id)),
    };
    results.push(award_points(pool, user_id, &req1).await?);

    // philosophy +1
    let req2 = AwardPointsRequest {
        world_id: world_id.to_string(),
        source_type: crate::models::game::source_type::TIMER_SHORT_COMPLETE.to_string(),
        domain_id: crate::models::game::domain::PHILOSOPHY.to_string(),
        event_id: format!("{}:phi", event_id),
        points_delta: 1,
        metadata_json: Some(format!(r#"{{"timer_id":{},"type":"short"}}"#, timer_id)),
    };
    results.push(award_points(pool, user_id, &req2).await?);

    Ok(results)
}

/// 计时器完成（长，≥25min）→ 积分入账（engineering +5, philosophy +3）。
pub async fn sync_timer_long_completed(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    event_id: &str,
    timer_id: i64,
) -> Result<Vec<AwardPointsResult>, AppError> {
    let mut results = Vec::with_capacity(2);

    let req1 = AwardPointsRequest {
        world_id: world_id.to_string(),
        source_type: crate::models::game::source_type::TIMER_LONG_COMPLETE.to_string(),
        domain_id: crate::models::game::domain::ENGINEERING.to_string(),
        event_id: format!("{}:eng", event_id),
        points_delta: 5,
        metadata_json: Some(format!(r#"{{"timer_id":{},"type":"long"}}"#, timer_id)),
    };
    results.push(award_points(pool, user_id, &req1).await?);

    let req2 = AwardPointsRequest {
        world_id: world_id.to_string(),
        source_type: crate::models::game::source_type::TIMER_LONG_COMPLETE.to_string(),
        domain_id: crate::models::game::domain::PHILOSOPHY.to_string(),
        event_id: format!("{}:phi", event_id),
        points_delta: 3,
        metadata_json: Some(format!(r#"{{"timer_id":{},"type":"long"}}"#, timer_id)),
    };
    results.push(award_points(pool, user_id, &req2).await?);

    Ok(results)
}

/// AI 会话单轮 → 积分入账（+5，cs 领域，每日上限 50）。
pub async fn sync_ai_chat_turn(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    event_id: &str,
    session_id: &str,
) -> Result<AwardPointsResult, AppError> {
    let req = AwardPointsRequest {
        world_id: world_id.to_string(),
        source_type: crate::models::game::source_type::AI_CHAT_TURN.to_string(),
        domain_id: crate::models::game::domain::CS.to_string(),
        event_id: event_id.to_string(),
        points_delta: 5,
        metadata_json: Some(format!(r#"{{"session_id":"{}"}}"#, session_id)),
    };
    award_points(pool, user_id, &req).await
}

/// YuanCode 使用 → 积分入账（+8，cs 领域，每日上限 80）。
pub async fn sync_yuancode_used(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    event_id: &str,
    invocation_id: &str,
) -> Result<AwardPointsResult, AppError> {
    let req = AwardPointsRequest {
        world_id: world_id.to_string(),
        source_type: crate::models::game::source_type::YUANCODE_USE.to_string(),
        domain_id: crate::models::game::domain::CS.to_string(),
        event_id: event_id.to_string(),
        points_delta: 8,
        metadata_json: Some(format!(r#"{{"invocation_id":"{}"}}"#, invocation_id)),
    };
    award_points(pool, user_id, &req).await
}

/// 日志记录 → 积分入账（+3，用户选择领域或默认 other，每日上限 30）。
pub async fn sync_journal_record(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    domain_id: &str,
    event_id: &str,
    journal_id: i64,
) -> Result<AwardPointsResult, AppError> {
    let req = AwardPointsRequest {
        world_id: world_id.to_string(),
        source_type: crate::models::game::source_type::JOURNAL_RECORD.to_string(),
        domain_id: domain_id.to_string(),
        event_id: event_id.to_string(),
        points_delta: 3,
        metadata_json: Some(format!(r#"{{"journal_id":{}}}"#, journal_id)),
    };
    award_points(pool, user_id, &req).await
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_today_shanghai_format() {
        let d = today_shanghai();
        assert_eq!(d.len(), 10, "日期格式应为 YYYY-MM-DD");
        assert_eq!(d.chars().nth(4), Some('-'));
        assert_eq!(d.chars().nth(7), Some('-'));
    }

    #[test]
    fn test_now_ms_positive() {
        let t = now_ms();
        assert!(t > 1_700_000_000_000, "时间戳应为毫秒级");
    }

    #[test]
    fn test_rejected_result_construction() {
        let req = AwardPointsRequest {
            world_id: "test".to_string(),
            source_type: "ai_chat_turn".to_string(),
            domain_id: "cs".to_string(),
            event_id: "evt-1".to_string(),
            points_delta: 5,
            metadata_json: None,
        };
        let result = rejected(PointsLogResult::RejectedDailyLimit, &req, "test");
        assert_eq!(result.result, "rejected_daily_limit");
        assert_eq!(result.points_credited, 0);
        assert!(!result.is_duplicate);
    }

    // ========================================================================
    // 建造进度推进：5 个纯算法测试 + 4 个集成测试
    // ========================================================================

    use crate::models::game::GameWorld;

    /// 构造测试用 GameBuilding
    fn make_building(
        id: &str,
        category: &str,
        subtype: &str,
        level: u32,
        progress: f64,
    ) -> GameBuilding {
        GameBuilding {
            id: id.to_string(),
            world_id: "test-world".to_string(),
            building_category: category.to_string(),
            building_subtype: subtype.to_string(),
            name: format!("test-{}", subtype),
            level,
            pos_x: 0.0,
            pos_y: 0.0,
            pos_z: 0.0,
            rotation_y: 0.0,
            status: "building".to_string(),
            build_progress: progress,
            knowledge_domain: "cs".to_string(),
            built_at: Some(0),
            completed_at: None,
        }
    }

    // -------- 5 个 compute_build_progress_deltas 纯算法测试 --------

    #[test]
    fn test_compute_build_progress_deltas_empty_or_nonpositive() {
        // 空 buildings
        let empty: Vec<GameBuilding> = vec![];
        assert!(compute_build_progress_deltas(&empty, 10).is_empty());

        // points_delta = 0
        let b = make_building("b1", "house", "thatch_cottage", 1, 0.0);
        assert!(compute_build_progress_deltas(&[b], 0).is_empty());

        // points_delta = -5
        let b = make_building("b1", "house", "thatch_cottage", 1, 0.0);
        assert!(compute_build_progress_deltas(&[b], -5).is_empty());
    }

    #[test]
    fn test_compute_build_progress_deltas_single_building_partial() {
        // thatch_cottage required=100 (level 1), progress=0%
        // points_delta=10 → delta = (10/100) × 100 = 10%
        let b = make_building("b1", "house", "thatch_cottage", 1, 0.0);
        let updates = compute_build_progress_deltas(&[b], 10);

        assert_eq!(updates.len(), 1);
        let u = &updates[0];
        assert_eq!(u.building_id, "b1");
        assert!((u.progress_before - 0.0).abs() < 1e-9);
        assert!((u.progress_after - 10.0).abs() < 1e-9);
        assert!((u.delta - 10.0).abs() < 1e-9);
        assert!(!u.completed);
    }

    #[test]
    fn test_compute_build_progress_deltas_capping_at_100() {
        // thatch_cottage required=100, progress=95%, points_delta=10
        // remaining = 100 - 95 = 5, share = 10, actual_share = min(10, 5) = 5
        // delta = (5/100) × 100 = 5%, progress_after = 95 + 5 = 100% (capping)
        let b = make_building("b1", "house", "thatch_cottage", 1, 95.0);
        let updates = compute_build_progress_deltas(&[b], 10);

        assert_eq!(updates.len(), 1);
        let u = &updates[0];
        assert!((u.progress_after - 100.0).abs() < 1e-9);
        assert!(u.completed);
    }

    #[test]
    fn test_compute_build_progress_deltas_multiple_buildings_proportional() {
        // 2 个建筑：
        //   b1 = thatch_cottage (required=100, progress=0, remaining=100)
        //   b2 = wooden_house   (required=250, progress=0, remaining=250)
        //   total_remaining = 350
        // points_delta = 20:
        //   b1 share = 20 × (100/350) = 5.7142...
        //   b2 share = 20 × (250/350) = 14.2857...
        //   delta_b1 = (5.7142.../100) × 100 = 5.7142...%
        //   delta_b2 = (14.2857.../250) × 100 = 5.7142...%
        let b1 = make_building("b1", "house", "thatch_cottage", 1, 0.0);
        let b2 = make_building("b2", "house", "wooden_house", 1, 0.0);
        let updates = compute_build_progress_deltas(&[b1, b2], 20);

        assert_eq!(updates.len(), 2);
        let u1 = &updates[0];
        let u2 = &updates[1];
        // 两建筑的 delta 百分比应相等（按比例分摊后转回百分比相同）
        assert!((u1.delta - u2.delta).abs() < 1e-6, "delta 应按比例相等");
        assert!(!u1.completed);
        assert!(!u2.completed);
        // delta ≈ 5.7143%
        assert!(
            (u1.delta - 5.714285714).abs() < 1e-4,
            "delta 应约为 5.7143%, 实际 = {}",
            u1.delta
        );
    }

    #[test]
    fn test_compute_build_progress_deltas_all_completed_returns_empty() {
        // 所有建筑 progress >= 100 → 应被跳过
        let b1 = make_building("b1", "house", "thatch_cottage", 1, 100.0);
        let b2 = make_building("b2", "house", "wooden_house", 1, 105.0);
        let updates = compute_build_progress_deltas(&[b1, b2], 10);
        assert!(updates.is_empty());
    }

    // -------- 4 个 update_build_progress 集成测试 --------

    /// 建立内存 SQLite + 迁移 + 创建测试 world
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::db::migrations::run_migrations(&pool).await.unwrap();

        let world = GameWorld {
            id: "test-world".to_string(),
            user_id: 1,
            player_name: "tester".to_string(),
            civilization_level: 1,
            realm_major: "mortal".to_string(),
            realm_minor: "early".to_string(),
            dao_foundation: "white".to_string(),
            total_xp: 0,
            realm_xp: 0,
            map_width: 100,
            map_height: 100,
            created_at: 0,
            updated_at: 0,
        };
        game_repo::insert_world(&pool, &world).await.unwrap();

        pool
    }

    #[tokio::test]
    async fn test_update_build_progress_single_building() {
        let pool = setup_test_db().await;

        // 插入 1 个 thatch_cottage (required=100, progress=0)
        let b = make_building("b1", "house", "thatch_cottage", 1, 0.0);
        game_repo::insert_building(&pool, &b).await.unwrap();

        // 推进 10 积分 → delta = 10%
        let updates = update_build_progress(&pool, 1, "test-world", "cs", 10)
            .await
            .unwrap();

        assert_eq!(updates.len(), 1);
        assert!((updates[0].delta - 10.0).abs() < 1e-9);
        assert!(!updates[0].completed);

        // 验证 DB 中 build_progress 增长到 10
        let updated = game_repo::get_building(&pool, "b1").await.unwrap().unwrap();
        assert!((updated.build_progress - 10.0).abs() < 1e-9);
        assert_eq!(updated.status, "building");
    }

    #[tokio::test]
    async fn test_update_build_progress_completion() {
        let pool = setup_test_db().await;

        // 插入 1 个 thatch_cottage (required=100, progress=95)
        let b = make_building("b1", "house", "thatch_cottage", 1, 95.0);
        game_repo::insert_building(&pool, &b).await.unwrap();

        // 推进 10 积分（应 capping 100% 并完成）
        let updates = update_build_progress(&pool, 1, "test-world", "cs", 10)
            .await
            .unwrap();

        assert_eq!(updates.len(), 1);
        assert!(updates[0].completed);
        assert!((updates[0].progress_after - 100.0).abs() < 1e-9);

        // 验证 DB 中 status='completed' + completed_at 非空
        let updated = game_repo::get_building(&pool, "b1").await.unwrap().unwrap();
        assert_eq!(updated.status, "completed");
        assert!(updated.completed_at.is_some());
        assert!((updated.build_progress - 100.0).abs() < 1e-9);

        // 验证 game_build_history 有 complete 记录
        let history = game_repo::list_build_history_by_building(&pool, "b1", 100)
            .await
            .unwrap();
        assert_eq!(history.len(), 1, "应有一条 complete 历史记录");
        assert_eq!(history[0].event_type, "complete");
        assert_eq!(history[0].building_id, "b1");
    }

    #[tokio::test]
    async fn test_update_build_progress_no_buildings_returns_empty() {
        let pool = setup_test_db().await;

        // 不插入任何建筑
        let updates = update_build_progress(&pool, 1, "test-world", "cs", 10)
            .await
            .unwrap();
        assert!(updates.is_empty());
    }

    #[tokio::test]
    async fn test_update_build_progress_negative_delta_returns_empty() {
        let pool = setup_test_db().await;

        let b = make_building("b1", "house", "thatch_cottage", 1, 0.0);
        game_repo::insert_building(&pool, &b).await.unwrap();

        // 负数 points_delta 应早返回空 vec
        let updates = update_build_progress(&pool, 1, "test-world", "cs", -5)
            .await
            .unwrap();
        assert!(updates.is_empty());
    }
}
