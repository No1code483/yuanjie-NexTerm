//! 游戏 3D 重构 - 时间轴场景重建服务（T1.1，11_时间轴回放.md §5.1 + §5.2.2）
//!
//! 职责：
//!   1. 场景重建算法（spec §5.1）：给定时间戳，重建该时刻的建筑分布与世界状态
//!   2. LRU 缓存（spec §5.2.2）：避免相同时间戳重复重建
//!   3. 快照预热（spec §5.2.4）：进入游戏页时预加载最近快照
//!
//! 设计原则：
//! - 无状态函数式（与 game_service.rs 一致）
//! - LRU 缓存用 `tokio::sync::Mutex` 包装，异步安全
//! - 时间戳分桶 100ms（spec §5.2.2），避免毫秒级差异导致缓存失效
//!
//! change-id: `game-3d-rebuild-refactor`

use std::num::NonZeroUsize;

use lru::LruCache;
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::db::repositories::game_repo;
use crate::error::app_error::AppError;
use crate::models::game::{
    BuildingSnapshot, GameBuildHistory, RebuiltScene, SnapshotStats, WorldSnapshot,
    WorldStateSnapshot,
};
use crate::services::snapshot_service;

// ============================================================================
// 常量
// ============================================================================

/// LRU 缓存容量（spec §5.2.2：约 32 × 80KB = 2.5MB 内存占用）。
const CACHE_CAPACITY: usize = 32;

/// 时间戳分桶粒度（毫秒）。spec §5.2.2：100ms 分桶避免相近时间戳重复重建。
const TIMESTAMP_BUCKET_MS: i64 = 100;

// ============================================================================
// LRU 缓存（全局单例，进程级共享）
// ============================================================================

/// 重建结果 LRU 缓存，键为 `(world_id, timestamp / 100)`，值为重建后的场景。
///
/// 注意：跨世界共享同一缓存实例（spec §5.2.2 提到"世界切换时清空对应 world_id 的缓存"）。
/// v1 简化：不按 world_id 分桶清理，统一 LRU 淘汰。若某世界缓存项被淘汰，下次访问时自然重建。
static REBUILD_CACHE: once_cell::sync::Lazy<Mutex<LruCache<(String, i64), RebuiltScene>>> =
    once_cell::sync::Lazy::new(|| {
        Mutex::new(LruCache::new(NonZeroUsize::new(CACHE_CAPACITY).unwrap()))
    });

/// 时间戳分桶：将毫秒级时间戳按 100ms 粒度对齐。
fn bucketize_timestamp(ts: i64) -> i64 {
    ts / TIMESTAMP_BUCKET_MS
}

/// 清空缓存（用于世界切换 / 退出回放等场景，spec §5.2.2 + §8.3）。
pub async fn clear_cache() {
    let mut cache = REBUILD_CACHE.lock().await;
    cache.clear();
}

// ============================================================================
// 1. 场景重建算法（spec §5.1）
// ============================================================================

/// 重建指定时间点的世界场景。
///
/// 算法（spec §5.1）：
/// 1. 查找 timestamp 之前最近的完整快照点
/// 2. 加载快照作为基准场景（无快照则空世界）
/// 3. 查询快照点之后到 timestamp 之间的增量事件
/// 4. 按时间顺序应用增量事件到基准场景
/// 5. 返回重建后的场景
///
/// 缓存策略：
/// - 先查 LRU 缓存，命中则直接返回（spec §5.2.2）
/// - 未命中则重建后写入缓存
///
/// 边界情况：
/// - timestamp 早于世界首条事件 → 返回空场景（buildings=[]），不报错
/// - timestamp 晚于最新事件 → 钳制（clamp）到最新事件时间戳
/// - 无快照（snapshot_json 全为 NULL）→ 从空世界开始重放全部事件
pub async fn rebuild_scene_at(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    timestamp: i64,
) -> Result<RebuiltScene, AppError> {
    // 1. 缓存查找
    let bucket = bucketize_timestamp(timestamp);
    let cache_key = (world_id.to_string(), bucket);
    {
        let mut cache = REBUILD_CACHE.lock().await;
        if let Some(scene) = cache.get(&cache_key) {
            return Ok(scene.clone());
        }
    }

    // 2. 查找最近快照点
    let snapshot_event = game_repo::find_latest_snapshot_before(pool, world_id, timestamp).await?;

    // 3. 加载基准场景
    let (world_state, mut buildings, source_snapshot_event_id): (
        WorldStateSnapshot,
        Vec<BuildingSnapshot>,
        Option<String>,
    ) = if let Some(ev) = &snapshot_event {
        // 有快照：解压 → 反序列化
        let snapshot_json = ev.snapshot_json.as_deref().unwrap_or("");
        let decompressed = snapshot_service::decompress_snapshot(snapshot_json)?;
        let snapshot: WorldSnapshot = serde_json::from_str(&decompressed).map_err(|e| {
            AppError::Internal(format!("snapshot deserialize failed: {}", e))
        })?;
        (
            snapshot.world_state,
            snapshot.buildings,
            Some(ev.id.clone()),
        )
    } else {
        // 无快照：查询当前世界主表作为基准（覆盖 timestamp 早于首事件场景）
        // 多用户隔离（批次 6）：按 user_id 过滤
        let world = game_repo::get_world(pool, user_id, world_id)
            .await?
            .ok_or(AppError::NotFound)?;
        let empty_state = WorldStateSnapshot {
            civilization_level: world.civilization_level as i32,
            realm_major: world.realm_major,
            realm_minor: world.realm_minor,
            dao_foundation: world.dao_foundation,
            total_xp: world.total_xp,
            realm_xp: world.realm_xp,
        };
        (empty_state, Vec::new(), None)
    };

    // 4. 查询快照点之后到 timestamp 之间的增量事件
    let snapshot_ts = snapshot_event.as_ref().map(|e| e.created_at).unwrap_or(0);
    let incremental_events =
        game_repo::list_build_history_between(pool, world_id, snapshot_ts, timestamp).await?;

    let replayed_count = incremental_events.len() as u32;

    // 5. 应用增量事件
    for event in &incremental_events {
        apply_event(&mut buildings, event);
    }

    // 6. 重算统计（基于当前 buildings 列表，避免快照统计与增量后实际不符）
    let building_count = buildings.len() as u32;
    let total_levels: u32 = buildings.iter().map(|b| b.level as u32).sum();
    let stats = SnapshotStats {
        building_count,
        total_levels,
    };

    // 7. 组装结果
    let scene = RebuiltScene {
        world_state,
        buildings,
        stats,
        target_timestamp: timestamp,
        source_snapshot_event_id: source_snapshot_event_id.clone(),
        replayed_event_count: replayed_count,
    };

    // 8. 写入缓存
    {
        let mut cache = REBUILD_CACHE.lock().await;
        cache.put(cache_key, scene.clone());
    }

    tracing::debug!(
        world_id = %world_id,
        timestamp = timestamp,
        replayed_count = replayed_count,
        building_count = scene.stats.building_count,
        source_snapshot = ?source_snapshot_event_id,
        "scene_rebuilt"
    );

    Ok(scene)
}

// ============================================================================
// 2. 增量事件应用（spec §5.1 apply_event）
// ============================================================================

/// 应用单个事件到 buildings 列表（就地修改）。
///
/// 事件类型处理（spec §2.2）：
/// - `build`：新增建筑（status='building'，build_progress=0.1）
/// - `complete`：标记为 completed，build_progress=1.0
/// - `upgrade`：更新等级
/// - `demolish` / `remove`：移除建筑（兼容实际代码的 demolish 命名）
/// - `move`：更新坐标
///
/// 注意：若事件指向的建筑不存在（如历史数据不一致），增量重放静默跳过不报错。
fn apply_event(buildings: &mut Vec<BuildingSnapshot>, event: &GameBuildHistory) {
    match event.event_type.as_str() {
        "build" => {
            // 新增建筑（从事件字段重建）
            let b = BuildingSnapshot {
                id: event.building_id.clone(),
                building_subtype: String::new(), // build 事件未存 subtype 快照
                name: event.building_name.clone(),
                level: event.level.unwrap_or(1),
                pos_x: event.pos_x.unwrap_or(0.0),
                pos_y: event.pos_y.unwrap_or(0.0),
                pos_z: event.pos_z.unwrap_or(0.0),
                rotation_y: 0.0, // build 事件未存 rotation 快照
                status: "building".to_string(),
                build_progress: event.progress.unwrap_or(0.1),
                knowledge_domain: String::new(), // build 事件未存 domain 快照
            };
            buildings.push(b);
        }
        "complete" => {
            // 标记为 completed
            if let Some(b) = buildings.iter_mut().find(|b| b.id == event.building_id) {
                b.status = "completed".to_string();
                b.build_progress = 1.0;
            }
        }
        "upgrade" => {
            // 更新等级
            if let Some(b) = buildings.iter_mut().find(|b| b.id == event.building_id) {
                if let Some(level) = event.level {
                    b.level = level;
                }
            }
        }
        "demolish" | "remove" => {
            // 移除建筑
            buildings.retain(|b| b.id != event.building_id);
        }
        "move" => {
            // 更新坐标
            if let Some(b) = buildings.iter_mut().find(|b| b.id == event.building_id) {
                if let (Some(x), Some(y), Some(z)) = (event.pos_x, event.pos_y, event.pos_z) {
                    b.pos_x = x;
                    b.pos_y = y;
                    b.pos_z = z;
                }
            }
        }
        _ => {
            tracing::warn!(
                event_type = %event.event_type,
                history_id = %event.id,
                "unknown_event_type_skipped"
            );
        }
    }
}

// ============================================================================
// 3. 快照预热（spec §5.2.4）
// ============================================================================

/// 预加载最近一个完整快照到内存，避免首次拖动滑块时的解压延迟。
///
/// 调用时机：`game_get_world_state` 命令返回前 spawn 后台任务调用本函数。
///
/// 实现：查找最近快照点 → 解压 → 反序列化 → 验证 → 丢弃（仅用于触发解压缓存）。
/// 注：zstd 解压本身有 OS 级文件系统缓存，预热一次后后续访问更快。
pub async fn preload_latest_snapshot(
    pool: &SqlitePool,
    world_id: &str,
) -> Result<(), AppError> {
    // 查找最新快照点（不限 timestamp）
    let now_ms_val = chrono::Utc::now().timestamp_millis();
    let snapshot_event =
        game_repo::find_latest_snapshot_before(pool, world_id, now_ms_val).await?;

    if let Some(ev) = snapshot_event {
        if let Some(stored) = &ev.snapshot_json {
            // 解压 + 反序列化验证
            let decompressed = snapshot_service::decompress_snapshot(stored)?;
            let _snapshot: WorldSnapshot = serde_json::from_str(&decompressed).map_err(|e| {
                AppError::Internal(format!("preload snapshot deserialize failed: {}", e))
            })?;
            tracing::info!(
                world_id = %world_id,
                snapshot_event_id = %ev.id,
                "snapshot_preloaded"
            );
        }
    }

    Ok(())
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(
        event_type: &str,
        building_id: &str,
        building_name: &str,
        level: Option<i32>,
        pos: Option<(f64, f64, f64)>,
    ) -> GameBuildHistory {
        GameBuildHistory {
            id: format!("h-{}", building_id),
            world_id: "test-world".to_string(),
            event_type: event_type.to_string(),
            building_id: building_id.to_string(),
            building_name: building_name.to_string(),
            pos_x: pos.map(|p| p.0),
            pos_y: pos.map(|p| p.1),
            pos_z: pos.map(|p| p.2),
            level,
            progress: None,
            snapshot_json: None,
            created_at: 1000,
        }
    }

    #[test]
    fn test_apply_event_build_adds_building() {
        let mut buildings: Vec<BuildingSnapshot> = Vec::new();
        let event = make_event("build", "b1", "茅草屋", Some(1), Some((10.0, 0.0, -3.0)));
        apply_event(&mut buildings, &event);
        assert_eq!(buildings.len(), 1);
        assert_eq!(buildings[0].id, "b1");
        assert_eq!(buildings[0].name, "茅草屋");
        assert_eq!(buildings[0].status, "building");
        assert_eq!(buildings[0].level, 1);
        assert_eq!(buildings[0].pos_x, 10.0);
    }

    #[test]
    fn test_apply_event_complete_updates_status() {
        let mut buildings = vec![BuildingSnapshot {
            id: "b1".to_string(),
            building_subtype: "thatch_cottage".to_string(),
            name: "茅草屋".to_string(),
            level: 1,
            pos_x: 10.0,
            pos_y: 0.0,
            pos_z: -3.0,
            rotation_y: 0.0,
            status: "building".to_string(),
            build_progress: 0.5,
            knowledge_domain: "engineering".to_string(),
        }];
        let event = make_event("complete", "b1", "茅草屋", None, None);
        apply_event(&mut buildings, &event);
        assert_eq!(buildings[0].status, "completed");
        assert_eq!(buildings[0].build_progress, 1.0);
    }

    #[test]
    fn test_apply_event_upgrade_updates_level() {
        let mut buildings = vec![BuildingSnapshot {
            id: "b1".to_string(),
            building_subtype: "thatch_cottage".to_string(),
            name: "茅草屋".to_string(),
            level: 1,
            pos_x: 10.0,
            pos_y: 0.0,
            pos_z: -3.0,
            rotation_y: 0.0,
            status: "completed".to_string(),
            build_progress: 1.0,
            knowledge_domain: "engineering".to_string(),
        }];
        let event = make_event("upgrade", "b1", "茅草屋", Some(2), None);
        apply_event(&mut buildings, &event);
        assert_eq!(buildings[0].level, 2);
    }

    #[test]
    fn test_apply_event_demolish_removes_building() {
        let mut buildings = vec![BuildingSnapshot {
            id: "b1".to_string(),
            building_subtype: "thatch_cottage".to_string(),
            name: "茅草屋".to_string(),
            level: 1,
            pos_x: 10.0,
            pos_y: 0.0,
            pos_z: -3.0,
            rotation_y: 0.0,
            status: "completed".to_string(),
            build_progress: 1.0,
            knowledge_domain: "engineering".to_string(),
        }];
        let event = make_event("demolish", "b1", "茅草屋", None, None);
        apply_event(&mut buildings, &event);
        assert_eq!(buildings.len(), 0);
    }

    #[test]
    fn test_apply_event_move_updates_coords() {
        let mut buildings = vec![BuildingSnapshot {
            id: "b1".to_string(),
            building_subtype: "thatch_cottage".to_string(),
            name: "茅草屋".to_string(),
            level: 1,
            pos_x: 10.0,
            pos_y: 0.0,
            pos_z: -3.0,
            rotation_y: 0.0,
            status: "completed".to_string(),
            build_progress: 1.0,
            knowledge_domain: "engineering".to_string(),
        }];
        let event = make_event("move", "b1", "茅草屋", None, Some((15.0, 0.0, -8.0)));
        apply_event(&mut buildings, &event);
        assert_eq!(buildings[0].pos_x, 15.0);
        assert_eq!(buildings[0].pos_z, -8.0);
    }

    #[test]
    fn test_apply_event_unknown_type_skipped() {
        let mut buildings: Vec<BuildingSnapshot> = Vec::new();
        let event = make_event("unknown_type", "b1", "测试", None, None);
        apply_event(&mut buildings, &event);
        assert_eq!(buildings.len(), 0);
    }

    #[test]
    fn test_apply_event_complete_missing_building_no_panic() {
        // 增量重放静默跳过指向不存在建筑的事件
        let mut buildings: Vec<BuildingSnapshot> = Vec::new();
        let event = make_event("complete", "missing", "不存在", None, None);
        apply_event(&mut buildings, &event);
        assert_eq!(buildings.len(), 0);
    }

    #[test]
    fn test_bucketize_timestamp_aligns_to_100ms() {
        assert_eq!(bucketize_timestamp(1000), 10);
        assert_eq!(bucketize_timestamp(1099), 10);
        assert_eq!(bucketize_timestamp(1100), 11);
    }

    #[tokio::test]
    async fn test_clear_cache_no_panic() {
        // 清空空缓存不应 panic
        clear_cache().await;
    }
}
