//! 游戏 3D 重构 - IPC 命令层
//!
//! Task 5.1：实现前端 → 后端的全部游戏 IPC 命令（22 个）。
//! change-id: `game-3d-rebuild-refactor`
//!
//! 命令清单（按 tasks.md 阶段5 + 08_API 接口清单）：
//! - **世界与境界**（6）：game_init_world / game_get_world_state / game_get_realm_info
//!   / game_get_breakthrough_preview / game_start_breakthrough / game_submit_breakthrough
//! - **建筑系统**（7）：game_get_buildings / game_get_building_catalog / game_start_building
//!   / game_upgrade_building / game_remove_building / game_move_building / game_get_build_history
//! - **知识联动**（4）：game_get_knowledge_domains / game_get_knowledge_progress
//!   / game_map_kb_category / game_sync_knowledge_event（内部便利命令）
//! - **突破历史 + 世界列表**（3）：game_get_breakthrough_history / game_list_worlds / game_delete_world
//! - **统计聚合便利**（2）：game_get_events_and_tasks / game_get_build_timeline
//!
//! 风格约定：与 `todo_commands.rs` 一致
//! - 函数签名：`pub async fn xxx(state: State<'_, AppState>, ...) -> Result<ApiResponse<T>, String>`
//! - service 调用：`&state.pool` 传 SqlitePool
//! - 错误处理：`Err(e) => Err(e.into())` 将 AppError 转 String

use tauri::State;
// TimeZone trait 提供 from_local_datetime / from_utc_datetime 等方法
use chrono::TimeZone;

use crate::commands::common::require_auth;
use crate::db::connection::AppState;
use crate::db::repositories::game_repo::{self, WorldState, WorldSummary};
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;
use crate::models::game::{
    GameBreakthroughRecord, GameBuildHistory, GameBuilding, GameKbCategoryMapping,
    GameKnowledgeDomain, GameKnowledgeProgress, RebuiltScene, building_required_points,
};
use crate::services::game_breakthrough_service::{
    self, BreakthroughAnswer, BreakthroughOutcome, BreakthroughSession,
};
use crate::services::game_knowledge_sync::{self, AwardPointsRequest};
use crate::services::game_service::{self, RealmInfo, StartBuildingRequest};
use crate::services::timeline_service;

// ============================================================================
// 1. 世界与境界
// ============================================================================

/// 创建新世界（32×32 平原，凡人初期，白道基，12 领域进度初始化）。
#[tauri::command]
pub async fn game_init_world(
    state: State<'_, AppState>,
    player_name: Option<String>,
) -> Result<ApiResponse<WorldState>, String> {
    let user_id = require_auth(&state).await?;
    match game_service::init_world(&state.pool, user_id, player_name.as_deref()).await {
        Ok(world) => {
            // 返回完整世界状态（含 12 领域进度）
            // 多用户隔离（批次 6）：按 user_id 过滤
            let world_state = game_repo::get_world_state(&state.pool, user_id, &world.id)
                .await
                .map_err(|e: AppError| -> String { e.into() })?
                .ok_or_else(|| "世界创建后查询失败".to_string())?;
            Ok(ApiResponse::success(world_state))
        }
        Err(e) => Err(e.into()),
    }
}

/// 获取世界完整状态（世界 + 全部建筑 + 12 领域进度）。
#[tauri::command]
pub async fn game_get_world_state(
    state: State<'_, AppState>,
    world_id: String,
) -> Result<ApiResponse<Option<WorldState>>, String> {
    // 多用户隔离（批次 6）：按 user_id 过滤
    let user_id = require_auth(&state).await?;
    match game_repo::get_world_state(&state.pool, user_id, &world_id).await {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Err(e.into()),
    }
}

/// 获取世界境界信息（境界信息卡数据源）。
#[tauri::command]
pub async fn game_get_realm_info(
    state: State<'_, AppState>,
    world_id: String,
) -> Result<ApiResponse<RealmInfo>, String> {
    // 多用户隔离（批次 6）：按 user_id 过滤
    let user_id = require_auth(&state).await?;
    match game_service::get_realm_info(&state.pool, user_id, &world_id).await {
        Ok(info) => Ok(ApiResponse::success(info)),
        Err(e) => Err(e.into()),
    }
}

/// 预览突破考验信息（不实际出题，仅返回境界/冷却等元数据）。
///
/// 前端可用此判断当前是否可突破 + 显示预期题量。
#[tauri::command]
pub async fn game_get_breakthrough_preview(
    state: State<'_, AppState>,
    world_id: String,
) -> Result<ApiResponse<BreakthroughPreviewInfo>, String> {
    // 多用户隔离（批次 6）：按 user_id 过滤
    let user_id = require_auth(&state).await?;

    let world = game_repo::get_world(&state.pool, user_id, &world_id)
        .await
        .map_err(|e: AppError| -> String { e.into() })?
        .ok_or_else(|| "世界不存在".to_string())?;

    // 获取最近突破记录，判断冷却
    let recent = game_repo::list_breakthrough_records(&state.pool, &world_id, 1)
        .await
        .map_err(|e: AppError| -> String { e.into() })?;
    let cooldown_until = recent.first().and_then(|r| {
        if r.result == "failed" || r.result == "dropped" {
            Some(r.created_at + 24 * 3600 * 1000)
        } else {
            None
        }
    });

    Ok(ApiResponse::success(BreakthroughPreviewInfo {
        realm_major: world.realm_major,
        realm_minor: world.realm_minor,
        dao_foundation: world.dao_foundation,
        can_breakthrough: cooldown_until
            .map(|t| chrono::Utc::now().timestamp_millis() >= t)
            .unwrap_or(true),
        cooldown_until,
    }))
}

/// 开始突破考验（D4.1: 优先 AI 出题，失败降级 mock；返回会话）。
///
/// 内部校验：境界 / 修为圆满 / 道基等级 / 冷却期，全部通过后返回题目。
/// `model_id = Some(id)` 时走 AI 出题；`None` 时走 mock。
#[tauri::command]
pub async fn game_start_breakthrough(
    state: State<'_, AppState>,
    world_id: String,
    model_id: Option<i64>,
) -> Result<ApiResponse<BreakthroughSession>, String> {
    let user_id = require_auth(&state).await?;

    let ai_ctx = match model_id {
        Some(mid) => Some(game_breakthrough_service::AiContext {
            pool: state.pool.clone(),
            mek_manager: state.mek_manager.clone(),
            user_id,
            model_id: mid,
        }),
        None => None,
    };

    // 多用户隔离（批次 6）：传 user_id 给 service 层
    match game_breakthrough_service::prepare_breakthrough(&state.pool, user_id, &world_id, ai_ctx.as_ref()).await {
        Ok(session) => Ok(ApiResponse::success(session)),
        Err(e) => Err(e.into()),
    }
}

/// 提交突破考验（D4.1: 优先 AI 批改，失败降级 mock + 道基评定 + 结果处理）。
#[tauri::command]
pub async fn game_submit_breakthrough(
    state: State<'_, AppState>,
    session: BreakthroughSession,
    answers: Vec<BreakthroughAnswer>,
    model_id: Option<i64>,
) -> Result<ApiResponse<BreakthroughOutcome>, String> {
    let user_id = require_auth(&state).await?;

    let ai_ctx = match model_id {
        Some(mid) => Some(game_breakthrough_service::AiContext {
            pool: state.pool.clone(),
            mek_manager: state.mek_manager.clone(),
            user_id,
            model_id: mid,
        }),
        None => None,
    };

    // 多用户隔离（批次 6）：传 user_id 给 service 层
    match game_breakthrough_service::submit_breakthrough(&state.pool, user_id, &session, &answers, ai_ctx.as_ref()).await {
        Ok(outcome) => Ok(ApiResponse::success(outcome)),
        Err(e) => Err(e.into()),
    }
}

// ============================================================================
// 2. 建筑系统
// ============================================================================

/// 获取世界所有建筑（可按状态过滤）。
#[tauri::command]
pub async fn game_get_buildings(
    state: State<'_, AppState>,
    world_id: String,
    status: Option<String>,
) -> Result<ApiResponse<Vec<GameBuilding>>, String> {
    let user_id = require_auth(&state).await?;
    // 多用户隔离：验证 world_id 归属当前用户
    game_repo::get_world(&state.pool, user_id, &world_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "世界不存在或无权访问".to_string())?;
    let result = match status {
        Some(s) => game_repo::list_buildings_by_status(&state.pool, &world_id, &s).await,
        None => game_repo::list_buildings_by_world(&state.pool, &world_id).await,
    };
    match result {
        Ok(buildings) => Ok(ApiResponse::success(buildings)),
        Err(e) => Err(e.into()),
    }
}

/// 获取建筑目录（8 大类 + 子类静态配置）。
///
/// 注：当前返回简化目录（仅大类与子类标识），完整 GLB 模型清单由前端资源层维护。
/// 未来可扩展为从配置文件加载完整子类属性（积分门槛 / 时间系数 / 等级上限等）。
#[tauri::command]
pub async fn game_get_building_catalog(
    state: State<'_, AppState>,
) -> Result<ApiResponse<BuildingCatalog>, String> {
    let _user_id = require_auth(&state).await?;

    // 辅助构造：subtype + name → BuildingSubtypeInfo（base_cost 由 building_required_points 计算）
    // level=1 时基础积分，供前端预建造面板展示成本
    let mk_sub = |subtype: &str, name: &str, category: &str| -> BuildingSubtypeInfo {
        BuildingSubtypeInfo {
            subtype: subtype.to_string(),
            name: name.to_string(),
            base_cost: building_required_points(category, subtype, 1),
        }
    };

    // 8 大类清单（对应 04 文档 §2.1）
    let categories = vec![
        BuildingCategoryInfo {
            category: "house".to_string(),
            name: "房屋".to_string(),
            civilization_level_required: 1,
            subtypes: vec![
                mk_sub("thatch_cottage", "茅草屋", "house"),
                mk_sub("wooden_house", "木屋", "house"),
                mk_sub("brick_house", "砖瓦房", "house"),
                mk_sub("courtyard", "四合院", "house"),
            ],
        },
        BuildingCategoryInfo {
            category: "town".to_string(),
            name: "城镇".to_string(),
            civilization_level_required: 2,
            subtypes: vec![
                mk_sub("market", "集市", "town"),
                mk_sub("tavern", "酒馆", "town"),
                mk_sub("school", "学堂", "town"),
                mk_sub("temple", "神庙", "town"),
                mk_sub("watchtower", "瞭望塔", "town"),
            ],
        },
        BuildingCategoryInfo {
            category: "city".to_string(),
            name: "城池".to_string(),
            civilization_level_required: 3,
            subtypes: vec![
                mk_sub("city_wall", "城墙", "city"),
                mk_sub("government", "衙门", "city"),
                mk_sub("bank", "钱庄", "city"),
                mk_sub("warehouse", "仓库", "city"),
                mk_sub("academy", "书院", "city"),
            ],
        },
        BuildingCategoryInfo {
            category: "kingdom".to_string(),
            name: "王国".to_string(),
            civilization_level_required: 4,
            subtypes: vec![
                mk_sub("throne_hall", "王座大厅", "kingdom"),
                mk_sub("court", "法院", "kingdom"),
                mk_sub("embassy", "使馆", "kingdom"),
                mk_sub("barracks", "兵营", "kingdom"),
            ],
        },
        BuildingCategoryInfo {
            category: "palace".to_string(),
            name: "宫殿".to_string(),
            civilization_level_required: 5,
            subtypes: vec![
                mk_sub("grand_hall", "大殿", "palace"),
                mk_sub("inner_palace", "宫闱", "palace"),
                mk_sub("castle", "城堡", "palace"),
                mk_sub("treasure_pavilion", "藏宝阁", "palace"),
                mk_sub("spirit_garden", "御花园", "palace"),
            ],
        },
        BuildingCategoryInfo {
            category: "technology".to_string(),
            name: "科技".to_string(),
            civilization_level_required: 6,
            subtypes: vec![
                mk_sub("lab", "实验室", "technology"),
                mk_sub("factory", "工厂", "technology"),
                mk_sub("reactor", "反应堆", "technology"),
                mk_sub("observatory", "天文台", "technology"),
                mk_sub("data_center", "数据中心", "technology"),
            ],
        },
        BuildingCategoryInfo {
            category: "sect".to_string(),
            name: "宗门".to_string(),
            civilization_level_required: 7,
            subtypes: vec![
                mk_sub("main_hall", "主殿", "sect"),
                mk_sub("cultivation_cave", "修炼洞府", "sect"),
                mk_sub("scripture_pavilion", "藏经阁", "sect"),
                mk_sub("alchemy_room", "炼丹房", "sect"),
            ],
        },
        BuildingCategoryInfo {
            category: "immortal".to_string(),
            name: "修仙".to_string(),
            civilization_level_required: 8,
            subtypes: vec![
                mk_sub("flying_palace", "凌霄宝殿", "immortal"),
                mk_sub("spirit_tower", "聚灵塔", "immortal"),
                mk_sub("ascension_altar", "飞升台", "immortal"),
                mk_sub("immortal_pavilion", "仙居", "immortal"),
            ],
        },
    ];

    Ok(ApiResponse::success(BuildingCatalog { categories }))
}

/// 开始建造（预建造：扣积分 + 建建筑 + 记历史，事务保证原子性）。
#[tauri::command]
pub async fn game_start_building(
    state: State<'_, AppState>,
    request: StartBuildingRequest,
) -> Result<ApiResponse<GameBuilding>, String> {
    // 多用户隔离（批次 6）：传 user_id 给 service 层
    let user_id = require_auth(&state).await?;
    match game_service::start_building(&state.pool, user_id, &request).await {
        Ok(building) => Ok(ApiResponse::success(building)),
        Err(e) => Err(e.into()),
    }
}

/// 升级建筑（扣积分 + 升级 + 记历史）。
#[tauri::command]
pub async fn game_upgrade_building(
    state: State<'_, AppState>,
    building_id: String,
    upgrade_cost: i64,
) -> Result<ApiResponse<GameBuilding>, String> {
    // 多用户隔离（批次 6）：传 user_id 给 service 层
    let user_id = require_auth(&state).await?;
    match game_service::upgrade_building(&state.pool, user_id, &building_id, upgrade_cost).await {
        Ok(building) => Ok(ApiResponse::success(building)),
        Err(e) => Err(e.into()),
    }
}

/// 拆除建筑（返还 50% 积分 + 删建筑 + 记历史）。
#[tauri::command]
pub async fn game_remove_building(
    state: State<'_, AppState>,
    building_id: String,
    refund_ratio: Option<f64>,
) -> Result<ApiResponse<i64>, String> {
    // 多用户隔离（批次 6）：传 user_id 给 service 层
    let user_id = require_auth(&state).await?;
    let ratio = refund_ratio.unwrap_or(0.5);
    match game_service::remove_building(&state.pool, user_id, &building_id, ratio).await {
        Ok(refund_amount) => Ok(ApiResponse::success(refund_amount)),
        Err(e) => Err(e.into()),
    }
}

/// 移动建筑（消耗 10% 建筑成本积分 + 更新坐标 + 记历史）。
#[tauri::command]
pub async fn game_move_building(
    state: State<'_, AppState>,
    building_id: String,
    new_pos_x: f64,
    new_pos_y: f64,
    new_pos_z: f64,
    new_rotation_y: Option<f64>,
    move_cost: i64,
) -> Result<ApiResponse<GameBuilding>, String> {
    // 多用户隔离（批次 6）：传 user_id 给 service 层
    let user_id = require_auth(&state).await?;
    match game_service::move_building(
        &state.pool,
        user_id,
        &building_id,
        new_pos_x,
        new_pos_y,
        new_pos_z,
        new_rotation_y,
        move_cost,
    )
    .await
    {
        Ok(building) => Ok(ApiResponse::success(building)),
        Err(e) => Err(e.into()),
    }
}

/// 获取建造历史（时间轴回放数据源）。
#[tauri::command]
pub async fn game_get_build_history(
    state: State<'_, AppState>,
    world_id: String,
    limit: Option<i64>,
) -> Result<ApiResponse<Vec<GameBuildHistory>>, String> {
    let user_id = require_auth(&state).await?;
    // 多用户隔离：验证 world_id 归属当前用户
    game_repo::get_world(&state.pool, user_id, &world_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "世界不存在或无权访问".to_string())?;
    let limit = limit.unwrap_or(100);
    match game_repo::list_build_history(&state.pool, &world_id, limit).await {
        Ok(history) => Ok(ApiResponse::success(history)),
        Err(e) => Err(e.into()),
    }
}

// ============================================================================
// 3. 知识联动
// ============================================================================

/// 获取全部 12 知识领域。
#[tauri::command]
pub async fn game_get_knowledge_domains(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<GameKnowledgeDomain>>, String> {
    let _user_id = require_auth(&state).await?;
    match game_repo::list_domains(&state.pool).await {
        Ok(domains) => Ok(ApiResponse::success(domains)),
        Err(e) => Err(e.into()),
    }
}

/// 获取世界在某领域的进度（或全部 12 领域）。
#[tauri::command]
pub async fn game_get_knowledge_progress(
    state: State<'_, AppState>,
    world_id: String,
    domain_id: Option<String>,
) -> Result<ApiResponse<Vec<GameKnowledgeProgress>>, String> {
    let user_id = require_auth(&state).await?;
    // 多用户隔离：验证 world_id 归属当前用户
    game_repo::get_world(&state.pool, user_id, &world_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "世界不存在或无权访问".to_string())?;
    let result = match domain_id {
        Some(d) => {
            let progress = game_repo::get_progress(&state.pool, &world_id, &d).await?;
            progress.into_iter().collect()
        }
        None => game_repo::list_progress_by_world(&state.pool, &world_id).await?,
    };
    Ok(ApiResponse::success(result))
}

/// 设置知识库分类到知识领域的映射（已存在则 UPSERT 覆盖）。
///
/// 修改映射不回溯历史积分，仅影响新条目入账领域。
/// `mapped_by` 固定记为 `manual`（AI 自动映射走 service 层另一条路径）。
/// 错误码：GAME_060=分类不存在 / GAME_061=领域 ID 非法。
#[tauri::command]
pub async fn game_map_kb_category(
    state: State<'_, AppState>,
    category_id: i64,
    domain_id: String,
) -> Result<ApiResponse<()>, String> {
    let _user_id = require_auth(&state).await?;
    // 校验领域 ID 合法性（12 个预定义之一）
    let valid = crate::models::game::domain::ALL.contains(&domain_id.as_str());
    if !valid {
        return Err(AppError::Validation(format!("非法知识领域 ID: {domain_id}")).into());
    }
    let mapping = GameKbCategoryMapping {
        category_id,
        domain_id,
        mapped_by: "manual".to_string(),
        confidence: None,
        mapped_at: chrono::Utc::now().timestamp_millis(),
    };
    match game_repo::upsert_category_mapping(&state.pool, &mapping).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

/// 获取所有 KB 分类映射（设置页左栏列表用，前端与 kb_categories LEFT JOIN）。
///
/// 返回 `GameKbCategoryMapping[]`，未映射分类不在列表中（前端用 kb_categories
/// 全集做 LEFT JOIN 即可得到"未映射"分类）。
#[tauri::command]
pub async fn game_get_kb_category_mappings(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<GameKbCategoryMapping>>, String> {
    let _user_id = require_auth(&state).await?;
    match game_repo::list_all_mappings(&state.pool).await {
        Ok(mappings) => Ok(ApiResponse::success(mappings)),
        Err(e) => Err(e.into()),
    }
}

/// 近 N 天每领域每日积分趋势响应（知识领域积分卡 sparkline 数据源）。
///
/// - `date_labels`：日期标签数组（`'YYYY-MM-DD'`，长度 = `days`，Asia/Shanghai 时区）
/// - `trend`：`Record<domain_id, number[]>`，每领域每日入账积分增量（长度 = `days`，缺日补 0）
/// - `daily_totals`：全领域每日总积分（长度 = `days`）
#[derive(serde::Serialize)]
pub struct PointsTrendResponse {
    pub days: u32,
    pub date_labels: Vec<String>,
    pub trend: std::collections::HashMap<String, Vec<i64>>,
    pub daily_totals: Vec<i64>,
}

/// 获取近 N 天每领域每日积分趋势（仅 `result='accepted'` 的入账积分）。
///
/// `days` 默认 7，上限 30。日期对齐 Asia/Shanghai 时区（`+8 hours`）。
/// 用于知识领域积分卡的 sparkline 趋势图与日均/周累统计。
#[tauri::command]
pub async fn game_get_points_trend(
    state: State<'_, AppState>,
    world_id: String,
    days: Option<u32>,
) -> Result<ApiResponse<PointsTrendResponse>, String> {
    let user_id = require_auth(&state).await?;
    // 多用户隔离：验证 world_id 归属当前用户
    game_repo::get_world(&state.pool, user_id, &world_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "世界不存在或无权访问".to_string())?;
    let days = days.unwrap_or(7).clamp(1, 30);

    // Asia/Shanghai 时区（UTC+8）
    let shanghai_offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
    let now_shanghai = chrono::Utc::now().with_timezone(&shanghai_offset);
    // days 天前的 00:00:00（含当天，故取 days 天前）
    let start_date = now_shanghai.date_naive() - chrono::Duration::days(days as i64 - 1);
    let start_naive = start_date.and_hms_opt(0, 0, 0).unwrap();
    let start_ms = shanghai_offset
        .from_local_datetime(&start_naive)
        .unwrap()
        .timestamp_millis();

    // 生成日期标签数组（升序，长度 = days）
    let date_labels: Vec<String> = (0..days)
        .map(|i| {
            (start_date + chrono::Duration::days(i as i64))
                .format("%Y-%m-%d")
                .to_string()
        })
        .collect();

    // 查询聚合数据
    let rows = game_repo::aggregate_daily_points_by_domain(&state.pool, &world_id, start_ms)
        .await?;

    // 组装 trend: Record<domain_id, Vec<i64>>（按 date_labels 对齐，缺日补 0）
    use std::collections::HashMap;
    let mut daily_map_by_domain: HashMap<String, HashMap<String, i64>> = HashMap::new();
    for row in rows {
        daily_map_by_domain
            .entry(row.domain_id)
            .or_default()
            .insert(row.date_str, row.total_delta);
    }

    let mut trend: HashMap<String, Vec<i64>> = HashMap::new();
    for (domain_id, daily_map) in daily_map_by_domain {
        let series: Vec<i64> = date_labels
            .iter()
            .map(|d| *daily_map.get(d).unwrap_or(&0))
            .collect();
        trend.insert(domain_id, series);
    }

    // 计算每日全领域总积分（按列求和：第 i 日 = 所有领域 series[i] 之和）
    let daily_totals: Vec<i64> = (0..days as usize)
        .map(|i| {
            trend
                .values()
                .map(|series| series.get(i).copied().unwrap_or(0))
                .sum()
        })
        .collect();

    Ok(ApiResponse::success(PointsTrendResponse {
        days,
        date_labels,
        trend,
        daily_totals,
    }))
}

// ============================================================================
// 4. 突破历史 + 世界列表
// ============================================================================

/// 获取突破历史（突破历程时间线数据源）。
#[tauri::command]
pub async fn game_get_breakthrough_history(
    state: State<'_, AppState>,
    world_id: String,
    limit: Option<i64>,
) -> Result<ApiResponse<Vec<GameBreakthroughRecord>>, String> {
    let user_id = require_auth(&state).await?;
    // 多用户隔离：验证 world_id 归属当前用户
    game_repo::get_world(&state.pool, user_id, &world_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "世界不存在或无权访问".to_string())?;
    let limit = limit.unwrap_or(20);
    match game_repo::list_breakthrough_records(&state.pool, &world_id, limit).await {
        Ok(records) => Ok(ApiResponse::success(records)),
        Err(e) => Err(e.into()),
    }
}

/// 获取所有世界列表（含聚合摘要，LEFT JOIN 避免 N+1）。
#[tauri::command]
pub async fn game_list_worlds(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<WorldSummary>>, String> {
    // 多用户隔离（批次 6）：按 user_id 过滤
    let user_id = require_auth(&state).await?;
    match game_repo::list_world_summaries(&state.pool, user_id).await {
        Ok(summaries) => Ok(ApiResponse::success(summaries)),
        Err(e) => Err(e.into()),
    }
}

/// 删除世界（级联删除建筑/进度/历史等）。
#[tauri::command]
pub async fn game_delete_world(
    state: State<'_, AppState>,
    world_id: String,
) -> Result<ApiResponse<()>, String> {
    // 多用户隔离（批次 6）：按 user_id 过滤
    let user_id = require_auth(&state).await?;
    match game_repo::delete_world(&state.pool, user_id, &world_id).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.into()),
    }
}

// ============================================================================
// 5. IPC 专用 DTO
// ============================================================================

/// 突破预览信息（前端境界突破按钮状态判断）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BreakthroughPreviewInfo {
    pub realm_major: String,
    pub realm_minor: String,
    pub dao_foundation: String,
    /// 是否可突破（境界/修为/道基/冷却全部通过）。
    pub can_breakthrough: bool,
    /// 冷却截止时间戳（ms），无冷却时为 `None`。
    pub cooldown_until: Option<i64>,
}

/// 建筑目录（8 大类 + 子类）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildingCatalog {
    pub categories: Vec<BuildingCategoryInfo>,
}

/// 建筑大类信息。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildingCategoryInfo {
    pub category: String,
    pub name: String,
    pub civilization_level_required: u32,
    pub subtypes: Vec<BuildingSubtypeInfo>,
}

/// 建筑子类信息（简化版，完整属性由前端资源层维护）。
///
/// `base_cost` 为 level=1 时所需领域积分（由 `building_required_points` 计算），
/// 供前端预建造面板展示成本，避免前端硬编码重复 cost 表。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildingSubtypeInfo {
    pub subtype: String,
    pub name: String,
    pub base_cost: i64,
}

// ============================================================================
// 6. 内部便利命令：知识事件同步（08_API §4.4）
// ============================================================================

/// 内部命令：接收其它模块（知识库/待办/计时器/AI/YuanCode/日志）的积分事件。
///
/// **重要**：此命令主要供后端内部调用（如 `todo_complete`、`chat_send_message` 完成后）。
/// 前端约定不直接 invoke 此命令。
///
/// 错误码：GAME_062=事件类型非法 / GAME_063=领域 ID 非法 / GAME_064=重复事件（幂等拒绝）
/// / GAME_065=超出每日上限（静默拒绝，返回 points_added=0）/ GAME_066=负向事件导致积分变负。
#[tauri::command]
pub async fn game_sync_knowledge_event(
    state: State<'_, AppState>,
    world_id: String,
    event_type: String,
    domain_id: String,
    points: i32,
    source_data: serde_json::Value,
) -> Result<ApiResponse<SyncResult>, String> {
    // 多用户隔离（批次 6）：按 user_id 过滤
    let user_id = require_auth(&state).await?;

    // 1. 校验 event_type（11 种预定义之一）
    if !crate::models::game::source_type::ALL.contains(&event_type.as_str()) {
        return Err(AppError::Validation(format!("非法事件类型: {event_type}")).into());
    }
    // 2. 校验 domain_id（12 种预定义之一）
    if !crate::models::game::domain::ALL.contains(&domain_id.as_str()) {
        return Err(AppError::Validation(format!("非法知识领域 ID: {domain_id}")).into());
    }
    // 3. 从 source_data 提取幂等键 event_id
    let event_id = source_data
        .get("event_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Validation("source_data 缺少 event_id 字段".to_string()))?
        .to_string();

    // 4. 调用 award_points 执行入账
    let req = AwardPointsRequest {
        world_id: world_id.clone(),
        source_type: event_type,
        domain_id: domain_id.clone(),
        event_id,
        points_delta: points as i64,
        metadata_json: Some(source_data.to_string()),
    };
    let result = game_knowledge_sync::award_points(&state.pool, user_id, &req).await?;

    // 5. 计算修为增量（仅入账成功时；公式 02 文档 §2.2：积分 × 道基系数）
    let realm_xp_added = if result.points_credited > 0 {
        let world = game_repo::get_world(&state.pool, user_id, &world_id)
            .await
            .map_err(|e: AppError| -> String { e.into() })?
            .ok_or_else(|| "世界不存在".to_string())?;
        let dao = crate::models::game::DaoFoundation::from_str(&world.dao_foundation)
            .ok_or_else(|| {
                AppError::Internal(format!("无效的 dao_foundation: {}", world.dao_foundation))
                    .to_string()
            })?;
        let multiplier = crate::models::game::dao_foundation_multiplier(dao);
        result.points_credited as f64 * multiplier
    } else {
        0.0
    };

    Ok(ApiResponse::success(SyncResult {
        points_added: result.points_credited,
        // service 层 BuildProgressUpdate → commands 层 BuildProgressDelta 映射
        build_progress_updated: result
            .build_progress_updated
            .iter()
            .map(|u| BuildProgressDelta {
                building_id: u.building_id.clone(),
                progress_before: u.progress_before,
                progress_after: u.progress_after,
                delta: u.delta,
                completed: u.completed,
            })
            .collect(),
        realm_xp_added,
    }))
}

// ============================================================================
// 7. 统计聚合便利命令（08_API §5.1 / §5.2）
// ============================================================================

/// 获取首页所需的待办事件与提示信息（4 卡片之「事件与任务卡」数据源）。
///
/// 聚合：即将完工建筑 + 突破提示 + 近期事件（建造历史 + 突破历史）。
#[tauri::command]
pub async fn game_get_events_and_tasks(
    state: State<'_, AppState>,
    world_id: String,
) -> Result<ApiResponse<EventsAndTasks>, String> {
    // 多用户隔离（批次 6）：按 user_id 过滤
    let user_id = require_auth(&state).await?;

    // 1. 即将完工建筑（建造中 + 按进度倒序取前 5）
    let buildings = game_repo::list_buildings_by_status(&state.pool, &world_id, "building")
        .await
        .map_err(|e: AppError| -> String { e.into() })?;
    let mut sorted_buildings = buildings;
    sorted_buildings.sort_by(|a, b| {
        b.build_progress
            .partial_cmp(&a.build_progress)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let upcoming_buildings: Vec<UpcomingBuilding> = sorted_buildings
        .iter()
        .take(5)
        .map(|b| UpcomingBuilding {
            building_id: b.id.clone(),
            name: b.name.clone(),
            building_subtype: b.building_subtype.clone(),
            progress: b.build_progress,
            // 建造成本/速率模型待补全，暂不预估完工时间
            remaining_points: 0,
            estimated_complete_at: None,
        })
        .collect();

    // 2. 突破提示（境界 / 修为圆满 / 冷却）
    // 多用户隔离（批次 6）：按 user_id 过滤
    let world = game_repo::get_world(&state.pool, user_id, &world_id)
        .await
        .map_err(|e: AppError| -> String { e.into() })?
        .ok_or_else(|| "世界不存在".to_string())?;
    let recent = game_repo::list_breakthrough_records(&state.pool, &world_id, 1)
        .await
        .map_err(|e: AppError| -> String { e.into() })?;
    let cooldown_until = recent.first().and_then(|r| {
        if r.result == "failed" || r.result == "dropped" {
            Some(r.created_at + 24 * 3600 * 1000)
        } else {
            None
        }
    });
    let now_ms_val = chrono::Utc::now().timestamp_millis();
    let cooldown_remaining = cooldown_until
        .map(|t| ((t - now_ms_val).max(0)) as u64)
        .unwrap_or(0);
    let in_cooldown = cooldown_until.map(|t| now_ms_val < t).unwrap_or(false);

    let realm_major = crate::models::game::RealmMajor::from_str(&world.realm_major)
        .ok_or_else(|| {
            AppError::Internal(format!("无效的 realm_major: {}", world.realm_major)).to_string()
        })?;
    let realm_xp_upper = realm_major.xp_upper_bound();
    let at_immortal = realm_major == crate::models::game::RealmMajor::Immortal;
    let is_full = world.realm_xp >= realm_xp_upper;

    let (can_breakthrough, reason) = if at_immortal {
        (false, "已达仙境，无更高境界可突破".to_string())
    } else if in_cooldown {
        (false, format!("冷却中，剩余 {}ms", cooldown_remaining))
    } else if !is_full {
        (
            false,
            format!("修为未圆满：{}/{}", world.realm_xp, realm_xp_upper),
        )
    } else {
        (true, String::new())
    };

    let breakthrough_hint = BreakthroughHint {
        can_breakthrough,
        reason,
        cooldown_remaining,
    };

    // 3. 近期事件（建造历史 + 突破历史合并，按时间倒序取前 10）
    let build_history = game_repo::list_build_history(&state.pool, &world_id, 5)
        .await
        .map_err(|e: AppError| -> String { e.into() })?;
    let breakthrough_history = game_repo::list_breakthrough_records(&state.pool, &world_id, 5)
        .await
        .map_err(|e: AppError| -> String { e.into() })?;

    let mut events: Vec<(i64, RecentEvent)> = Vec::new();
    for h in build_history {
        events.push((
            h.created_at,
            RecentEvent {
                event_id: h.id.clone(),
                event_type: h.event_type.clone(),
                title: format!("建造事件: {}", h.building_name),
                description: h.snapshot_json.clone().unwrap_or_default(),
                timestamp: h.created_at,
            },
        ));
    }
    for r in breakthrough_history {
        events.push((
            r.created_at,
            RecentEvent {
                event_id: r.id.clone(),
                event_type: format!("breakthrough_{}", r.result),
                title: format!("突破考验: {}", r.result),
                description: r.score.map(|s| format!("得分: {}", s)).unwrap_or_default(),
                timestamp: r.created_at,
            },
        ));
    }
    events.sort_by(|a, b| b.0.cmp(&a.0));
    let recent_events: Vec<RecentEvent> = events.into_iter().take(10).map(|(_, e)| e).collect();

    Ok(ApiResponse::success(EventsAndTasks {
        upcoming_buildings,
        breakthrough_hint,
        recent_events,
    }))
}

/// 获取建造事件时间线（按时间倒序返回，用于时间线视图回放）。
///
/// 08_API §5.2：聚合建造历史 + 突破历史，按时间范围过滤后倒序。
#[tauri::command]
pub async fn game_get_build_timeline(
    state: State<'_, AppState>,
    world_id: String,
    from_time: Option<i64>,
    to_time: Option<i64>,
) -> Result<ApiResponse<Vec<TimelineEvent>>, String> {
    let user_id = require_auth(&state).await?;
    // 多用户隔离：验证 world_id 归属当前用户
    game_repo::get_world(&state.pool, user_id, &world_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "世界不存在或无权访问".to_string())?;

    // 1. 时间范围校验（默认 7 天前到当前）
    let now_ms_val = chrono::Utc::now().timestamp_millis();
    let from = from_time.unwrap_or(now_ms_val - 7 * 24 * 3600 * 1000);
    let to = to_time.unwrap_or(now_ms_val);
    if from > to {
        return Err(AppError::Validation("from_time 不能大于 to_time".to_string()).into());
    }

    // 2. 拉取建造历史 + 突破历史（一次拉取较多条，过滤后排序）
    let build_history = game_repo::list_build_history(&state.pool, &world_id, 1000)
        .await
        .map_err(|e: AppError| -> String { e.into() })?;
    let breakthrough_history =
        game_repo::list_breakthrough_records(&state.pool, &world_id, 1000)
            .await
            .map_err(|e: AppError| -> String { e.into() })?;

    // 3. 合并并按时间范围过滤
    let mut events: Vec<TimelineEvent> = Vec::new();
    for h in build_history {
        if h.created_at < from || h.created_at > to {
            continue;
        }
        events.push(TimelineEvent {
            id: h.id.clone(),
            event_type: h.event_type.clone(),
            building_id: Some(h.building_id.clone()),
            building_name: Some(h.building_name.clone()),
            level: h.level,
            points_delta: None,
            description: h.snapshot_json.clone().unwrap_or_default(),
            timestamp: h.created_at,
        });
    }
    for r in breakthrough_history {
        if r.created_at < from || r.created_at > to {
            continue;
        }
        events.push(TimelineEvent {
            id: r.id.clone(),
            event_type: "breakthrough".to_string(),
            building_id: None,
            building_name: None,
            level: None,
            points_delta: None,
            description: format!(
                "突破考验: {}, 得分: {}",
                r.result,
                r.score.map(|s| s.to_string()).unwrap_or_else(|| "无".into())
            ),
            timestamp: r.created_at,
        });
    }

    // 4. 按时间倒序排序
    events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(ApiResponse::success(events))
}

// ============================================================================
// 8. 便利命令 DTO（08_API §4.4 / §5.1 / §5.2）
// ============================================================================

/// 积分事件同步结果（08_API §4.4）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncResult {
    /// 实际入账积分（被拒绝则为 0）。
    pub points_added: i64,
    /// 被推进进度的建筑列表（仅入账成功且 points > 0 时非空）。
    pub build_progress_updated: Vec<BuildProgressDelta>,
    /// 修为增量 = 积分 × 道基系数。
    pub realm_xp_added: f64,
}

/// 建筑进度增量（08_API §4.4）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildProgressDelta {
    pub building_id: String,
    pub progress_before: f64,
    pub progress_after: f64,
    pub delta: f64,
    pub completed: bool,
}

/// 事件与任务卡数据（08_API §5.1）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventsAndTasks {
    pub upcoming_buildings: Vec<UpcomingBuilding>,
    pub breakthrough_hint: BreakthroughHint,
    pub recent_events: Vec<RecentEvent>,
}

/// 即将完工建筑（08_API §5.1）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpcomingBuilding {
    pub building_id: String,
    pub name: String,
    pub building_subtype: String,
    pub progress: f64,
    pub remaining_points: i64,
    pub estimated_complete_at: Option<i64>,
}

/// 突破提示（08_API §5.1）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BreakthroughHint {
    pub can_breakthrough: bool,
    /// 不可突破时给出原因。
    pub reason: String,
    /// 冷却剩余毫秒数；无冷却时为 0。
    pub cooldown_remaining: u64,
}

/// 近期事件（08_API §5.1）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecentEvent {
    pub event_id: String,
    pub event_type: String,
    pub title: String,
    pub description: String,
    pub timestamp: i64,
}

/// 时间线事件（08_API §5.2）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimelineEvent {
    pub id: String,
    /// `build_start` / `build_complete` / `upgrade_start` / `upgrade_complete` / `remove` / `move` / `breakthrough`
    pub event_type: String,
    pub building_id: Option<String>,
    pub building_name: Option<String>,
    pub level: Option<i32>,
    pub points_delta: Option<std::collections::HashMap<String, i32>>,
    pub description: String,
    pub timestamp: i64,
}

// ============================================================================
// 9. 时间轴回放（11_时间轴回放.md §3）
// ============================================================================

/// 重建指定时间点的世界场景（用于时间轴回放）。
///
/// 11_时间轴回放.md §3.1：根据时间戳查找最近快照，重放增量事件后返回完整场景。
///
/// 调用时机：用户拖动时间轴滑块松开时由前端发起。
/// 内部使用 LRU 缓存（spec §5.2.2），相同时间戳的二次请求命中缓存。
#[tauri::command]
pub async fn game_get_world_snapshot(
    state: State<'_, AppState>,
    world_id: String,
    // 毫秒级 Unix 时间戳，目标重建时间点。
    timestamp: i64,
) -> Result<ApiResponse<RebuiltScene>, String> {
    // 多用户隔离（批次 6）：传 user_id 给 service 层
    let user_id = require_auth(&state).await?;
    match timeline_service::rebuild_scene_at(&state.pool, user_id, &world_id, timestamp).await {
        Ok(scene) => Ok(ApiResponse::success(scene)),
        Err(e) => Err(e.into()),
    }
}

/// 退出回放模式，清空 LRU 缓存。
///
/// 11_时间轴回放.md §3.3：用户点击「返回游戏」按钮，前端调用本命令清空后端缓存。
/// 同时前端 store 切回实时模式（由前端处理）。
#[tauri::command]
pub async fn game_exit_replay(
    state: State<'_, AppState>,
    world_id: String,
) -> Result<ApiResponse<()>, String> {
    let _user_id = require_auth(&state).await?;
    timeline_service::clear_cache().await;
    tracing::info!(world_id = %world_id, "replay_exited_cache_cleared");
    Ok(ApiResponse::success(()))
}

// ============================================================================
// D4.2 智能 NPC 系统命令
// ============================================================================

/// 列出所有 NPC（首次调用时自动 seed 内置 NPC）
#[tauri::command]
pub async fn game_npc_list(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<crate::services::game_npc_service::GameNpc>>, String> {
    let _user_id = require_auth(&state).await?;
    // 首次调用自动 seed
    crate::services::game_npc_service::GameNpcService::seed_builtin_npcs(&state.pool)
        .await
        .map_err(|e| e.to_string())?;
    let npcs = crate::services::game_npc_service::GameNpcService::list_npcs(&state.pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(npcs))
}

/// 获取单个 NPC 详情
#[tauri::command]
pub async fn game_npc_get(
    state: State<'_, AppState>,
    npc_id: String,
) -> Result<ApiResponse<Option<crate::services::game_npc_service::GameNpc>>, String> {
    let _user_id = require_auth(&state).await?;
    let npc = crate::services::game_npc_service::GameNpcService::get_npc(&state.pool, &npc_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(npc))
}

/// 加载与某 NPC 的对话历史
#[tauri::command]
pub async fn game_npc_history(
    state: State<'_, AppState>,
    world_id: String,
    npc_id: String,
    limit: Option<i64>,
) -> Result<ApiResponse<Vec<crate::services::game_npc_service::GameNpcConversation>>, String> {
    let user_id = require_auth(&state).await?;
    // 多用户隔离：验证 world_id 归属当前用户
    game_repo::get_world(&state.pool, user_id, &world_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "世界不存在或无权访问".to_string())?;
    let history = crate::services::game_npc_service::GameNpcService::load_history(
        &state.pool,
        &world_id,
        &npc_id,
        limit.unwrap_or(50),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(history))
}

/// 与 NPC 对话（调用云端 API 多轮对话）
#[tauri::command]
pub async fn game_npc_chat(
    state: State<'_, AppState>,
    world_id: String,
    npc_id: String,
    message: String,
    model_id: Option<i64>,
) -> Result<ApiResponse<crate::services::game_npc_service::NpcChatResponse>, String> {
    let user_id = require_auth(&state).await?;

    let resolved_model_id = match model_id {
        Some(mid) => mid,
        None => {
            let models = crate::db::repositories::ai_repo::get_all_models(&state.pool, user_id)
                .await
                .map_err(|e| e.to_string())?;
            models
                .first()
                .map(|m| m.id)
                .ok_or_else(|| "未配置任何 AI 模型".to_string())?
        }
    };

    let request = crate::services::game_npc_service::NpcChatRequest {
        world_id,
        npc_id,
        user_id,
        model_id: resolved_model_id,
        message,
    };

    let response = crate::services::game_npc_service::GameNpcService::chat(
        &state.pool,
        &state.mek_manager,
        request,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(ApiResponse::success(response))
}

/// 清空与某 NPC 的对话历史
#[tauri::command]
pub async fn game_npc_clear_history(
    state: State<'_, AppState>,
    world_id: String,
    npc_id: String,
) -> Result<ApiResponse<()>, String> {
    let _user_id = require_auth(&state).await?;
    crate::services::game_npc_service::GameNpcService::clear_history(
        &state.pool,
        &world_id,
        &npc_id,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(()))
}

// ============================================================================
// D4.6 智能 NPC 深化：长期记忆 + 关系网 IPC
// ============================================================================

/// D4.6 加载 NPC 对玩家的长期记忆（按重要度倒序）。
///
/// 前端用于"已记住 N 件事"展开查看记忆详情面板。
#[tauri::command]
pub async fn game_npc_memories(
    state: State<'_, AppState>,
    world_id: String,
    npc_id: String,
    limit: Option<i64>,
) -> Result<ApiResponse<Vec<crate::services::game_npc_service::GameNpcMemory>>, String> {
    let _user_id = require_auth(&state).await?;
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let memories = crate::services::game_npc_service::GameNpcService::load_memories(
        &state.pool,
        &world_id,
        &npc_id,
        limit,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(memories))
}

/// D4.6 获取 NPC 对玩家的当前关系状态。
///
/// 前端用于关系条展示。若从未互动过，自动创建一行中立关系（value=0, label=neutral）。
#[tauri::command]
pub async fn game_npc_relationship(
    state: State<'_, AppState>,
    world_id: String,
    npc_id: String,
) -> Result<ApiResponse<crate::services::game_npc_service::GameNpcRelationship>, String> {
    let _user_id = require_auth(&state).await?;
    let relationship =
        crate::services::game_npc_service::GameNpcService::get_or_create_relationship(
            &state.pool,
            &world_id,
            &npc_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(relationship))
}

// ============================================================================
// D4.7 跨 NPC 关系联动：传闻机制 IPC
// ============================================================================

/// D4.7 加载传播给指定 NPC 的传闻（从其他 NPC 听说的关于玩家的事）。
///
/// 前端用于在 NPC 对话界面展示"传闻"面板，让玩家了解信息传播效果。
/// 按 importance 倒序返回，limit 默认 20、clamp 1-100。
#[tauri::command]
pub async fn game_npc_rumors(
    state: State<'_, AppState>,
    world_id: String,
    npc_id: String,
    limit: Option<i64>,
) -> Result<ApiResponse<Vec<crate::services::game_npc_service::GameNpcRumor>>, String> {
    let _user_id = require_auth(&state).await?;
    let limit = limit.unwrap_or(20).clamp(1, 100);
    let rumors = crate::services::game_npc_service::GameNpcService::load_rumors(
        &state.pool,
        &world_id,
        &npc_id,
        limit,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(rumors))
}

// ============================================================================
// D4.3 自适应难度：玩家能力评估 IPC
// ============================================================================

/// D4.3 获取玩家能力评估（突破考验历史表现 + 难度乘数）。
///
/// 前端用于显示技能评分、评级标签、连胜/连败、难度调整提示。
#[tauri::command]
pub async fn game_player_skill(
    state: State<'_, AppState>,
    world_id: String,
) -> Result<ApiResponse<crate::services::game_difficulty_service::PlayerSkill>, String> {
    let _user_id = require_auth(&state).await?;
    let skill = crate::services::game_difficulty_service::GameDifficultyService::get_or_create_skill(
        &state.pool,
        &world_id,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(skill))
}
