//! xin.wellness 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发（阶段3 批次4a-1）。
//!
//! 与 boards.xin L1 同构：alias 只做旧名 → 逻辑名的映射登记；真实 handler 在
//! `dispatch_legacy`，运行时经 `handle.state::<AppState>()` 解析主应用真实 AppState。
//! 业务实现复用 `xin_wellness_commands` 原函数（含裁定 3 的按 `user_id` 分片收口）。
//!
//! **参数键口径**：Tauri v2 默认 `ArgumentCase::Camel`；本 dispatcher 以 snake_case 为先、
//! camelCase 回退，兼容两种口径。

pub const PLUGIN_ID: &str = "xin.wellness";
pub const SHORT_CODE: &str = "xw";

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
            new_command: concat!("xw:plugin:", $command),
        }
    };
}

/// 17 条 alias：`xin_wellness_commands` 全量（裁定 1：零消费全族「留 + alias」，
/// 登记「前端未接线」功能缺口）。`main.rs` 旧 transport 于本批 S7 移除。
pub const IPC_ALIASES: &[IpcAlias] = &[
    // 情绪 / 记忆 / 会话桥（4）
    alias!("xin_v2_mood_history"),
    alias!("xin_v2_consolidate_memories"),
    alias!("xin_v2_memory_links"),
    alias!("xin_v2_conversation_bridge"),
    // 提醒（4）
    alias!("xin_v2_add_reminder"),
    alias!("xin_v2_list_reminders"),
    alias!("xin_v2_dismiss_reminder"),
    alias!("xin_v2_delete_reminder"),
    // 习惯（3）
    alias!("xin_v2_register_habit"),
    alias!("xin_v2_checkin_habit"),
    alias!("xin_v2_list_habits"),
    // 番茄钟（4）
    alias!("xin_v2_pomodoro_start"),
    alias!("xin_v2_pomodoro_complete_cycle"),
    alias!("xin_v2_pomodoro_stop"),
    alias!("xin_v2_pomodoro_status"),
    // 摘要 / 演化（2）
    alias!("xin_v2_activity_digest"),
    alias!("xin_v2_personality_evolution"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::xin_wellness_commands;
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

fn arg_u32(args: &Json, key: &str) -> Result<u32, String> {
    get(args, key)
        .and_then(Json::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| format!("参数 {key} 缺失或不是 u32 整数"))
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
        // ===== 情绪 / 记忆 / 会话桥（4）=====
        "xin_v2_mood_history" => {
            let trigger_text = arg_opt_str(&args, "trigger_text")?;
            to_json(xin_wellness_commands::xin_v2_mood_history(state, trigger_text).await?)
        }
        "xin_v2_consolidate_memories" => {
            to_json(xin_wellness_commands::xin_v2_consolidate_memories(state).await?)
        }
        "xin_v2_memory_links" => {
            to_json(xin_wellness_commands::xin_v2_memory_links(state).await?)
        }
        "xin_v2_conversation_bridge" => {
            let session_id = arg_opt_str(&args, "session_id")?;
            to_json(xin_wellness_commands::xin_v2_conversation_bridge(state, session_id).await?)
        }
        // ===== 提醒（4）=====
        "xin_v2_add_reminder" => {
            let title = arg_str(&args, "title")?;
            let description = arg_str(&args, "description")?;
            let trigger_at = arg_opt_str(&args, "trigger_at")?;
            let cron_expr = arg_opt_str(&args, "cron_expr")?;
            to_json(
                xin_wellness_commands::xin_v2_add_reminder(
                    state,
                    title,
                    description,
                    trigger_at,
                    cron_expr,
                )
                .await?,
            )
        }
        "xin_v2_list_reminders" => {
            to_json(xin_wellness_commands::xin_v2_list_reminders(state).await?)
        }
        "xin_v2_dismiss_reminder" => {
            let id = arg_str(&args, "id")?;
            to_json(xin_wellness_commands::xin_v2_dismiss_reminder(state, id).await?)
        }
        "xin_v2_delete_reminder" => {
            let id = arg_str(&args, "id")?;
            to_json(xin_wellness_commands::xin_v2_delete_reminder(state, id).await?)
        }
        // ===== 习惯（3）=====
        "xin_v2_register_habit" => {
            let name = arg_str(&args, "name")?;
            let category = arg_str(&args, "category")?;
            to_json(xin_wellness_commands::xin_v2_register_habit(state, name, category).await?)
        }
        "xin_v2_checkin_habit" => {
            let habit_id = arg_str(&args, "habit_id")?;
            to_json(xin_wellness_commands::xin_v2_checkin_habit(state, habit_id).await?)
        }
        "xin_v2_list_habits" => {
            to_json(xin_wellness_commands::xin_v2_list_habits(state).await?)
        }
        // ===== 番茄钟（4）=====
        "xin_v2_pomodoro_start" => {
            let task_name = arg_str(&args, "task_name")?;
            let duration_minutes = arg_u32(&args, "duration_minutes")?;
            let break_minutes = arg_u32(&args, "break_minutes")?;
            let total_cycles = arg_u32(&args, "total_cycles")?;
            to_json(
                xin_wellness_commands::xin_v2_pomodoro_start(
                    state,
                    task_name,
                    duration_minutes,
                    break_minutes,
                    total_cycles,
                )
                .await?,
            )
        }
        "xin_v2_pomodoro_complete_cycle" => {
            to_json(xin_wellness_commands::xin_v2_pomodoro_complete_cycle(state).await?)
        }
        "xin_v2_pomodoro_stop" => {
            to_json(xin_wellness_commands::xin_v2_pomodoro_stop(state).await?)
        }
        "xin_v2_pomodoro_status" => {
            to_json(xin_wellness_commands::xin_v2_pomodoro_status(state).await?)
        }
        // ===== 摘要 / 演化（2）=====
        "xin_v2_activity_digest" => {
            let period = arg_opt_str(&args, "period")?;
            to_json(xin_wellness_commands::xin_v2_activity_digest(state, period).await?)
        }
        "xin_v2_personality_evolution" => {
            to_json(xin_wellness_commands::xin_v2_personality_evolution(state).await?)
        }
        other => Err(format!("xin.wellness dispatcher 未登记命令: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "xin_v2_mood_history",
        "xin_v2_consolidate_memories",
        "xin_v2_memory_links",
        "xin_v2_conversation_bridge",
        "xin_v2_add_reminder",
        "xin_v2_list_reminders",
        "xin_v2_dismiss_reminder",
        "xin_v2_delete_reminder",
        "xin_v2_register_habit",
        "xin_v2_checkin_habit",
        "xin_v2_list_habits",
        "xin_v2_pomodoro_start",
        "xin_v2_pomodoro_complete_cycle",
        "xin_v2_pomodoro_stop",
        "xin_v2_pomodoro_status",
        "xin_v2_activity_digest",
        "xin_v2_personality_evolution",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 17);
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