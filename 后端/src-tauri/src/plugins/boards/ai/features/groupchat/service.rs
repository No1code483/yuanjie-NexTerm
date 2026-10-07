//! ai.groupchat 插件服务边界说明。
//!
//! 无自有表，无基线迁移。
//! 暂跨引用（登记批次 2b-2 收口）：`ai_get_orchestration_status` 经
//! `chat_service::get_messages` 汇总轮次/tokens（messages 表属 2b-2）；
//! `ai_end_group_chat` 经 `chat_service::stop_generation` 写 `state.force_stop_flags`。

pub const OWNED_TABLES: &[&str] = &[];