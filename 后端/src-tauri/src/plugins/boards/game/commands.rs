//! boards.game 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发（阶段3 批次4b）。
//!
//! 与 boards.xin / boards.terminal 同构：alias 只做旧名 → 逻辑名的映射登记；真实 handler
//! 在 `dispatch_legacy` —— 闭包仅捕获 AppHandle，运行时经 `handle.state::<AppState>()`
//! 解析主应用真实 AppState（契约 06_Rust代码契约 §8.1，pool / 认证 / 服务全部来源于此，
//! 禁止伪造或第二池）；业务实现复用 `game_commands` / `game_story_commands` /
//! `game_opponent_commands` / `game_natural_language_commands` / `game_behavior_commands` /
//! `game_intelligence_commands` 原函数，返回值序列化与旧 IPC 路径一致（V1 输入/输出快照等价）。
//!
//! **参数键口径**：Tauri v2 默认 `ArgumentCase::Camel`；本 dispatcher 以 snake_case 为先、
//! camelCase 回退，兼容两种口径，不按 transport 静默改写输入（《测试流程》IPC 双轨回放
//! 纪律第 1 条）。

pub const PLUGIN_ID: &str = "boards.game";
pub const SHORT_CODE: &str = "gm";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcAlias {
    pub plugin_id: &'static str,
    pub short_code: &'static str,
    pub legacy_command: &'static str,
    pub new_command: &'static str,
}

macro_rules! alias {
    ($command:literal) => {
        IpcAlias {
            plugin_id: PLUGIN_ID,
            short_code: SHORT_CODE,
            legacy_command: $command,
            new_command: concat!("gm:plugin:", $command),
        }
    };
}

/// 44 条 alias：game 35 + game_story 4 + game_intelligence 2 + game_opponent 1
/// + game_natural_language 1 + game_behavior 1。
/// `main.rs` 旧 transport 于本批 S7 移除。
pub const IPC_ALIASES: &[IpcAlias] = &[
    // 世界与境界（6）
    alias!("game_init_world"),
    alias!("game_get_world_state"),
    alias!("game_get_realm_info"),
    alias!("game_get_breakthrough_preview"),
    alias!("game_start_breakthrough"),
    alias!("game_submit_breakthrough"),
    // 建筑系统（7）
    alias!("game_get_buildings"),
    alias!("game_get_building_catalog"),
    alias!("game_start_building"),
    alias!("game_upgrade_building"),
    alias!("game_remove_building"),
    alias!("game_move_building"),
    alias!("game_get_build_history"),
    // 知识联动（6）
    alias!("game_get_knowledge_domains"),
    alias!("game_get_knowledge_progress"),
    alias!("game_map_kb_category"),
    alias!("game_get_kb_category_mappings"),
    alias!("game_sync_knowledge_event"),
    alias!("game_get_points_trend"),
    // 突破历史 + 世界列表（3）
    alias!("game_get_breakthrough_history"),
    alias!("game_list_worlds"),
    alias!("game_delete_world"),
    // 统计聚合便利（2）
    alias!("game_get_events_and_tasks"),
    alias!("game_get_build_timeline"),
    // 时间轴回放（2）
    alias!("game_get_world_snapshot"),
    alias!("game_exit_replay"),
    // NPC 系统（5）
    alias!("game_npc_list"),
    alias!("game_npc_get"),
    alias!("game_npc_history"),
    alias!("game_npc_chat"),
    alias!("game_npc_clear_history"),
    // D4.6 NPC 深化（2）
    alias!("game_npc_memories"),
    alias!("game_npc_relationship"),
    // D4.7 传闻机制（1）
    alias!("game_npc_rumors"),
    // D4.3 自适应难度（1）
    alias!("game_player_skill"),
    // D4.3 游戏对手 AI（1）
    alias!("game_opponent_decide"),
    // D4.4 动态剧情（4）
    alias!("game_story_generate"),
    alias!("game_story_advance"),
    alias!("game_story_get"),
    alias!("game_story_list"),
    // D4.4 自然语言交互（1）
    alias!("game_nl_parse"),
    // D4.6 行为分析 AI 洞察（1）
    alias!("game_analyze_behavior"),
    // D4.6 数据底层智能监测接入（2）
    alias!("game_intelligence_record_event"),
    alias!("game_intelligence_status"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::game_behavior_commands;
use crate::plugins::_legacy::commands::game_commands;
use crate::plugins::_legacy::commands::game_intelligence_commands;
use crate::plugins::_legacy::commands::game_natural_language_commands;
use crate::plugins::_legacy::commands::game_opponent_commands;
use crate::plugins::_legacy::commands::game_story_commands;
use crate::db::connection::AppState;

/// `snake_case` → `camelCase`（仅用于参数键回退探测，不改变任何入参值）。
fn to_camel(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    let mut upper = false;
    for ch in key.chars() {
        if ch == '_' {
            upper = true;
            continue;
        }
        if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// 取参数：snake_case 优先，camelCase 回退。
fn get<'a>(args: &'a Json, key: &str) -> Option<&'a Json> {
    args.get(key).or_else(|| {
        let camel = to_camel(key);
        args.get(camel.as_str())
    })
}

fn arg_str(args: &Json, key: &str) -> Result<String, String> {
    get(args, key)
        .and_then(Json::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("参数 {key} 缺失或不是字符串"))
}

fn arg_opt_str(args: &Json, key: &str) -> Result<Option<String>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_str()
            .map(|s| Some(s.to_string()))
            .ok_or_else(|| format!("参数 {key} 不是字符串或 null")),
    }
}

fn arg_opt_i64(args: &Json, key: &str) -> Result<Option<i64>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| format!("参数 {key} 不是整数或 null")),
    }
}

fn arg_opt_f64(args: &Json, key: &str) -> Result<Option<f64>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_f64()
            .map(Some)
            .ok_or_else(|| format!("参数 {key} 不是数字或 null")),
    }
}

fn parse_named<T: serde::de::DeserializeOwned>(args: &Json, key: &str) -> Result<T, String> {
    get(args, key)
        .cloned()
        .ok_or_else(|| format!("参数 {key} 缺失"))
        .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
}

fn to_json<T: serde::Serialize>(value: T) -> Result<Json, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

/// 逐条分发到旧命令函数。legacy 为 IPC_ALIASES.legacy_command 值。
pub(crate) async fn dispatch_legacy<R: Runtime>(
    handle: AppHandle<R>,
    legacy: &'static str,
    args: Json,
) -> Result<Json, String> {
    let state = handle.state::<AppState>();
    match legacy {
        // ===== 世界与境界（6）=====
        "game_init_world" => {
            let player_name = arg_opt_str(&args, "player_name")?;
            to_json(game_commands::game_init_world(state, player_name).await?)
        }
        "game_get_world_state" => {
            let world_id = arg_str(&args, "world_id")?;
            to_json(game_commands::game_get_world_state(state, world_id).await?)
        }
        "game_get_realm_info" => {
            let world_id = arg_str(&args, "world_id")?;
            to_json(game_commands::game_get_realm_info(state, world_id).await?)
        }
        "game_get_breakthrough_preview" => {
            let world_id = arg_str(&args, "world_id")?;
            to_json(game_commands::game_get_breakthrough_preview(state, world_id).await?)
        }
        "game_start_breakthrough" => {
            let world_id = arg_str(&args, "world_id")?;
            let model_id = arg_opt_i64(&args, "model_id")?;
            to_json(
                game_commands::game_start_breakthrough(state, world_id, model_id).await?,
            )
        }
        "game_submit_breakthrough" => {
            let session: serde_json::Value = parse_named(&args, "session")?;
            let answers: Vec<serde_json::Value> = parse_named(&args, "answers")?;
            let model_id = arg_opt_i64(&args, "model_id")?;
            // 反序列化 session + answers 到 service 层类型
            let session: crate::plugins::_legacy::services::game_breakthrough_service::BreakthroughSession =
                serde_json::from_value(session).map_err(|e| e.to_string())?;
            let answers: Vec<crate::plugins::_legacy::services::game_breakthrough_service::BreakthroughAnswer> =
                serde_json::from_value(serde_json::to_value(answers).unwrap())
                    .map_err(|e| e.to_string())?;
            to_json(
                game_commands::game_submit_breakthrough(state, session, answers, model_id).await?,
            )
        }
        // ===== 建筑系统（7）=====
        "game_get_buildings" => {
            let world_id = arg_str(&args, "world_id")?;
            let status = arg_opt_str(&args, "status")?;
            to_json(game_commands::game_get_buildings(state, world_id, status).await?)
        }
        "game_get_building_catalog" => {
            to_json(game_commands::game_get_building_catalog(state).await?)
        }
        "game_start_building" => {
            let request: crate::plugins::_legacy::services::game_service::StartBuildingRequest =
                parse_named(&args, "request")?;
            to_json(game_commands::game_start_building(state, request).await?)
        }
        "game_upgrade_building" => {
            let building_id = arg_str(&args, "building_id")?;
            let upgrade_cost = arg_opt_i64(&args, "upgrade_cost")?
                .ok_or_else(|| "参数 upgrade_cost 缺失".to_string())?;
            to_json(
                game_commands::game_upgrade_building(
                    state,
                    building_id,
                    upgrade_cost as i64,
                )
                .await?,
            )
        }
        "game_remove_building" => {
            let building_id = arg_str(&args, "building_id")?;
            let refund_ratio = arg_opt_f64(&args, "refund_ratio")?;
            to_json(
                game_commands::game_remove_building(state, building_id, refund_ratio).await?,
            )
        }
        "game_move_building" => {
            let building_id = arg_str(&args, "building_id")?;
            let new_pos_x = arg_opt_f64(&args, "new_pos_x")?
                .ok_or_else(|| "参数 new_pos_x 缺失".to_string())?;
            let new_pos_y = arg_opt_f64(&args, "new_pos_y")?
                .ok_or_else(|| "参数 new_pos_y 缺失".to_string())?;
            let new_pos_z = arg_opt_f64(&args, "new_pos_z")?
                .ok_or_else(|| "参数 new_pos_z 缺失".to_string())?;
            let new_rotation_y = arg_opt_f64(&args, "new_rotation_y")?;
            let move_cost = arg_opt_i64(&args, "move_cost")?
                .ok_or_else(|| "参数 move_cost 缺失".to_string())?;
            to_json(
                game_commands::game_move_building(
                    state,
                    building_id,
                    new_pos_x,
                    new_pos_y,
                    new_pos_z,
                    new_rotation_y,
                    move_cost as i64,
                )
                .await?,
            )
        }
        "game_get_build_history" => {
            let world_id = arg_str(&args, "world_id")?;
            let limit = arg_opt_i64(&args, "limit")?;
            to_json(
                game_commands::game_get_build_history(state, world_id, limit).await?,
            )
        }
        // ===== 知识联动（5）=====
        "game_get_knowledge_domains" => {
            to_json(game_commands::game_get_knowledge_domains(state).await?)
        }
        "game_get_knowledge_progress" => {
            let world_id = arg_str(&args, "world_id")?;
            let domain_id = arg_opt_str(&args, "domain_id")?;
            to_json(
                game_commands::game_get_knowledge_progress(state, world_id, domain_id).await?,
            )
        }
        "game_map_kb_category" => {
            let category_id = arg_opt_i64(&args, "category_id")?
                .ok_or_else(|| "参数 category_id 缺失".to_string())?;
            let domain_id = arg_str(&args, "domain_id")?;
            to_json(
                game_commands::game_map_kb_category(state, category_id as i64, domain_id).await?,
            )
        }
        "game_get_kb_category_mappings" => {
            to_json(game_commands::game_get_kb_category_mappings(state).await?)
        }
        "game_sync_knowledge_event" => {
            let world_id = arg_str(&args, "world_id")?;
            let event_type = arg_str(&args, "event_type")?;
            let domain_id = arg_str(&args, "domain_id")?;
            let points = arg_opt_i64(&args, "points")?
                .ok_or_else(|| "参数 points 缺失".to_string())?;
            let source_data: serde_json::Value = parse_named(&args, "source_data")?;
            to_json(
                game_commands::game_sync_knowledge_event(
                    state,
                    world_id,
                    event_type,
                    domain_id,
                    points as i32,
                    source_data,
                )
                .await?,
            )
        }
        "game_get_points_trend" => {
            let world_id = arg_str(&args, "world_id")?;
            let days = arg_opt_i64(&args, "days")?;
            to_json(
                game_commands::game_get_points_trend(
                    state,
                    world_id,
                    days.map(|d| d as u32),
                )
                .await?,
            )
        }
        // ===== 突破历史 + 世界列表（3）=====
        "game_get_breakthrough_history" => {
            let world_id = arg_str(&args, "world_id")?;
            let limit = arg_opt_i64(&args, "limit")?;
            to_json(
                game_commands::game_get_breakthrough_history(state, world_id, limit).await?,
            )
        }
        "game_list_worlds" => {
            to_json(game_commands::game_list_worlds(state).await?)
        }
        "game_delete_world" => {
            let world_id = arg_str(&args, "world_id")?;
            to_json(game_commands::game_delete_world(state, world_id).await?)
        }
        // ===== 统计聚合便利（2）=====
        "game_get_events_and_tasks" => {
            let world_id = arg_str(&args, "world_id")?;
            to_json(game_commands::game_get_events_and_tasks(state, world_id).await?)
        }
        "game_get_build_timeline" => {
            let world_id = arg_str(&args, "world_id")?;
            let from_time = arg_opt_i64(&args, "from_time")?;
            let to_time = arg_opt_i64(&args, "to_time")?;
            to_json(
                game_commands::game_get_build_timeline(state, world_id, from_time, to_time).await?,
            )
        }
        // ===== 时间轴回放（2）=====
        "game_get_world_snapshot" => {
            let world_id = arg_str(&args, "world_id")?;
            let timestamp = arg_opt_i64(&args, "timestamp")?
                .ok_or_else(|| "参数 timestamp 缺失".to_string())?;
            to_json(
                game_commands::game_get_world_snapshot(state, world_id, timestamp).await?,
            )
        }
        "game_exit_replay" => {
            let world_id = arg_str(&args, "world_id")?;
            to_json(game_commands::game_exit_replay(state, world_id).await?)
        }
        // ===== NPC 系统（5）=====
        "game_npc_list" => {
            to_json(game_commands::game_npc_list(state).await?)
        }
        "game_npc_get" => {
            let npc_id = arg_str(&args, "npc_id")?;
            to_json(game_commands::game_npc_get(state, npc_id).await?)
        }
        "game_npc_history" => {
            let world_id = arg_str(&args, "world_id")?;
            let npc_id = arg_str(&args, "npc_id")?;
            let limit = arg_opt_i64(&args, "limit")?;
            to_json(
                game_commands::game_npc_history(state, world_id, npc_id, limit).await?,
            )
        }
        "game_npc_chat" => {
            let world_id = arg_str(&args, "world_id")?;
            let npc_id = arg_str(&args, "npc_id")?;
            let message = arg_str(&args, "message")?;
            let model_id = arg_opt_i64(&args, "model_id")?;
            to_json(
                game_commands::game_npc_chat(state, world_id, npc_id, message, model_id).await?,
            )
        }
        "game_npc_clear_history" => {
            let world_id = arg_str(&args, "world_id")?;
            let npc_id = arg_str(&args, "npc_id")?;
            to_json(
                game_commands::game_npc_clear_history(state, world_id, npc_id).await?,
            )
        }
        // ===== D4.6 NPC 深化（2）=====
        "game_npc_memories" => {
            let world_id = arg_str(&args, "world_id")?;
            let npc_id = arg_str(&args, "npc_id")?;
            let limit = arg_opt_i64(&args, "limit")?;
            to_json(
                game_commands::game_npc_memories(state, world_id, npc_id, limit).await?,
            )
        }
        "game_npc_relationship" => {
            let world_id = arg_str(&args, "world_id")?;
            let npc_id = arg_str(&args, "npc_id")?;
            to_json(
                game_commands::game_npc_relationship(state, world_id, npc_id).await?,
            )
        }
        // ===== D4.7 传闻机制（1）=====
        "game_npc_rumors" => {
            let world_id = arg_str(&args, "world_id")?;
            let npc_id = arg_str(&args, "npc_id")?;
            let limit = arg_opt_i64(&args, "limit")?;
            to_json(
                game_commands::game_npc_rumors(state, world_id, npc_id, limit).await?,
            )
        }
        // ===== D4.3 自适应难度（1）=====
        "game_player_skill" => {
            let world_id = arg_str(&args, "world_id")?;
            to_json(game_commands::game_player_skill(state, world_id).await?)
        }
        // ===== D4.3 游戏对手 AI（1）=====
        "game_opponent_decide" => {
            let request: crate::plugins::_legacy::services::game_opponent_service::OpponentDecisionRequest =
                parse_named(&args, "request")?;
            to_json(
                game_opponent_commands::game_opponent_decide(state, request).await?,
            )
        }
        // ===== D4.4 动态剧情（4）=====
        "game_story_generate" => {
            let request: crate::plugins::_legacy::services::game_story_service::GenerateStoryRequest =
                parse_named(&args, "request")?;
            to_json(game_story_commands::game_story_generate(state, request).await?)
        }
        "game_story_advance" => {
            let request: crate::plugins::_legacy::services::game_story_service::AdvanceStoryRequest =
                parse_named(&args, "request")?;
            to_json(game_story_commands::game_story_advance(state, request).await?)
        }
        "game_story_get" => {
            let story_id = arg_str(&args, "story_id")?;
            to_json(game_story_commands::game_story_get(state, story_id).await?)
        }
        "game_story_list" => {
            let limit = arg_opt_i64(&args, "limit")?;
            to_json(game_story_commands::game_story_list(state, limit).await?)
        }
        // ===== D4.4 自然语言交互（1）=====
        "game_nl_parse" => {
            let request: crate::plugins::_legacy::services::game_natural_language_service::ParseCommandRequest =
                parse_named(&args, "request")?;
            to_json(game_natural_language_commands::game_nl_parse(state, request).await?)
        }
        // ===== D4.6 行为分析 AI 洞察（1）=====
        "game_analyze_behavior" => {
            let request: crate::plugins::_legacy::services::game_behavior_analyzer_service::AnalyzeBehaviorRequest =
                parse_named(&args, "request")?;
            to_json(game_behavior_commands::game_analyze_behavior(state, request).await?)
        }
        // ===== D4.6 数据底层智能监测接入（2）=====
        "game_intelligence_record_event" => {
            let event_type = arg_str(&args, "event_type")?;
            let detail = arg_opt_str(&args, "detail")?;
            to_json(
                game_intelligence_commands::game_intelligence_record_event(
                    state,
                    event_type,
                    detail,
                )
                .await?,
            )
        }
        "game_intelligence_status" => {
            to_json(game_intelligence_commands::game_intelligence_status(state).await?)
        }
        other => Err(format!("boards.game dispatcher 未登记命令: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "game_init_world",
        "game_get_world_state",
        "game_get_realm_info",
        "game_get_breakthrough_preview",
        "game_start_breakthrough",
        "game_submit_breakthrough",
        "game_get_buildings",
        "game_get_building_catalog",
        "game_start_building",
        "game_upgrade_building",
        "game_remove_building",
        "game_move_building",
        "game_get_build_history",
        "game_get_knowledge_domains",
        "game_get_knowledge_progress",
        "game_map_kb_category",
        "game_get_kb_category_mappings",
        "game_sync_knowledge_event",
        "game_get_points_trend",
        "game_get_breakthrough_history",
        "game_list_worlds",
        "game_delete_world",
        "game_get_events_and_tasks",
        "game_get_build_timeline",
        "game_get_world_snapshot",
        "game_exit_replay",
        "game_npc_list",
        "game_npc_get",
        "game_npc_history",
        "game_npc_chat",
        "game_npc_clear_history",
        "game_npc_memories",
        "game_npc_relationship",
        "game_npc_rumors",
        "game_player_skill",
        "game_opponent_decide",
        "game_story_generate",
        "game_story_advance",
        "game_story_get",
        "game_story_list",
        "game_nl_parse",
        "game_analyze_behavior",
        "game_intelligence_record_event",
        "game_intelligence_status",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 44);
        assert_eq!(actual.len(), IPC_ALIASES.len(), "legacy_command 存在重复");
        assert_eq!(actual, expected);
    }

    #[test]
    fn alias_map_uses_formal_id_and_unique_short_code_commands() {
        let new_commands: HashSet<_> = IPC_ALIASES.iter().map(|alias| alias.new_command).collect();

        assert_eq!(new_commands.len(), IPC_ALIASES.len(), "new_command 存在重复");
        for alias in IPC_ALIASES {
            assert_eq!(alias.plugin_id, PLUGIN_ID);
            assert_eq!(alias.short_code, SHORT_CODE);
            assert_eq!(
                alias.new_command,
                format!("{SHORT_CODE}:plugin:{}", alias.legacy_command)
            );
        }
    }
}