//! boards.xin 的 L2 Feature 插件集合。
//! 阶段3 批次4a-1：xin.wellness（健康助手：提醒 / 习惯 / 番茄钟，17 命令，短码 xw）。
//! 阶段3 批次4a-1：xin.realtime（实时语音对话，4 命令，短码 xr）。
//! 阶段3 批次4a-2：xin.orchestration（编排面 87 命令，短码 xo）。
//! 小欣「一切皆插件」拆分：12 个纯前端面板子插件，无命令无表（chat 必备）。

pub mod briefing;
pub mod chat;
pub mod checkpoint;
pub mod compaction;
pub mod dream;
pub mod evolution;
pub mod memory;
pub mod mood;
pub mod orchestration;
pub mod realtime;
pub mod review;
pub mod search;
pub mod skill;
pub mod tool;
pub mod wellness;