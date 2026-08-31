//! 游戏 3D 重构 - Repository 层
//!
//! 对应 `07_数据库设计.md` v2.0 的 10 张表 CRUD。
//! change-id: `game-3d-rebuild-refactor`
//!
//! 设计原则：
//! - 使用 `sqlx::query_as::<_, T>` + `FromRow` 自动映射行→结构体
//! - 时间戳统一用毫秒级 Unix（`chrono::Utc::now().timestamp_millis()`）
//! - 每日计数器日期用 Asia/Shanghai 时区的 `'YYYY-MM-DD'`
//! - 幂等插入用 `INSERT OR IGNORE` / `ON CONFLICT DO UPDATE`
//! - 删除世界依赖外键 `ON DELETE CASCADE` 自动清理关联数据

use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::game::{
    GameBreakthroughRecord, GameBuildHistory, GameBuilding, GameDailyLimitCounter,
    GameKbCategoryMapping, GameKnowledgeDomain, GameKnowledgeProgress, GamePointsLog,
    GamePointsSourceConfig, GameWorld,
};

// ============================================================================
// 辅助函数
// ============================================================================

/// 当前时间戳（毫秒级 Unix）。
fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// 当前日期（Asia/Shanghai 时区，`'YYYY-MM-DD'`）。
///
/// 采用 UTC+8 固定偏移（不依赖系统时区，避免部署环境差异）。
fn today_shanghai() -> String {
    let now = chrono::Utc::now();
    let beijing = now + chrono::Duration::hours(8);
    beijing.format("%Y-%m-%d").to_string()
}

// ============================================================================
// 1. game_knowledge_domains（字典表：12 个知识领域）
// ============================================================================

/// 列出全部 12 个知识领域，按 `sort_order` 排序。
pub async fn list_domains(pool: &SqlitePool) -> Result<Vec<GameKnowledgeDomain>, AppError> {
    sqlx::query_as::<_, GameKnowledgeDomain>(
        "SELECT id, name, name_en, description, building_category, sort_order
         FROM game_knowledge_domains
         ORDER BY sort_order ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 按 id 查询单个知识领域。
pub async fn get_domain(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<GameKnowledgeDomain>, AppError> {
    sqlx::query_as::<_, GameKnowledgeDomain>(
        "SELECT id, name, name_en, description, building_category, sort_order
         FROM game_knowledge_domains
         WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

// ============================================================================
// 2. game_points_source_config（字典表：11 种积分来源配置）
// ============================================================================

/// 列出全部积分来源配置。
pub async fn list_source_configs(pool: &SqlitePool) -> Result<Vec<GamePointsSourceConfig>, AppError> {
    sqlx::query_as::<_, GamePointsSourceConfig>(
        "SELECT source_type, default_domain_id, points_per_event, daily_limit, description, enabled, updated_at
         FROM game_points_source_config
         ORDER BY source_type ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 按 `source_type` 查询单个积分来源配置。
pub async fn get_source_config(
    pool: &SqlitePool,
    source_type: &str,
) -> Result<Option<GamePointsSourceConfig>, AppError> {
    sqlx::query_as::<_, GamePointsSourceConfig>(
        "SELECT source_type, default_domain_id, points_per_event, daily_limit, description, enabled, updated_at
         FROM game_points_source_config
         WHERE source_type = ?",
    )
    .bind(source_type)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

/// 更新积分来源配置（points_per_event / daily_limit / enabled / description）。
/// `source_type` 必须已存在（由迁移种子数据保证 11 条）。
pub async fn update_source_config(
    pool: &SqlitePool,
    source_type: &str,
    points_per_event: i64,
    daily_limit: Option<i64>,
    enabled: bool,
    description: Option<&str>,
) -> Result<(), AppError> {
    let now = now_ms();
    sqlx::query(
        "UPDATE game_points_source_config
         SET points_per_event = ?, daily_limit = ?, enabled = ?, description = ?, updated_at = ?
         WHERE source_type = ?",
    )
    .bind(points_per_event)
    .bind(daily_limit)
    .bind(if enabled { 1i64 } else { 0i64 })
    .bind(description)
    .bind(now)
    .bind(source_type)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

// ============================================================================
// 3. game_worlds（核心实体：世界/文明主表）
// ============================================================================

/// 插入新世界。调用方负责生成 `id`（UUID v4）与时间戳。
///
/// 多用户隔离（批次 6）：INSERT 显式写入 `world.user_id`，确保世界归属正确用户。
pub async fn insert_world(pool: &SqlitePool, world: &GameWorld) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO game_worlds
         (id, user_id, player_name, civilization_level, realm_major, realm_minor, dao_foundation,
          total_xp, realm_xp, map_width, map_height, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&world.id)
    .bind(world.user_id)
    .bind(&world.player_name)
    .bind(world.civilization_level as i64)
    .bind(&world.realm_major)
    .bind(&world.realm_minor)
    .bind(&world.dao_foundation)
    .bind(world.total_xp)
    .bind(world.realm_xp)
    .bind(world.map_width as i64)
    .bind(world.map_height as i64)
    .bind(world.created_at)
    .bind(world.updated_at)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

/// 按 id 查询世界。
///
/// 多用户隔离（批次 6）：必须同时匹配 `user_id`，防止跨用户读取世界数据。
pub async fn get_world(
    pool: &SqlitePool,
    user_id: i64,
    id: &str,
) -> Result<Option<GameWorld>, AppError> {
    sqlx::query_as::<_, GameWorld>(
        "SELECT id, user_id, player_name, civilization_level, realm_major, realm_minor,
                dao_foundation, total_xp, realm_xp, map_width, map_height, created_at, updated_at
         FROM game_worlds
         WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

/// 列出指定用户的所有世界，按 `updated_at` 倒序（「最近游玩」排序）。
///
/// 多用户隔离（批次 6）：按 `user_id` 过滤，用户只能看到自己的世界。
pub async fn list_worlds(pool: &SqlitePool, user_id: i64) -> Result<Vec<GameWorld>, AppError> {
    sqlx::query_as::<_, GameWorld>(
        "SELECT id, user_id, player_name, civilization_level, realm_major, realm_minor,
                dao_foundation, total_xp, realm_xp, map_width, map_height, created_at, updated_at
         FROM game_worlds
         WHERE user_id = ?
         ORDER BY updated_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 更新世界状态（境界/道基/修为/文明等级/地图尺寸/玩家名）。
/// 自动更新 `updated_at`。
///
/// 多用户隔离（批次 6）：WHERE 同时匹配 `id` 和 `world.user_id`，防止跨用户修改。
pub async fn update_world(pool: &SqlitePool, world: &GameWorld) -> Result<(), AppError> {
    let now = now_ms();
    sqlx::query(
        "UPDATE game_worlds
         SET player_name = ?, civilization_level = ?, realm_major = ?, realm_minor = ?,
             dao_foundation = ?, total_xp = ?, realm_xp = ?, map_width = ?, map_height = ?,
             updated_at = ?
         WHERE id = ? AND user_id = ?",
    )
    .bind(&world.player_name)
    .bind(world.civilization_level as i64)
    .bind(&world.realm_major)
    .bind(&world.realm_minor)
    .bind(&world.dao_foundation)
    .bind(world.total_xp)
    .bind(world.realm_xp)
    .bind(world.map_width as i64)
    .bind(world.map_height as i64)
    .bind(now)
    .bind(&world.id)
    .bind(world.user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

/// 删除世界。依赖外键 `ON DELETE CASCADE` 自动清理关联数据
/// （buildings / progress / breakthrough_records / build_history / points_log / daily_limit）。
///
/// 多用户隔离（批次 6）：必须同时匹配 `user_id`，防止跨用户删除世界。
pub async fn delete_world(pool: &SqlitePool, user_id: i64, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM game_worlds WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

// ============================================================================
// 4. game_buildings（核心实体：建筑实例）
// ============================================================================

/// 插入新建筑实例。
pub async fn insert_building(pool: &SqlitePool, b: &GameBuilding) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO game_buildings
         (id, world_id, building_category, building_subtype, name, level,
          pos_x, pos_y, pos_z, rotation_y, status, build_progress,
          knowledge_domain, built_at, completed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&b.id)
    .bind(&b.world_id)
    .bind(&b.building_category)
    .bind(&b.building_subtype)
    .bind(&b.name)
    .bind(b.level as i64)
    .bind(b.pos_x)
    .bind(b.pos_y)
    .bind(b.pos_z)
    .bind(b.rotation_y)
    .bind(&b.status)
    .bind(b.build_progress)
    .bind(&b.knowledge_domain)
    .bind(b.built_at)
    .bind(b.completed_at)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

/// 按 id 查询单个建筑。
pub async fn get_building(pool: &SqlitePool, id: &str) -> Result<Option<GameBuilding>, AppError> {
    sqlx::query_as::<_, GameBuilding>(
        "SELECT id, world_id, building_category, building_subtype, name, level,
                pos_x, pos_y, pos_z, rotation_y, status, build_progress,
                knowledge_domain, built_at, completed_at
         FROM game_buildings
         WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

/// 列出某世界的全部建筑（3D 场景加载最高频路径）。
pub async fn list_buildings_by_world(
    pool: &SqlitePool,
    world_id: &str,
) -> Result<Vec<GameBuilding>, AppError> {
    sqlx::query_as::<_, GameBuilding>(
        "SELECT id, world_id, building_category, building_subtype, name, level,
                pos_x, pos_y, pos_z, rotation_y, status, build_progress,
                knowledge_domain, built_at, completed_at
         FROM game_buildings
         WHERE world_id = ?",
    )
    .bind(world_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 列出某世界指定状态的建筑（如筛选「建造中」轮询进度）。
pub async fn list_buildings_by_status(
    pool: &SqlitePool,
    world_id: &str,
    status: &str,
) -> Result<Vec<GameBuilding>, AppError> {
    sqlx::query_as::<_, GameBuilding>(
        "SELECT id, world_id, building_category, building_subtype, name, level,
                pos_x, pos_y, pos_z, rotation_y, status, build_progress,
                knowledge_domain, built_at, completed_at
         FROM game_buildings
         WHERE world_id = ? AND status = ?",
    )
    .bind(world_id)
    .bind(status)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 列出某世界某领域指定状态的建筑（推进建造进度用）。
///
/// TODO: 当前仅有 (world_id) 和 (status) 单列索引，
/// 未来可加 (world_id, knowledge_domain, status) 复合索引优化。
pub async fn list_buildings_by_world_domain_status(
    pool: &SqlitePool,
    world_id: &str,
    domain_id: &str,
    status: &str,
) -> Result<Vec<GameBuilding>, AppError> {
    sqlx::query_as::<_, GameBuilding>(
        "SELECT id, world_id, building_category, building_subtype, name, level,
                pos_x, pos_y, pos_z, rotation_y, status, build_progress,
                knowledge_domain, built_at, completed_at
         FROM game_buildings
         WHERE world_id = ? AND knowledge_domain = ? AND status = ?",
    )
    .bind(world_id)
    .bind(domain_id)
    .bind(status)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 更新建筑（等级/坐标/旋转/状态/进度/时间戳）。
pub async fn update_building(pool: &SqlitePool, b: &GameBuilding) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE game_buildings
         SET building_category = ?, building_subtype = ?, name = ?, level = ?,
             pos_x = ?, pos_y = ?, pos_z = ?, rotation_y = ?,
             status = ?, build_progress = ?, knowledge_domain = ?,
             built_at = ?, completed_at = ?
         WHERE id = ?",
    )
    .bind(&b.building_category)
    .bind(&b.building_subtype)
    .bind(&b.name)
    .bind(b.level as i64)
    .bind(b.pos_x)
    .bind(b.pos_y)
    .bind(b.pos_z)
    .bind(b.rotation_y)
    .bind(&b.status)
    .bind(b.build_progress)
    .bind(&b.knowledge_domain)
    .bind(b.built_at)
    .bind(b.completed_at)
    .bind(&b.id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

/// 删除建筑（仅删除建筑实例，历史记录由 `game_build_history` 保留）。
pub async fn delete_building(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM game_buildings WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

// ============================================================================
// 5. game_knowledge_progress（业务表：世界在各领域的积分进度）
// ============================================================================

/// 插入或更新进度记录（`UNIQUE(world_id, domain_id)` 冲突时更新）。
/// 调用方需保证 `points = total_earned - total_consumed` 且 `points >= 0`。
pub async fn upsert_progress(
    pool: &SqlitePool,
    p: &GameKnowledgeProgress,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO game_knowledge_progress
         (id, world_id, domain_id, points, level, total_earned, total_consumed, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(world_id, domain_id) DO UPDATE SET
             points = excluded.points,
             level = excluded.level,
             total_earned = excluded.total_earned,
             total_consumed = excluded.total_consumed,
             updated_at = excluded.updated_at",
    )
    .bind(&p.id)
    .bind(&p.world_id)
    .bind(&p.domain_id)
    .bind(p.points)
    .bind(p.level)
    .bind(p.total_earned)
    .bind(p.total_consumed)
    .bind(p.created_at)
    .bind(p.updated_at)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

/// 查询某世界在某领域的进度。
pub async fn get_progress(
    pool: &SqlitePool,
    world_id: &str,
    domain_id: &str,
) -> Result<Option<GameKnowledgeProgress>, AppError> {
    sqlx::query_as::<_, GameKnowledgeProgress>(
        "SELECT id, world_id, domain_id, points, level, total_earned, total_consumed, created_at, updated_at
         FROM game_knowledge_progress
         WHERE world_id = ? AND domain_id = ?",
    )
    .bind(world_id)
    .bind(domain_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

/// 列出某世界在全部 12 个领域的进度（加载世界时批量获取）。
pub async fn list_progress_by_world(
    pool: &SqlitePool,
    world_id: &str,
) -> Result<Vec<GameKnowledgeProgress>, AppError> {
    sqlx::query_as::<_, GameKnowledgeProgress>(
        "SELECT id, world_id, domain_id, points, level, total_earned, total_consumed, created_at, updated_at
         FROM game_knowledge_progress
         WHERE world_id = ?",
    )
    .bind(world_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 为世界初始化 12 个领域的进度记录（创建世界时调用）。
/// 使用 `INSERT OR IGNORE` 保证幂等（已存在的记录不受影响）。
pub async fn ensure_progress_for_world(
    pool: &SqlitePool,
    world_id: &str,
) -> Result<(), AppError> {
    let now = now_ms();
    let domains = list_domains(pool).await?;
    for d in domains {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT OR IGNORE INTO game_knowledge_progress
             (id, world_id, domain_id, points, level, total_earned, total_consumed, created_at, updated_at)
             VALUES (?, ?, ?, 0, 1, 0, 0, ?, ?)",
        )
        .bind(&id)
        .bind(world_id)
        .bind(&d.id)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    }
    Ok(())
}

// ============================================================================
// 6. game_breakthrough_records（业务表：境界突破流水）
// ============================================================================

/// 插入突破记录。
pub async fn insert_breakthrough_record(
    pool: &SqlitePool,
    r: &GameBreakthroughRecord,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO game_breakthrough_records
         (id, world_id, from_realm, to_realm, score, dao_foundation_awarded,
          result, questions_json, answers_json, ai_review, weakness_json, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&r.id)
    .bind(&r.world_id)
    .bind(&r.from_realm)
    .bind(&r.to_realm)
    .bind(r.score)
    .bind(&r.dao_foundation_awarded)
    .bind(&r.result)
    .bind(&r.questions_json)
    .bind(&r.answers_json)
    .bind(&r.ai_review)
    .bind(&r.weakness_json)
    .bind(r.created_at)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

/// 列出某世界的突破记录（按时间倒序，支撑「突破历程」时间线）。
pub async fn list_breakthrough_records(
    pool: &SqlitePool,
    world_id: &str,
    limit: i64,
) -> Result<Vec<GameBreakthroughRecord>, AppError> {
    sqlx::query_as::<_, GameBreakthroughRecord>(
        "SELECT id, world_id, from_realm, to_realm, score, dao_foundation_awarded,
                result, questions_json, answers_json, ai_review, weakness_json, created_at
         FROM game_breakthrough_records
         WHERE world_id = ?
         ORDER BY created_at DESC
         LIMIT ?",
    )
    .bind(world_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

// ============================================================================
// 7. game_build_history（业务表：建造历史/时间轴回放）
// ============================================================================

/// 插入建造历史事件。
pub async fn insert_build_history(
    pool: &SqlitePool,
    h: &GameBuildHistory,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO game_build_history
         (id, world_id, event_type, building_id, building_name,
          pos_x, pos_y, pos_z, level, progress, snapshot_json, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&h.id)
    .bind(&h.world_id)
    .bind(&h.event_type)
    .bind(&h.building_id)
    .bind(&h.building_name)
    .bind(h.pos_x)
    .bind(h.pos_y)
    .bind(h.pos_z)
    .bind(h.level)
    .bind(h.progress)
    .bind(&h.snapshot_json)
    .bind(h.created_at)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

/// 列出某世界的建造历史（按时间倒序，支撑时间轴滑块回放）。
pub async fn list_build_history(
    pool: &SqlitePool,
    world_id: &str,
    limit: i64,
) -> Result<Vec<GameBuildHistory>, AppError> {
    sqlx::query_as::<_, GameBuildHistory>(
        "SELECT id, world_id, event_type, building_id, building_name,
                pos_x, pos_y, pos_z, level, progress, snapshot_json, created_at
         FROM game_build_history
         WHERE world_id = ?
         ORDER BY created_at DESC
         LIMIT ?",
    )
    .bind(world_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 列出某建筑的建造历史（支撑单建筑升级/移动历程）。
pub async fn list_build_history_by_building(
    pool: &SqlitePool,
    building_id: &str,
    limit: i64,
) -> Result<Vec<GameBuildHistory>, AppError> {
    sqlx::query_as::<_, GameBuildHistory>(
        "SELECT id, world_id, event_type, building_id, building_name,
                pos_x, pos_y, pos_z, level, progress, snapshot_json, created_at
         FROM game_build_history
         WHERE building_id = ?
         ORDER BY created_at DESC
         LIMIT ?",
    )
    .bind(building_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

// ============================================================================
// 8. game_kb_category_mapping（桥接表：KB 分类→游戏领域映射）
// ============================================================================

/// 插入或更新 KB 分类映射（`category_id` 为主键，冲突时更新）。
pub async fn upsert_category_mapping(
    pool: &SqlitePool,
    m: &GameKbCategoryMapping,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO game_kb_category_mapping
         (category_id, domain_id, mapped_at, mapped_by, confidence)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(category_id) DO UPDATE SET
             domain_id = excluded.domain_id,
             mapped_at = excluded.mapped_at,
             mapped_by = excluded.mapped_by,
             confidence = excluded.confidence",
    )
    .bind(m.category_id)
    .bind(&m.domain_id)
    .bind(m.mapped_at)
    .bind(&m.mapped_by)
    .bind(m.confidence)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

/// 按 KB 分类 id 查询映射。
pub async fn get_category_mapping(
    pool: &SqlitePool,
    category_id: i64,
) -> Result<Option<GameKbCategoryMapping>, AppError> {
    sqlx::query_as::<_, GameKbCategoryMapping>(
        "SELECT category_id, domain_id, mapped_at, mapped_by, confidence
         FROM game_kb_category_mapping
         WHERE category_id = ?",
    )
    .bind(category_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

/// 按领域 id 反查关联的 KB 分类（领域详情页展示来源）。
pub async fn list_mappings_by_domain(
    pool: &SqlitePool,
    domain_id: &str,
) -> Result<Vec<GameKbCategoryMapping>, AppError> {
    sqlx::query_as::<_, GameKbCategoryMapping>(
        "SELECT category_id, domain_id, mapped_at, mapped_by, confidence
         FROM game_kb_category_mapping
         WHERE domain_id = ?",
    )
    .bind(domain_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 列出所有 KB 分类映射（设置页左栏展示用，前端与 kb_categories LEFT JOIN）。
pub async fn list_all_mappings(
    pool: &SqlitePool,
) -> Result<Vec<GameKbCategoryMapping>, AppError> {
    sqlx::query_as::<_, GameKbCategoryMapping>(
        "SELECT category_id, domain_id, mapped_at, mapped_by, confidence
         FROM game_kb_category_mapping
         ORDER BY mapped_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 删除 KB 分类映射。
pub async fn delete_category_mapping(
    pool: &SqlitePool,
    category_id: i64,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM game_kb_category_mapping WHERE category_id = ?")
        .bind(category_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

// ============================================================================
// 9. game_points_log（审计表：积分变更日志，幂等 event_id）
// ============================================================================

/// 插入积分日志。
///
/// `event_id` 设 `UNIQUE` 约束，同一业务事件重复触发会因约束冲突返回错误，
/// 调用方应先调用 `find_points_log_by_event` 检查幂等性，或捕获 `AppError::Database` 并视为已处理。
pub async fn insert_points_log(pool: &SqlitePool, log: &GamePointsLog) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO game_points_log
         (id, world_id, source_type, domain_id, points_delta, points_after,
          result, event_id, metadata_json, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&log.id)
    .bind(&log.world_id)
    .bind(&log.source_type)
    .bind(&log.domain_id)
    .bind(log.points_delta)
    .bind(log.points_after)
    .bind(&log.result)
    .bind(&log.event_id)
    .bind(&log.metadata_json)
    .bind(log.created_at)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

/// 按 `event_id` 查询已存在的积分日志（幂等检查）。
pub async fn find_points_log_by_event(
    pool: &SqlitePool,
    event_id: &str,
) -> Result<Option<GamePointsLog>, AppError> {
    sqlx::query_as::<_, GamePointsLog>(
        "SELECT id, world_id, source_type, domain_id, points_delta, points_after,
                result, event_id, metadata_json, created_at
         FROM game_points_log
         WHERE event_id = ?",
    )
    .bind(event_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

/// 列出某世界的积分流水（按时间倒序）。
pub async fn list_points_log(
    pool: &SqlitePool,
    world_id: &str,
    limit: i64,
) -> Result<Vec<GamePointsLog>, AppError> {
    sqlx::query_as::<_, GamePointsLog>(
        "SELECT id, world_id, source_type, domain_id, points_delta, points_after,
                result, event_id, metadata_json, created_at
         FROM game_points_log
         WHERE world_id = ?
         ORDER BY created_at DESC
         LIMIT ?",
    )
    .bind(world_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 列出某世界在某领域的积分明细（按时间倒序）。
pub async fn list_points_log_by_domain(
    pool: &SqlitePool,
    world_id: &str,
    domain_id: &str,
    limit: i64,
) -> Result<Vec<GamePointsLog>, AppError> {
    sqlx::query_as::<_, GamePointsLog>(
        "SELECT id, world_id, source_type, domain_id, points_delta, points_after,
                result, event_id, metadata_json, created_at
         FROM game_points_log
         WHERE world_id = ? AND domain_id = ?
         ORDER BY created_at DESC
         LIMIT ?",
    )
    .bind(world_id)
    .bind(domain_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 每日每领域积分聚合行（用于趋势图数据源）。
///
/// `date_str` 为 Asia/Shanghai 时区的 `'YYYY-MM-DD'`。
/// `total_delta` 为该领域当日所有 `result='accepted'` 的积分变动总和。
#[derive(sqlx::FromRow)]
pub struct DailyPointsAgg {
    pub domain_id: String,
    pub date_str: String,
    pub total_delta: i64,
}

/// 聚合查询：近 `days` 天每领域每日入账积分（仅 `result='accepted'`）。
///
/// 返回扁平行列表，由调用方组装为 `Record<domain_id, Vec<i64>>`。
/// 日期对齐 Asia/Shanghai 时区（`+8 hours`），`start_ms` 为查询起点（毫秒级 Unix）。
pub async fn aggregate_daily_points_by_domain(
    pool: &SqlitePool,
    world_id: &str,
    start_ms: i64,
) -> Result<Vec<DailyPointsAgg>, AppError> {
    // created_at 为毫秒级 Unix；strftime 需秒级 → 除以 1000。
    // '+8 hours' 将 UTC 对齐为 Asia/Shanghai 时区。
    sqlx::query_as::<_, DailyPointsAgg>(
        "SELECT domain_id,
                strftime('%Y-%m-%d', created_at / 1000, 'unixepoch', '+8 hours') AS date_str,
                SUM(points_delta) AS total_delta
         FROM game_points_log
         WHERE world_id = ? AND created_at >= ? AND result = 'accepted'
         GROUP BY domain_id, date_str
         ORDER BY date_str ASC",
    )
    .bind(world_id)
    .bind(start_ms)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

// ============================================================================
// 10. game_daily_limit_counter（限流表：每日上限计数器）
// ============================================================================

/// 插入或更新每日计数器（`UNIQUE(world_id, counter_date, domain_id, source_type)` 冲突时累加）。
///
/// `delta` 为本次新增次数（通常为 1）。返回更新后的 `current_count`。
pub async fn upsert_daily_counter(
    pool: &SqlitePool,
    world_id: &str,
    domain_id: &str,
    source_type: &str,
    delta: i64,
) -> Result<i64, AppError> {
    let now = now_ms();
    let today = today_shanghai();
    let id = uuid::Uuid::new_v4().to_string();

    // ON CONFLICT 时累加 current_count
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO game_daily_limit_counter
         (id, world_id, counter_date, domain_id, source_type, current_count, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(world_id, counter_date, domain_id, source_type) DO UPDATE SET
             current_count = current_count + excluded.current_count,
             updated_at = excluded.updated_at
         RETURNING current_count",
    )
    .bind(&id)
    .bind(world_id)
    .bind(&today)
    .bind(domain_id)
    .bind(source_type)
    .bind(delta)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(row.0)
}

/// 查询当日某领域+来源的计数器（入账前限流校验）。
pub async fn get_daily_counter(
    pool: &SqlitePool,
    world_id: &str,
    domain_id: &str,
    source_type: &str,
) -> Result<Option<GameDailyLimitCounter>, AppError> {
    let today = today_shanghai();
    sqlx::query_as::<_, GameDailyLimitCounter>(
        "SELECT id, world_id, counter_date, domain_id, source_type, current_count, updated_at
         FROM game_daily_limit_counter
         WHERE world_id = ? AND counter_date = ? AND domain_id = ? AND source_type = ?",
    )
    .bind(world_id)
    .bind(&today)
    .bind(domain_id)
    .bind(source_type)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

/// 列出某世界当日全部计数器（批量限流校验）。
pub async fn list_daily_counters(
    pool: &SqlitePool,
    world_id: &str,
) -> Result<Vec<GameDailyLimitCounter>, AppError> {
    let today = today_shanghai();
    sqlx::query_as::<_, GameDailyLimitCounter>(
        "SELECT id, world_id, counter_date, domain_id, source_type, current_count, updated_at
         FROM game_daily_limit_counter
         WHERE world_id = ? AND counter_date = ?",
    )
    .bind(world_id)
    .bind(&today)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

// ============================================================================
// 11. 便利函数（跨表聚合 + 默认初始化）
// ============================================================================

/// 世界状态聚合 DTO（跨表查询结果）。
///
/// 用于一次性加载 3D 场景所需的全部数据：
/// 世界主表 + 全部建筑 + 12 领域进度。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorldState {
    pub world: GameWorld,
    pub buildings: Vec<GameBuilding>,
    pub progress: Vec<GameKnowledgeProgress>,
}

/// 创建新世界并初始化 12 个领域的进度记录（便利函数）。
///
/// - 生成 UUID v4 作为 `id`
/// - 应用默认值（玩家名「无名修士」、境界 mortal:early、道基 white、地图 32×32）
/// - 插入 `game_worlds` 记录
/// - 为 12 个领域各插入一条初始 progress 记录（points=0, level=1）
///
/// 返回创建的 `GameWorld`。调用方可继续调用 `get_world_state` 获取完整状态。
pub async fn create_world_with_defaults(
    pool: &SqlitePool,
    user_id: i64,
    player_name: Option<&str>,
) -> Result<GameWorld, AppError> {
    use crate::models::game::{
        DEFAULT_CIVILIZATION_LEVEL, DEFAULT_DAO_FOUNDATION, DEFAULT_MAP_HEIGHT,
        DEFAULT_MAP_WIDTH, DEFAULT_PLAYER_NAME, DEFAULT_REALM_MAJOR, DEFAULT_REALM_MINOR,
    };

    let now = now_ms();
    let id = uuid::Uuid::new_v4().to_string();
    let world = GameWorld {
        id,
        user_id,
        player_name: player_name.unwrap_or(DEFAULT_PLAYER_NAME).to_string(),
        civilization_level: DEFAULT_CIVILIZATION_LEVEL,
        realm_major: DEFAULT_REALM_MAJOR.as_str().to_string(),
        realm_minor: DEFAULT_REALM_MINOR.as_str().to_string(),
        dao_foundation: DEFAULT_DAO_FOUNDATION.as_str().to_string(),
        total_xp: 0,
        realm_xp: 0,
        map_width: DEFAULT_MAP_WIDTH,
        map_height: DEFAULT_MAP_HEIGHT,
        created_at: now,
        updated_at: now,
    };

    // 事务保证「世界 + 12 条 progress」原子性
    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    // 插入世界
    sqlx::query(
        "INSERT INTO game_worlds
         (id, user_id, player_name, civilization_level, realm_major, realm_minor, dao_foundation,
          total_xp, realm_xp, map_width, map_height, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&world.id)
    .bind(world.user_id)
    .bind(&world.player_name)
    .bind(world.civilization_level as i64)
    .bind(&world.realm_major)
    .bind(&world.realm_minor)
    .bind(&world.dao_foundation)
    .bind(world.total_xp)
    .bind(world.realm_xp)
    .bind(world.map_width as i64)
    .bind(world.map_height as i64)
    .bind(world.created_at)
    .bind(world.updated_at)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // 为 12 个领域各插入一条初始 progress（INSERT OR IGNORE 幂等）
    let domains = sqlx::query_as::<_, GameKnowledgeDomain>(
        "SELECT id, name, name_en, description, building_category, sort_order
         FROM game_knowledge_domains ORDER BY sort_order ASC",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    for d in domains {
        let progress_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT OR IGNORE INTO game_knowledge_progress
             (id, world_id, domain_id, points, level, total_earned, total_consumed, created_at, updated_at)
             VALUES (?, ?, ?, 0, 1, 0, 0, ?, ?)",
        )
        .bind(&progress_id)
        .bind(&world.id)
        .bind(&d.id)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;
    }

    tx.commit().await.map_err(AppError::Database)?;
    Ok(world)
}

/// 一次性加载世界的完整状态（世界 + 全部建筑 + 12 领域进度）。
///
/// 3D 场景加载的最高频路径：单次调用返回渲染所需的全部数据，
/// 避免前端发起 3 次独立请求。
///
/// 返回 `None` 表示世界不存在。
pub async fn get_world_state(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
) -> Result<Option<WorldState>, AppError> {
    let world = match get_world(pool, user_id, world_id).await? {
        Some(w) => w,
        None => return Ok(None),
    };

    let buildings = list_buildings_by_world(pool, world_id).await?;
    let progress = list_progress_by_world(pool, world_id).await?;

    Ok(Some(WorldState {
        world,
        buildings,
        progress,
    }))
}

/// 世界列表摘要（带统计信息）。
///
/// 用于存档列表展示，避免逐个查询建筑数与积分总和。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct WorldSummary {
    pub id: String,
    pub player_name: String,
    pub civilization_level: i64,
    pub realm_major: String,
    pub realm_minor: String,
    pub dao_foundation: String,
    pub total_xp: i64,
    pub realm_xp: i64,
    /// 建筑总数（含所有状态）。
    pub building_count: i64,
    /// 已建成建筑数（status='completed'）。
    pub completed_building_count: i64,
    /// 12 领域累计积分总和。
    pub total_points: i64,
    pub updated_at: i64,
    pub created_at: i64,
}

/// 列出指定用户所有世界的摘要（含建筑数、积分总和），按 `updated_at` 倒序。
///
/// 通过 LEFT JOIN + 聚合一次性返回，避免 N+1 查询。
///
/// 多用户隔离（批次 6）：按 `w.user_id` 过滤，用户只能看到自己的世界摘要。
pub async fn list_world_summaries(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<WorldSummary>, AppError> {
    sqlx::query_as::<_, WorldSummary>(
        "SELECT
            w.id,
            w.player_name,
            w.civilization_level,
            w.realm_major,
            w.realm_minor,
            w.dao_foundation,
            w.total_xp,
            w.realm_xp,
            w.updated_at,
            w.created_at,
            COUNT(b.id) AS building_count,
            SUM(CASE WHEN b.status = 'completed' THEN 1 ELSE 0 END) AS completed_building_count,
            COALESCE(SUM(p.points), 0) AS total_points
         FROM game_worlds w
         LEFT JOIN game_buildings b ON b.world_id = w.id
         LEFT JOIN game_knowledge_progress p ON p.world_id = w.id
         WHERE w.user_id = ?
         GROUP BY w.id
         ORDER BY w.updated_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

// ============================================================================
// 9. 时间轴回放辅助查询（T1.1，11_时间轴回放.md §5.1 场景重建算法）
// ============================================================================

/// 列出某世界在时间范围 `[from, to]` 内的建造历史（按时间正序，支撑增量重放）。
///
/// 注意与 `list_build_history` 区别：前者倒序 + limit，本函数正序 + 时间范围过滤。
pub async fn list_build_history_between(
    pool: &SqlitePool,
    world_id: &str,
    from: i64,
    to: i64,
) -> Result<Vec<GameBuildHistory>, AppError> {
    sqlx::query_as::<_, GameBuildHistory>(
        "SELECT id, world_id, event_type, building_id, building_name,
                pos_x, pos_y, pos_z, level, progress, snapshot_json, created_at
         FROM game_build_history
         WHERE world_id = ? AND created_at > ? AND created_at <= ?
         ORDER BY created_at ASC",
    )
    .bind(world_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

/// 查找某世界在 `timestamp` 之前最近的完整快照点（`snapshot_json IS NOT NULL`）。
///
/// 用于场景重建算法 §5.1 步骤 1：从目标时间向前找最近的快照作为基准场景。
/// 返回 `None` 表示无快照（timestamp 早于首条快照或无历史记录）。
pub async fn find_latest_snapshot_before(
    pool: &SqlitePool,
    world_id: &str,
    timestamp: i64,
) -> Result<Option<GameBuildHistory>, AppError> {
    sqlx::query_as::<_, GameBuildHistory>(
        "SELECT id, world_id, event_type, building_id, building_name,
                pos_x, pos_y, pos_z, level, progress, snapshot_json, created_at
         FROM game_build_history
         WHERE world_id = ? AND created_at <= ? AND snapshot_json IS NOT NULL
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(world_id)
    .bind(timestamp)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

/// 统计某世界自 `since_ts` 之后已记录的建造历史事件数（含 `since_ts` 时刻不算）。
///
/// 用于快照存储决策：当自上次快照后累计 ≥10 条事件时触发周期性快照。
pub async fn count_events_since(
    pool: &SqlitePool,
    world_id: &str,
    since_ts: i64,
) -> Result<i64, AppError> {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM game_build_history
         WHERE world_id = ? AND created_at > ?",
    )
    .bind(world_id)
    .bind(since_ts)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(count.0)
}

/// 回填 `snapshot_json` 字段到指定的建造历史记录。
///
/// 用于快照存储：service 层先 `insert_build_history`（snapshot_json=NULL），
/// 随后判断需要快照时调用本函数回填压缩后的快照数据。
pub async fn update_build_history_snapshot(
    pool: &SqlitePool,
    history_id: &str,
    snapshot_json: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE game_build_history SET snapshot_json = ? WHERE id = ?",
    )
    .bind(snapshot_json)
    .bind(history_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

/// 列出某世界的全部建造历史（按时间正序，无 limit）。
///
/// 用于快照预热 + 时间轴初始化。注意：单世界事件数 > 5000 时建议改用分页。
pub async fn list_all_build_history_asc(
    pool: &SqlitePool,
    world_id: &str,
) -> Result<Vec<GameBuildHistory>, AppError> {
    sqlx::query_as::<_, GameBuildHistory>(
        "SELECT id, world_id, event_type, building_id, building_name,
                pos_x, pos_y, pos_z, level, progress, snapshot_json, created_at
         FROM game_build_history
         WHERE world_id = ?
         ORDER BY created_at ASC",
    )
    .bind(world_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
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
        // 格式 YYYY-MM-DD，长度 10
        assert_eq!(d.len(), 10, "日期格式应为 YYYY-MM-DD");
        assert_eq!(d.chars().nth(4), Some('-'));
        assert_eq!(d.chars().nth(7), Some('-'));
    }

    #[test]
    fn test_now_ms_positive() {
        let t = now_ms();
        // 2026-07-16 之后的时间戳（ms）应远大于 1_700_000_000_000
        assert!(t > 1_700_000_000_000, "时间戳应为毫秒级");
    }
}
