//! boards.ai（L1）服务边界说明。
//!
//! 2b-1：L1 骨架 + 插槽声明；ai_models / ai_agents 归属由对应 L2（ai.models /
//! ai.agent）自行登记。
//! 2b-2（裁定 12）：L1 承载会话面 18 命令。实现层（chat_service / chat_repo /
//! ai_orchestrator / models）沿用 2b-1 模式仍留主应用，dispatcher 复用
//! （裁定 15，实现层迁移登记阶段4）。
//! BUG-033 修订（2026-09-27）：4 张会话表归属已转移至 ai.sessions，L1 不再持有表。

pub const OWNED_TABLES: &[&str] = &[];