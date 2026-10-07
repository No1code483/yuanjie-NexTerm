//! boards.xin 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发（阶段3 批次4a-1）。
//!
//! 与 boards.terminal / boards.ai 同构：alias 只做旧名 → 逻辑名的映射登记；真实 handler
//! 在 `dispatch_legacy` —— 闭包仅捕获 AppHandle，运行时经 `handle.state::<AppState>()`
//! 解析主应用真实 AppState（契约 06_Rust代码契约 §8.1，pool / 认证 / 服务全部来源于此，
//! 禁止伪造或第二池）；业务实现复用 `xin_basic_commands` 原函数，返回值序列化与旧 IPC
//! 路径一致。
//!
//! **参数键口径**：Tauri v2 默认 `ArgumentCase::Camel`；本 dispatcher 以 snake_case 为先、
//! camelCase 回退，兼容两种口径，不按 transport 静默改写输入（《测试流程》IPC 双轨回放
//! 纪律第 1 条）。

pub const PLUGIN_ID: &str = "boards.xin";
pub const SHORT_CODE: &str = "xn";

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
            new_command: concat!("xn:plugin:", $command),
        }
    };
}

/// 28 条 alias：`xin_basic_commands` 全量（裁定 1：零消费 17 条全部「留 + alias」）。
/// `main.rs` 旧 transport 于本批 S7 移除。
pub const IPC_ALIASES: &[IpcAlias] = &[
    // 配置 / 人格（5）
    alias!("xin_get_config"),
    alias!("xin_set_config"),
    alias!("xin_get_personas"),
    alias!("xin_get_active_persona"),
    alias!("xin_set_active_persona"),
    // 人格记忆 / 生长（5）
    alias!("xin_persona_memory"),
    alias!("xin_persona_switch_history"),
    alias!("xin_memorized_personas"),
    alias!("xin_grow_persona"),
    alias!("xin_self_growing_persona"),
    // 记忆（4）
    alias!("xin_save_memory"),
    alias!("xin_get_memories"),
    alias!("xin_search_memories"),
    alias!("xin_delete_memory"),
    // 情绪 / 情感（5）
    alias!("xin_analyze_sentiment"),
    alias!("xin_get_tts_status"),
    alias!("xin_tts_speak"),
    alias!("xin_process_multimodal"),
    alias!("xin_get_mood"),
    alias!("xin_update_mood"),
    alias!("xin_emotion_trend"),
    // 摘要 / 简报 / 洞察（4）
    alias!("xin_add_summary"),
    alias!("xin_get_summaries"),
    alias!("xin_daily_briefing"),
    alias!("xin_personality_insights"),
    // 语音（3）
    alias!("xin_voice_input"),
    alias!("xin_tts"),
    alias!("xin_tts_list_voices"),
];

// ========== dispatcher 业务 handler 分发 ==========

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::xin_basic_commands;
use crate::db::connection::AppState;
use crate::models::xin::{
    ConversationSummary, MemoryCategory, MultimodalInput, TtsSpeakRequest, UserMemory, XinConfig,
};

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

fn parse_opt_named<T: serde::de::DeserializeOwned>(
    args: &Json,
    key: &str,
) -> Result<Option<T>, String> {
    match get(args, key) {
        None | Some(Json::Null) => Ok(None),
        Some(v) => serde_json::from_value(v.clone())
            .map(Some)
            .map_err(|e| e.to_string()),
    }
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
        // ===== 配置 / 人格（5）=====
        "xin_get_config" => to_json(xin_basic_commands::xin_get_config(state).await?),
        "xin_set_config" => {
            let config: XinConfig = parse_named(&args, "config")?;
            to_json(xin_basic_commands::xin_set_config(state, config).await?)
        }
        "xin_get_personas" => to_json(xin_basic_commands::xin_get_personas(state).await?),
        "xin_get_active_persona" => {
            to_json(xin_basic_commands::xin_get_active_persona(state).await?)
        }
        "xin_set_active_persona" => {
            let persona_id = arg_str(&args, "persona_id")?;
            to_json(xin_basic_commands::xin_set_active_persona(state, persona_id).await?)
        }
        // ===== 人格记忆 / 生长（5）=====
        "xin_persona_memory" => {
            let persona_id = arg_str(&args, "persona_id")?;
            to_json(xin_basic_commands::xin_persona_memory(state, persona_id).await?)
        }
        "xin_persona_switch_history" => {
            let limit = arg_opt_i64(&args, "limit")?;
            to_json(xin_basic_commands::xin_persona_switch_history(state, limit).await?)
        }
        "xin_memorized_personas" => {
            to_json(xin_basic_commands::xin_memorized_personas(state).await?)
        }
        "xin_grow_persona" => {
            let profile: crate::plugins::_legacy::services::xin_personality_service::UserProfile =
                parse_named(&args, "profile")?;
            to_json(xin_basic_commands::xin_grow_persona(state, profile).await?)
        }
        "xin_self_growing_persona" => {
            to_json(xin_basic_commands::xin_self_growing_persona(state).await?)
        }
        // ===== 记忆（4）=====
        "xin_save_memory" => {
            let memory: UserMemory = parse_named(&args, "memory")?;
            to_json(xin_basic_commands::xin_save_memory(state, memory).await?)
        }
        "xin_get_memories" => {
            let category: Option<MemoryCategory> = parse_opt_named(&args, "category")?;
            let limit: Option<usize> = parse_opt_named(&args, "limit")?;
            to_json(xin_basic_commands::xin_get_memories(state, category, limit).await?)
        }
        "xin_search_memories" => {
            let query = arg_str(&args, "query")?;
            to_json(xin_basic_commands::xin_search_memories(state, query).await?)
        }
        "xin_delete_memory" => {
            let id = arg_str(&args, "id")?;
            to_json(xin_basic_commands::xin_delete_memory(state, id).await?)
        }
        // ===== 情绪 / 情感 / 多模态 / TTS 状态（7）=====
        "xin_analyze_sentiment" => {
            let text = arg_str(&args, "text")?;
            to_json(xin_basic_commands::xin_analyze_sentiment(state, text).await?)
        }
        "xin_get_tts_status" => to_json(xin_basic_commands::xin_get_tts_status(state).await?),
        "xin_tts_speak" => {
            let request: TtsSpeakRequest = parse_named(&args, "request")?;
            to_json(xin_basic_commands::xin_tts_speak(state, request).await?)
        }
        "xin_process_multimodal" => {
            let input: MultimodalInput = parse_named(&args, "input")?;
            to_json(xin_basic_commands::xin_process_multimodal(state, input).await?)
        }
        "xin_get_mood" => to_json(xin_basic_commands::xin_get_mood(state).await?),
        "xin_update_mood" => {
            let text = arg_str(&args, "text")?;
            to_json(xin_basic_commands::xin_update_mood(state, text).await?)
        }
        "xin_emotion_trend" => {
            let limit = arg_opt_i64(&args, "limit")?;
            to_json(xin_basic_commands::xin_emotion_trend(state, limit).await?)
        }
        // ===== 摘要 / 简报 / 洞察（4）=====
        "xin_add_summary" => {
            let summary: ConversationSummary = parse_named(&args, "summary")?;
            to_json(xin_basic_commands::xin_add_summary(state, summary).await?)
        }
        "xin_get_summaries" => {
            let limit: Option<usize> = parse_opt_named(&args, "limit")?;
            to_json(xin_basic_commands::xin_get_summaries(state, limit).await?)
        }
        "xin_daily_briefing" => to_json(xin_basic_commands::xin_daily_briefing(state).await?),
        "xin_personality_insights" => {
            to_json(xin_basic_commands::xin_personality_insights(state).await?)
        }
        // ===== 语音（3）=====
        "xin_voice_input" => {
            let audio_path = arg_str(&args, "audio_path")?;
            let language = arg_opt_str(&args, "language")?;
            let model_id = arg_opt_i64(&args, "model_id")?;
            to_json(
                xin_basic_commands::xin_voice_input(state, audio_path, language, model_id).await?,
            )
        }
        "xin_tts" => {
            let text = arg_str(&args, "text")?;
            let voice = arg_opt_str(&args, "voice")?;
            let speed = arg_opt_f64(&args, "speed")?;
            let pitch = arg_opt_f64(&args, "pitch")?;
            to_json(xin_basic_commands::xin_tts(state, text, voice, speed, pitch).await?)
        }
        "xin_tts_list_voices" => {
            to_json(xin_basic_commands::xin_tts_list_voices(state).await?)
        }
        other => Err(format!("boards.xin dispatcher 未登记命令: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "xin_get_config",
        "xin_set_config",
        "xin_get_personas",
        "xin_get_active_persona",
        "xin_set_active_persona",
        "xin_persona_memory",
        "xin_persona_switch_history",
        "xin_memorized_personas",
        "xin_grow_persona",
        "xin_self_growing_persona",
        "xin_save_memory",
        "xin_get_memories",
        "xin_search_memories",
        "xin_delete_memory",
        "xin_analyze_sentiment",
        "xin_get_tts_status",
        "xin_tts_speak",
        "xin_process_multimodal",
        "xin_get_mood",
        "xin_update_mood",
        "xin_emotion_trend",
        "xin_add_summary",
        "xin_get_summaries",
        "xin_daily_briefing",
        "xin_personality_insights",
        "xin_voice_input",
        "xin_tts",
        "xin_tts_list_voices",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 28);
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