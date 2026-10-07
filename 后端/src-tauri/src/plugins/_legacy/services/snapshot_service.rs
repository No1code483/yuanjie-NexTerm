//! 游戏 3D 重构 - 快照生成服务（T1.1，11_时间轴回放.md §3 + §5.2.1）
//!
//! 职责：
//!   1. 生成完整世界状态快照（WorldSnapshot）
//!   2. zstd 压缩 / 解压快照 JSON（spec §5.2.1）
//!   3. 快照存储决策（spec §2.3：每 10 事件 + 重要节点）
//!
//! 设计原则：
//! - 无状态函数式（与 game_service.rs 一致）
//! - 快照生成事务外执行（避免长事务持有连接）
//! - 压缩级别 3（spec §5.2.1，平衡速度与压缩比，约 7:1）
//!
//! change-id: `game-3d-rebuild-refactor`

use sqlx::SqlitePool;

use crate::db::repositories::game_repo;
use crate::error::app_error::AppError;
use crate::models::game::{
    BuildingSnapshot, GameWorld, SnapshotMeta, SnapshotStats, SnapshotTriggerReason, WorldSnapshot,
    WorldStateSnapshot,
};

// ============================================================================
// 常量
// ============================================================================

/// zstd 压缩级别（spec §5.2.1：平衡速度与压缩比）。
const SNAPSHOT_COMPRESSION_LEVEL: i32 = 3;

/// 周期性快照触发阈值：自上次快照后累计事件数达到此值时触发快照。
/// spec §2.3：每 10 个事件一次完整快照。
pub const PERIODIC_SNAPSHOT_EVENT_THRESHOLD: i64 = 10;

// ============================================================================
// 1. 快照生成
// ============================================================================

/// 生成当前世界状态完整快照。
///
/// 调用时机：
/// - 每周期（PERIODIC_SNAPSHOT_EVENT_THRESHOLD 个事件后）
/// - 重要节点（complete/upgrade/remove）
/// - 世界初始化首条事件
///
/// 流程（spec §3.2）：
/// 1. 查询世界主表 → WorldStateSnapshot
/// 2. 查询所有建筑 → Vec<BuildingSnapshot>
/// 3. 计算统计（建筑数 + 等级总和）→ SnapshotStats
/// 4. 组装 WorldSnapshot 结构
///
/// 参数：
/// - `event_id`：触发快照的事件 ID
/// - `event_seq`：该事件在当前世界事件流中的序号（从 1 开始）
/// - `trigger_reason`：快照触发原因
pub async fn generate_snapshot(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    event_id: &str,
    event_seq: u32,
    trigger_reason: SnapshotTriggerReason,
) -> Result<WorldSnapshot, AppError> {
    // 1. 查询世界主表
    // 多用户隔离（批次 6）：按 user_id 过滤
    let world: GameWorld = game_repo::get_world(pool, user_id, world_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let world_state = WorldStateSnapshot {
        civilization_level: world.civilization_level as i32,
        realm_major: world.realm_major,
        realm_minor: world.realm_minor,
        dao_foundation: world.dao_foundation,
        total_xp: world.total_xp,
        realm_xp: world.realm_xp,
    };

    // 2. 查询所有建筑（含所有状态：planning/building/completed/ruined）
    let buildings_db = game_repo::list_buildings_by_world(pool, world_id).await?;

    let buildings: Vec<BuildingSnapshot> = buildings_db
        .iter()
        .map(|b| BuildingSnapshot {
            id: b.id.clone(),
            building_subtype: b.building_subtype.clone(),
            name: b.name.clone(),
            level: b.level as i32,
            pos_x: b.pos_x,
            pos_y: b.pos_y,
            pos_z: b.pos_z,
            rotation_y: b.rotation_y,
            status: b.status.clone(),
            build_progress: b.build_progress,
            knowledge_domain: b.knowledge_domain.clone(),
        })
        .collect();

    // 3. 统计
    let building_count = buildings.len() as u32;
    let total_levels: u32 = buildings.iter().map(|b| b.level as u32).sum();
    let stats = SnapshotStats {
        building_count,
        total_levels,
    };

    // 4. 组装快照
    let now_ms_val = chrono::Utc::now().timestamp_millis();
    let snapshot_meta = SnapshotMeta {
        event_id: event_id.to_string(),
        event_seq,
        world_id: world_id.to_string(),
        captured_at: now_ms_val,
        trigger_reason: trigger_reason.as_str().to_string(),
    };

    Ok(WorldSnapshot {
        schema_version: WorldSnapshot::SCHEMA_VERSION,
        snapshot_meta,
        world_state,
        buildings,
        stats,
    })
}

// ============================================================================
// 2. zstd 压缩 / 解压
// ============================================================================

/// 将快照 JSON 字符串压缩为 zstd 字节流，再以 base64 编码为字符串存入 DB。
///
/// 存储格式：
/// - DB `snapshot_json` 字段类型为 TEXT
/// - 内部存储 base64(zstd_compressed_json)
/// - base64 前缀为 `zstd:` 4 字符标识，便于与明文 JSON 区分
///
/// 前缀策略：v1 采用 `zstd:<base64>` 前缀，避免与未来其它压缩格式冲突。
pub fn compress_snapshot(json: &str) -> Result<String, AppError> {
    let compressed = zstd::encode_all(json.as_bytes(), SNAPSHOT_COMPRESSION_LEVEL)
        .map_err(|e| AppError::Internal(format!("snapshot zstd compress failed: {}", e)))?;
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &compressed);
    Ok(format!("zstd:{}", b64))
}

/// 解压 `zstd:<base64>` 格式的快照字符串为原始 JSON。
///
/// 若输入不以前缀 `zstd:` 开头，则视为明文 JSON 直接返回（兼容调试场景）。
pub fn decompress_snapshot(stored: &str) -> Result<String, AppError> {
    if let Some(b64) = stored.strip_prefix("zstd:") {
        let compressed = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .map_err(|e| AppError::Internal(format!("snapshot base64 decode failed: {}", e)))?;
        let bytes = zstd::decode_all(compressed.as_slice())
            .map_err(|e| AppError::Internal(format!("snapshot zstd decompress failed: {}", e)))?;
        String::from_utf8(bytes)
            .map_err(|e| AppError::Internal(format!("snapshot utf8 decode failed: {}", e)))
    } else {
        // 明文 JSON（兼容调试场景）
        Ok(stored.to_string())
    }
}

/// 判断存储的 snapshot_json 是否为 zstd 压缩格式（用于调试 / 监控）。
#[allow(dead_code)]
pub fn is_compressed(stored: &str) -> bool {
    stored.starts_with("zstd:")
}

// ============================================================================
// 3. 快照存储决策
// ============================================================================

/// 判断当前事件后是否需要存储快照。
///
/// 决策规则（spec §2.3）：
/// 1. **世界初始化**：world_init（spec §2.3 第 3 项，首条事件触发）
/// 2. **重要节点**：complete / upgrade / remove（spec §2.3 第 2 项）
/// 3. **周期性**：自上次快照后累计 ≥ PERIODIC_SNAPSHOT_EVENT_THRESHOLD 条事件
/// 4. **文明等级变化**：civ_level_change（spec §2.3 第 4 项，外部触发）
///
/// 参数：
/// - `event_type`：当前事件类型（build/complete/upgrade/demolish/move）
/// - `events_since_last_snapshot`：自上次快照后累计事件数
/// - `is_first_event`：是否为该世界首条事件
pub fn should_snapshot(
    event_type: &str,
    events_since_last_snapshot: i64,
    is_first_event: bool,
) -> Option<SnapshotTriggerReason> {
    // 1. 世界初始化首条事件
    if is_first_event {
        return Some(SnapshotTriggerReason::WorldInit);
    }

    // 2. 重要节点
    let important = match event_type {
        "complete" => Some(SnapshotTriggerReason::Complete),
        "upgrade" => Some(SnapshotTriggerReason::Upgrade),
        "demolish" | "remove" => Some(SnapshotTriggerReason::Remove),
        _ => None,
    };
    if important.is_some() {
        return important;
    }

    // 3. 周期性触发
    if events_since_last_snapshot >= PERIODIC_SNAPSHOT_EVENT_THRESHOLD {
        return Some(SnapshotTriggerReason::Periodic10);
    }

    None
}

// ============================================================================
// 4. 序号计算
// ============================================================================

/// 计算当前事件在世界事件流中的序号（从 1 开始）。
///
/// 实现：查询该世界全部历史事件总数 + 1（含当前正在插入的事件）。
/// 注：调用此函数时应在外部事务提交后调用，确保 INSERT 已生效。
pub async fn compute_event_seq(
    pool: &SqlitePool,
    world_id: &str,
) -> Result<u32, AppError> {
    let count = game_repo::count_events_since(pool, world_id, 0).await?;
    Ok(count as u32)
}

// ============================================================================
// 5. 完整快照写入流程
// ============================================================================

/// 完整的快照写入流程：生成 + 压缩 + 回填到 history 记录。
///
/// 调用时机：service 层在 `insert_build_history` 之后，判断 `should_snapshot`
/// 返回 `Some(reason)` 时调用本函数。
///
/// 流程：
/// 1. 计算事件序号
/// 2. 生成完整快照
/// 3. 序列化为 JSON + zstd 压缩 + base64 编码
/// 4. UPDATE game_build_history SET snapshot_json = ? WHERE id = ?
///
/// 失败处理：快照写入失败不回滚主事务（仅记录日志），保证主业务不受影响。
pub async fn write_snapshot(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    history_id: &str,
    trigger_reason: SnapshotTriggerReason,
) -> Result<(), AppError> {
    // 1. 计算事件序号
    let event_seq = compute_event_seq(pool, world_id).await?;

    // 2. 生成快照
    let snapshot = generate_snapshot(
        pool,
        user_id,
        world_id,
        history_id,
        event_seq,
        trigger_reason,
    )
    .await?;

    // 3. 序列化 + 压缩
    let json = serde_json::to_string(&snapshot)
        .map_err(|e| AppError::Internal(format!("snapshot serialize failed: {}", e)))?;
    let compressed = compress_snapshot(&json)?;

    // 4. 回填
    game_repo::update_build_history_snapshot(pool, history_id, &compressed).await?;

    tracing::info!(
        world_id = %world_id,
        history_id = %history_id,
        event_seq = event_seq,
        trigger_reason = %trigger_reason.as_str(),
        building_count = snapshot.stats.building_count,
        "snapshot_written"
    );

    Ok(())
}

// ============================================================================
// 6. 事件后自动快照写入便利函数（供 game_service 钩子调用）
// ============================================================================

/// 事件后自动快照写入便利函数：自动决策 + 写入。
///
/// 调用时机：service 层在 `insert_build_history` 事务提交后调用本函数。
/// 入参 `event_type` 与 history 表的 `event_type` 字段一致（build/upgrade/demolish/move/complete）。
///
/// 决策流程：
/// 1. 统计该世界当前总事件数（含本次）
/// 2. 若为首条事件（total_events == 1）→ WorldInit 触发
/// 3. 否则查找最近快照点，计算 `events_since_last_snapshot`
/// 4. 调用 `should_snapshot` 决策是否触发快照
/// 5. 触发则调用 `write_snapshot`（失败仅记录日志，不传播错误）
///
/// 失败处理：快照写入失败不回滚主事务（仅记录 warn 日志），保证主业务不受影响。
pub async fn maybe_write_snapshot_after_event(
    pool: &SqlitePool,
    user_id: i64,
    world_id: &str,
    history_id: &str,
    event_type: &str,
) -> Result<(), AppError> {
    // 1. 统计该世界总事件数（含本次 INSERT，commit 后已生效）
    let total_events = game_repo::count_events_since(pool, world_id, 0).await?;
    let is_first = total_events == 1;

    // 2. 计算自上次快照后累计事件数
    let events_since_last: i64 = if is_first {
        0
    } else {
        let now_ts = chrono::Utc::now().timestamp_millis();
        match game_repo::find_latest_snapshot_before(pool, world_id, now_ts).await? {
            Some(last_snap) => {
                // 自上次快照时间戳（不含）之后的事件数
                game_repo::count_events_since(pool, world_id, last_snap.created_at).await?
            }
            None => total_events,
        }
    };

    // 3. 决策
    if let Some(reason) = should_snapshot(event_type, events_since_last, is_first) {
        // 4. 写入（失败仅记录日志，不传播错误以保证主业务不受影响）
        if let Err(e) = write_snapshot(pool, user_id, world_id, history_id, reason).await {
            tracing::warn!(
                error = %e,
                world_id = %world_id,
                history_id = %history_id,
                event_type = %event_type,
                "snapshot_write_failed_after_event"
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

    #[test]
    fn test_compress_decompress_roundtrip() {
        let original = r#"{"schema_version":1,"buildings":[{"id":"b1","name":"测试"}]}"#;
        let compressed = compress_snapshot(original).unwrap();
        assert!(compressed.starts_with("zstd:"));
        let decompressed = decompress_snapshot(&compressed).unwrap();
        assert_eq!(original, decompressed);
    }

    #[test]
    fn test_decompress_plain_json_compatibility() {
        // 明文 JSON（无 zstd: 前缀）应直接返回
        let plain = r#"{"test": true}"#;
        let result = decompress_snapshot(plain).unwrap();
        assert_eq!(plain, result);
    }

    #[test]
    fn test_should_snapshot_first_event() {
        let result = should_snapshot("build", 0, true);
        assert_eq!(result, Some(SnapshotTriggerReason::WorldInit));
    }

    #[test]
    fn test_should_snapshot_important_complete() {
        let result = should_snapshot("complete", 1, false);
        assert_eq!(result, Some(SnapshotTriggerReason::Complete));
    }

    #[test]
    fn test_should_snapshot_important_upgrade() {
        let result = should_snapshot("upgrade", 1, false);
        assert_eq!(result, Some(SnapshotTriggerReason::Upgrade));
    }

    #[test]
    fn test_should_snapshot_important_demolish() {
        // demolish 应被识别为 remove（spec 兼容）
        let result = should_snapshot("demolish", 1, false);
        assert_eq!(result, Some(SnapshotTriggerReason::Remove));
    }

    #[test]
    fn test_should_snapshot_periodic_threshold() {
        // 第 10 条非重要事件触发周期性快照
        let result = should_snapshot("build", PERIODIC_SNAPSHOT_EVENT_THRESHOLD, false);
        assert_eq!(result, Some(SnapshotTriggerReason::Periodic10));
    }

    #[test]
    fn test_should_snapshot_no_snapshot_below_threshold() {
        // 第 5 条 build 事件，未达阈值
        let result = should_snapshot("build", 5, false);
        assert_eq!(result, None);
    }

    #[test]
    fn test_should_snapshot_move_never_triggers() {
        // move 不属于重要节点，仅在周期性触发时存储
        let result = should_snapshot("move", 5, false);
        assert_eq!(result, None);
        let result = should_snapshot("move", PERIODIC_SNAPSHOT_EVENT_THRESHOLD, false);
        assert_eq!(result, Some(SnapshotTriggerReason::Periodic10));
    }
}
