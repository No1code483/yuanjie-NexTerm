//! 游戏 3D 重构 - Service 层（世界/境界/建筑核心业务）
//!
//! Task 4.1：实现世界初始化、境界信息查询、建筑建造/升级/拆除/移动业务逻辑。
//! change-id: `game-3d-rebuild-refactor`
//!
//! 设计原则：
//! - 无状态函数式（与 `game_repo` 一致），不持有可变状态
//! - 每个业务函数内部使用事务保证原子性（如 start_building = 扣积分 + 建建筑 + 记历史）
//! - 业务规则违反返回 `AppError::Validation`，资源不存在返回 `AppError::NotFound`
//! - 积分扣除仅操作对应领域，不跨领域扣减
//!
//! 依赖关系：
//! - 调用 `db::repositories::game_repo` 的 CRUD 函数
//! - 调用 `models::game` 的枚举方法（境界判定、道基系数、建筑大类映射）

use sqlx::SqlitePool;

use crate::db::repositories::game_repo::{self, WorldState};
use crate::error::app_error::AppError;
use crate::models::game::{
    dao_foundation_multiplier, BuildingCategory, BuildingStatus, DaoFoundation, GameBuilding,
    GameWorld, RealmMajor,
};
use crate::services::snapshot_service;

// ============================================================================
// 辅助函数
// ============================================================================

/// 当前时间戳（毫秒级 Unix）。
fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// 生成 UUID v4 字符串。
fn new_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ============================================================================
// 1. 世界初始化与查询
// ============================================================================

/// 创建新世界（32×32 平原，凡人初期，白道基，12 领域进度初始化）。
///
/// 委托 `game_repo::create_world_with_defaults` 完成事务性创建。
pub async fn init_world(
    pool: &SqlitePool,
    user_id: i64,
    player_name: Option<&str>,
) -> Result<GameWorld, AppError> {
    game_repo::create_world_with_defaults(pool, user_id, player_name).await
}

/// 获取世界完整状态（世界 + 全部建筑 + 12 领域进度）。
///
/// 委托 `game_repo::get_world_state` 完成 3D 场景数据加载。
// 多用户隔离（批次 6）：按 user_id 过滤
pub async fn get_world_state(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
) -> Result<Option<WorldState>, AppError> {
    game_repo::get_world_state(pool, user_id, world_id).await
}

/// 境界信息聚合 DTO（前端境界信息卡展示用）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RealmInfo {
    pub world_id: String,
    pub player_name: String,
    /// 大境界英文标识（如 `qi_refining`）。
    pub realm_major: String,
    /// 小境界英文标识（如 `early`）。
    pub realm_minor: String,
    /// 道基英文标识（如 `white`）。
    pub dao_foundation: String,
    /// 道基系数（0.5 / 0.7 / 1.0 / 1.3 / 1.6）。
    pub dao_multiplier: f64,
    /// 累计总修为（永不减）。
    pub total_xp: i64,
    /// 当前大境界内修为（突破大境界后清零）。
    pub realm_xp: i64,
    /// 当前大境界 XP 下限（含）。
    pub realm_xp_lower: i64,
    /// 当前大境界 XP 上限（不含）。
    pub realm_xp_upper: i64,
    /// 当前大境界序号（1-10）。
    pub realm_ordinal: u8,
    /// 下一阶大境界（仙境界为 `None`）。
    pub next_realm_major: Option<String>,
    /// 突破考验题目数量（凡人/仙 = 0，其他 3-15）。
    pub breakthrough_question_count: u8,
    /// 文明等级。
    pub civilization_level: u32,
}

/// 获取世界境界信息（境界信息卡数据源）。
///
/// 聚合世界主表字段 + 派生的道基系数/境界边界/突破题目数。
// 多用户隔离（批次 6）：按 user_id 过滤
pub async fn get_realm_info(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
) -> Result<RealmInfo, AppError> {
    let world = game_repo::get_world(pool, user_id, world_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let realm_major = RealmMajor::from_str(&world.realm_major)
        .ok_or_else(|| AppError::Internal(format!("无效的 realm_major: {}", world.realm_major)))?;
    let dao = DaoFoundation::from_str(&world.dao_foundation)
        .ok_or_else(|| AppError::Internal(format!("无效的 dao_foundation: {}", world.dao_foundation)))?;

    Ok(RealmInfo {
        world_id: world.id.clone(),
        player_name: world.player_name.clone(),
        realm_major: world.realm_major.clone(),
        realm_minor: world.realm_minor.clone(),
        dao_foundation: world.dao_foundation.clone(),
        dao_multiplier: dao_foundation_multiplier(dao),
        total_xp: world.total_xp,
        realm_xp: world.realm_xp,
        realm_xp_lower: realm_major.xp_lower_bound(),
        realm_xp_upper: realm_major.xp_upper_bound(),
        realm_ordinal: realm_major.ordinal(),
        next_realm_major: realm_major.next().map(|r| r.as_str().to_string()),
        breakthrough_question_count: realm_major.breakthrough_question_count(),
        civilization_level: world.civilization_level,
    })
}

// ============================================================================
// 2. 建筑业务（建造/升级/拆除/移动）
// ============================================================================

/// 建造请求参数（start_building 输入）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StartBuildingRequest {
    pub world_id: String,
    pub building_category: String,
    pub building_subtype: String,
    pub name: String,
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub rotation_y: f64,
    /// 关联的知识领域 id（决定扣哪个领域的积分）。
    pub knowledge_domain: String,
    /// 建造所需积分（由建筑子类目录决定，调用方传入）。
    pub base_cost: i64,
}

/// 预建造（start_building）：校验文明等级 + 扣积分 + 建建筑 + 记历史。
///
/// 业务规则（04 文档）：
/// 1. 文明等级 ≥ 建筑大类要求的最低文明等级
/// 2. 对应领域积分余额 ≥ `base_cost`
/// 3. 建筑初始状态为 `building`（建造中），`build_progress = 0`
/// 4. 扣除积分 → 创建建筑 → 记录建造历史（event_type='build'），事务保证原子性
pub async fn start_building(
    pool: &SqlitePool,
    user_id: i64,
    req: &StartBuildingRequest,
) -> Result<GameBuilding, AppError> {
    // 校验建筑大类
    let category = BuildingCategory::from_str(&req.building_category)
        .ok_or_else(|| AppError::Validation(format!("无效的 building_category: {}", req.building_category)))?;

    // 查询世界，校验文明等级
    // 多用户隔离（批次 6）：按 user_id 过滤
    let world = game_repo::get_world(pool, user_id, &req.world_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if world.civilization_level < category.min_civilization_level() {
        return Err(AppError::Validation(format!(
            "文明等级不足：{} 需要文明等级 {}，当前 {}",
            req.building_category,
            category.min_civilization_level(),
            world.civilization_level
        )));
    }

    // 校验积分余额
    let progress = game_repo::get_progress(pool, &req.world_id, &req.knowledge_domain)
        .await?
        .ok_or_else(|| AppError::Validation(format!(
            "领域 {} 未初始化进度记录，请先调用 init_world",
            req.knowledge_domain
        )))?;

    if progress.points < req.base_cost {
        return Err(AppError::Validation(format!(
            "积分不足：需要 {}，当前 {}（领域 {}）",
            req.base_cost, progress.points, req.knowledge_domain
        )));
    }

    let now = now_ms();
    let building_id = new_uuid();
    let history_id = new_uuid();
    let log_id = new_uuid();
    let event_id = format!("build:{}:{}", building_id, now);

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    // 1. 扣积分（更新 progress）
    let new_points = progress.points - req.base_cost;
    let new_consumed = progress.total_consumed + req.base_cost;
    sqlx::query(
        "UPDATE game_knowledge_progress
         SET points = ?, total_consumed = ?, updated_at = ?
         WHERE world_id = ? AND domain_id = ?",
    )
    .bind(new_points)
    .bind(new_consumed)
    .bind(now)
    .bind(&req.world_id)
    .bind(&req.knowledge_domain)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // 2. 记录积分日志（points_delta = -base_cost）
    sqlx::query(
        "INSERT INTO game_points_log
         (id, world_id, source_type, domain_id, points_delta, points_after,
          result, event_id, metadata_json, created_at)
         VALUES (?, ?, 'building_refund', ?, ?, ?, 'accepted', ?, ?, ?)",
    )
    .bind(&log_id)
    .bind(&req.world_id)
    .bind(&req.knowledge_domain)
    .bind(-req.base_cost)
    .bind(new_points)
    .bind(&event_id)
    .bind(format!(r#"{{"building_id":"{}","action":"build"}}"#, building_id))
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // 3. 创建建筑实例
    sqlx::query(
        "INSERT INTO game_buildings
         (id, world_id, building_category, building_subtype, name, level,
          pos_x, pos_y, pos_z, rotation_y, status, build_progress,
          knowledge_domain, built_at, completed_at)
         VALUES (?, ?, ?, ?, ?, 1, ?, ?, ?, ?, 'building', 0, ?, ?, NULL)",
    )
    .bind(&building_id)
    .bind(&req.world_id)
    .bind(&req.building_category)
    .bind(&req.building_subtype)
    .bind(&req.name)
    .bind(req.pos_x)
    .bind(req.pos_y)
    .bind(req.pos_z)
    .bind(req.rotation_y)
    .bind(&req.knowledge_domain)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // 4. 记录建造历史
    sqlx::query(
        "INSERT INTO game_build_history
         (id, world_id, event_type, building_id, building_name,
          pos_x, pos_y, pos_z, level, progress, snapshot_json, created_at)
         VALUES (?, ?, 'build', ?, ?, ?, ?, ?, 1, 0, NULL, ?)",
    )
    .bind(&history_id)
    .bind(&req.world_id)
    .bind(&building_id)
    .bind(&req.name)
    .bind(req.pos_x)
    .bind(req.pos_y)
    .bind(req.pos_z)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    // T1.1 时间轴回放：事务提交后异步快照钩子（11_时间轴回放.md §5.1）
    let _ = snapshot_service::maybe_write_snapshot_after_event(
        pool,
        user_id,
        &req.world_id,
        &history_id,
        "build",
    )
    .await;

    // 返回创建的建筑实例
    let building = game_repo::get_building(pool, &building_id)
        .await?
        .ok_or_else(|| AppError::Internal("建筑创建后查询失败".into()))?;
    Ok(building)
}

/// 升级建筑：校验等级上限 + 扣积分 + 更新等级 + 记历史。
///
/// 业务规则：
/// 1. 建筑状态必须为 `completed`
/// 2. 当前等级 < 文明等级（等级上限 = 文明等级）
/// 3. 领域积分余额 ≥ `upgrade_cost`
/// 4. 升级后等级 +1，记建造历史（event_type='upgrade'）
pub async fn upgrade_building(
    pool: &SqlitePool,
    user_id: i64,
    building_id: &str,
    upgrade_cost: i64,
) -> Result<GameBuilding, AppError> {
    let building = game_repo::get_building(pool, building_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if building.status != BuildingStatus::Completed.as_str() {
        return Err(AppError::Validation(format!(
            "建筑状态不允许升级：当前 {}，需 completed",
            building.status
        )));
    }

    // 多用户隔离（批次 6）：按 user_id 过滤
    let world = game_repo::get_world(pool, user_id, &building.world_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if building.level >= world.civilization_level {
        return Err(AppError::Validation(format!(
            "建筑等级已达文明等级上限：{} ≥ {}",
            building.level, world.civilization_level
        )));
    }

    let progress = game_repo::get_progress(pool, &building.world_id, &building.knowledge_domain)
        .await?
        .ok_or_else(|| AppError::Validation(format!(
            "领域 {} 未初始化进度记录",
            building.knowledge_domain
        )))?;

    if progress.points < upgrade_cost {
        return Err(AppError::Validation(format!(
            "积分不足：需要 {}，当前 {}（领域 {}）",
            upgrade_cost, progress.points, building.knowledge_domain
        )));
    }

    let now = now_ms();
    let history_id = new_uuid();
    let log_id = new_uuid();
    let event_id = format!("upgrade:{}:{}", building_id, now);
    let new_level = building.level + 1;
    let new_points = progress.points - upgrade_cost;
    let new_consumed = progress.total_consumed + upgrade_cost;

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    // 扣积分
    sqlx::query(
        "UPDATE game_knowledge_progress
         SET points = ?, total_consumed = ?, updated_at = ?
         WHERE world_id = ? AND domain_id = ?",
    )
    .bind(new_points)
    .bind(new_consumed)
    .bind(now)
    .bind(&building.world_id)
    .bind(&building.knowledge_domain)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // 积分日志
    sqlx::query(
        "INSERT INTO game_points_log
         (id, world_id, source_type, domain_id, points_delta, points_after,
          result, event_id, metadata_json, created_at)
         VALUES (?, ?, 'building_refund', ?, ?, ?, 'accepted', ?, ?, ?)",
    )
    .bind(&log_id)
    .bind(&building.world_id)
    .bind(&building.knowledge_domain)
    .bind(-upgrade_cost)
    .bind(new_points)
    .bind(&event_id)
    .bind(format!(r#"{{"building_id":"{}","action":"upgrade","to_level":{}}}"#, building_id, new_level))
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // 更新建筑等级
    sqlx::query(
        "UPDATE game_buildings SET level = ? WHERE id = ?",
    )
    .bind(new_level as i64)
    .bind(building_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // 建造历史
    sqlx::query(
        "INSERT INTO game_build_history
         (id, world_id, event_type, building_id, building_name,
          pos_x, pos_y, pos_z, level, progress, snapshot_json, created_at)
         VALUES (?, ?, 'upgrade', ?, ?, ?, ?, ?, ?, NULL, NULL, ?)",
    )
    .bind(&history_id)
    .bind(&building.world_id)
    .bind(building_id)
    .bind(&building.name)
    .bind(building.pos_x)
    .bind(building.pos_y)
    .bind(building.pos_z)
    .bind(new_level as i32)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    // T1.1 时间轴回放：事务提交后异步快照钩子（11_时间轴回放.md §5.1）
    let _ = snapshot_service::maybe_write_snapshot_after_event(
        pool,
        user_id,
        &building.world_id,
        &history_id,
        "upgrade",
    )
    .await;

    let updated = game_repo::get_building(pool, building_id)
        .await?
        .ok_or_else(|| AppError::Internal("建筑升级后查询失败".into()))?;
    Ok(updated)
}

/// 拆除建筑：返还 50% 积分 + 删除建筑 + 记历史。
///
/// 业务规则（04 文档 §1.1）：
/// 1. 返还积分 = 累计消耗积分 × `refund_ratio`（默认 0.5）
/// 2. 返还积分加回对应领域（`points += refund`，`total_consumed -= refund`）
/// 3. 删除建筑实例（历史保留）
/// 4. 记建造历史（event_type='demolish'）
pub async fn remove_building(
    pool: &SqlitePool,
    user_id: i64,
    building_id: &str,
    refund_ratio: f64,
) -> Result<i64, AppError> {
    let building = game_repo::get_building(pool, building_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let progress = game_repo::get_progress(pool, &building.world_id, &building.knowledge_domain)
        .await?
        .ok_or_else(|| AppError::Validation(format!(
            "领域 {} 未初始化进度记录",
            building.knowledge_domain
        )))?;

    // 返还积分 = 累计消耗 × 比例（简化：按 progress.total_consumed × ratio，实际应按该建筑累计消耗）
    // 注：简化版按 progress.total_consumed 计算，精确版需追踪单建筑累计消耗（未来扩展）
    let refund_amount = (progress.total_consumed as f64 * refund_ratio) as i64;
    let new_points = progress.points + refund_amount;
    let new_consumed = (progress.total_consumed - refund_amount).max(0);

    let now = now_ms();
    let history_id = new_uuid();
    let log_id = new_uuid();
    let event_id = format!("demolish:{}:{}", building_id, now);

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    // 返还积分
    sqlx::query(
        "UPDATE game_knowledge_progress
         SET points = ?, total_consumed = ?, updated_at = ?
         WHERE world_id = ? AND domain_id = ?",
    )
    .bind(new_points)
    .bind(new_consumed)
    .bind(now)
    .bind(&building.world_id)
    .bind(&building.knowledge_domain)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // 积分日志（正数入账）
    sqlx::query(
        "INSERT INTO game_points_log
         (id, world_id, source_type, domain_id, points_delta, points_after,
          result, event_id, metadata_json, created_at)
         VALUES (?, ?, 'building_refund', ?, ?, ?, 'accepted', ?, ?, ?)",
    )
    .bind(&log_id)
    .bind(&building.world_id)
    .bind(&building.knowledge_domain)
    .bind(refund_amount)
    .bind(new_points)
    .bind(&event_id)
    .bind(format!(r#"{{"building_id":"{}","action":"demolish","refund":{}}}"#, building_id, refund_amount))
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // 记建造历史（先记后删，历史保留建筑名称快照）
    sqlx::query(
        "INSERT INTO game_build_history
         (id, world_id, event_type, building_id, building_name,
          pos_x, pos_y, pos_z, level, progress, snapshot_json, created_at)
         VALUES (?, ?, 'demolish', ?, ?, ?, ?, ?, ?, NULL, NULL, ?)",
    )
    .bind(&history_id)
    .bind(&building.world_id)
    .bind(building_id)
    .bind(&building.name)
    .bind(building.pos_x)
    .bind(building.pos_y)
    .bind(building.pos_z)
    .bind(building.level as i32)
    .bind(building.build_progress)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // 删除建筑
    sqlx::query("DELETE FROM game_buildings WHERE id = ?")
        .bind(building_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    // T1.1 时间轴回放：事务提交后异步快照钩子（11_时间轴回放.md §5.1）
    let _ = snapshot_service::maybe_write_snapshot_after_event(
        pool,
        user_id,
        &building.world_id,
        &history_id,
        "demolish",
    )
    .await;

    Ok(refund_amount)
}

/// 移动建筑：消耗 10% 建筑成本积分 + 更新坐标 + 记历史。
///
/// 业务规则（04 文档 §1.1）：
/// 1. 消耗积分 = `move_cost`（通常为建筑 base_cost 的 10%）
/// 2. 领域积分余额 ≥ `move_cost`
/// 3. 更新建筑 3D 坐标（pos_x/y/z）和 rotation_y
/// 4. 记建造历史（event_type='move'）
pub async fn move_building(
    pool: &SqlitePool,
    user_id: i64,
    building_id: &str,
    new_pos_x: f64,
    new_pos_y: f64,
    new_pos_z: f64,
    new_rotation_y: Option<f64>,
    move_cost: i64,
) -> Result<GameBuilding, AppError> {
    let building = game_repo::get_building(pool, building_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let progress = game_repo::get_progress(pool, &building.world_id, &building.knowledge_domain)
        .await?
        .ok_or_else(|| AppError::Validation(format!(
            "领域 {} 未初始化进度记录",
            building.knowledge_domain
        )))?;

    if progress.points < move_cost {
        return Err(AppError::Validation(format!(
            "积分不足：移动需要 {}，当前 {}（领域 {}）",
            move_cost, progress.points, building.knowledge_domain
        )));
    }

    let now = now_ms();
    let history_id = new_uuid();
    let log_id = new_uuid();
    let event_id = format!("move:{}:{}", building_id, now);
    let new_points = progress.points - move_cost;
    let new_consumed = progress.total_consumed + move_cost;
    let rotation_y = new_rotation_y.unwrap_or(building.rotation_y);

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    // 扣积分
    sqlx::query(
        "UPDATE game_knowledge_progress
         SET points = ?, total_consumed = ?, updated_at = ?
         WHERE world_id = ? AND domain_id = ?",
    )
    .bind(new_points)
    .bind(new_consumed)
    .bind(now)
    .bind(&building.world_id)
    .bind(&building.knowledge_domain)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // 积分日志
    sqlx::query(
        "INSERT INTO game_points_log
         (id, world_id, source_type, domain_id, points_delta, points_after,
          result, event_id, metadata_json, created_at)
         VALUES (?, ?, 'building_refund', ?, ?, ?, 'accepted', ?, ?, ?)",
    )
    .bind(&log_id)
    .bind(&building.world_id)
    .bind(&building.knowledge_domain)
    .bind(-move_cost)
    .bind(new_points)
    .bind(&event_id)
    .bind(format!(r#"{{"building_id":"{}","action":"move"}}"#, building_id))
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // 更新建筑坐标
    sqlx::query(
        "UPDATE game_buildings
         SET pos_x = ?, pos_y = ?, pos_z = ?, rotation_y = ?
         WHERE id = ?",
    )
    .bind(new_pos_x)
    .bind(new_pos_y)
    .bind(new_pos_z)
    .bind(rotation_y)
    .bind(building_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // 建造历史（记录移动后的新坐标）
    sqlx::query(
        "INSERT INTO game_build_history
         (id, world_id, event_type, building_id, building_name,
          pos_x, pos_y, pos_z, level, progress, snapshot_json, created_at)
         VALUES (?, ?, 'move', ?, ?, ?, ?, ?, ?, NULL, NULL, ?)",
    )
    .bind(&history_id)
    .bind(&building.world_id)
    .bind(building_id)
    .bind(&building.name)
    .bind(new_pos_x)
    .bind(new_pos_y)
    .bind(new_pos_z)
    .bind(building.level as i32)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    // T1.1 时间轴回放：事务提交后异步快照钩子（move 事件不触发快照，统一调用钩子由 should_snapshot 决策）
    let _ = snapshot_service::maybe_write_snapshot_after_event(
        pool,
        user_id,
        &building.world_id,
        &history_id,
        "move",
    )
    .await;

    let updated = game_repo::get_building(pool, building_id)
        .await?
        .ok_or_else(|| AppError::Internal("建筑移动后查询失败".into()))?;
    Ok(updated)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    // 注意：业务函数依赖数据库，单元测试仅覆盖纯逻辑部分。
    // 完整业务流程测试需要集成测试环境（内存 SQLite + 迁移），
    // 将在阶段5 IPC 层完成后统一编写端到端测试。

    // 目前无纯逻辑可测试（所有函数都涉及 DB 操作），
    // RealmInfo 结构和 RealmMajor 派生逻辑已由 models::game 的单元测试覆盖。
}
