//! ai.sessions 插件服务边界说明。
//!
//! batchC1：会话面服务 —— 归属声明参见 manifest。业务实现复用 chat_commands
//! (裁定 11：命令面收编到 ai.sessions，实现层仍留主应用，迁移登记阶段4）。
//!
//! 本 L2 不直接持有业务表（tables 归属登记在 manifest.db）：
//! - conversations / conversation_partners / messages / prompt_templates
//!   均在 boards.ai L1 migrations/0001_baseline.sql 登记 name_prefixed=0。
//! - ai.sessions 声明 db 权限，由内核校验。

pub const OWNED_TABLES: &[&str] = &[
    "conversations",
    "conversation_participants",
    "messages",
    "prompt_templates",
];
