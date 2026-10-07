//! boards.xin（L1）服务边界说明。
//!
//! 批次4a-1：L1 直接承载 `xin_basic_commands` 28 条命令 + 6 张表（`xin_config` /
//! `xin_memories` / `xin_summaries` / `xin_moods` / `xin_persona_memories` /
//! `xin_persona_switch_log`）。实现层（`xiaoxin_service` / `xin_personality_service` /
//! `xin_emotion_service` / `xin_tts_service` / `xin_stt_service` / `xin_multimodal_service`）
//! 仍留主应用，dispatcher 复用（2b-2 裁定 15 同口径，实现层迁移登记阶段4）。
//! 跨段依赖：`xin_memories` 另由 4a-2 段 `xin_dialogue_service` 写入（跨段写登记）。

pub const OWNED_TABLES: &[&str] = &[
    "xin_config",
    "xin_memories",
    "xin_summaries",
    "xin_moods",
    "xin_persona_memories",
    "xin_persona_switch_log",
];