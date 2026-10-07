//! xin.orchestration 插件服务边界说明。
//!
//! 批次4a-2：L2 编排面 87 条命令实现层全部留主应用（`xin_dialogue_service` /
//! `xin_context_service` / `xin_checkpoint_service` / `xin_compaction_service` /
//! `xin_michelin_service` / `xin_commit_service` / `xin_dream_service` /
//! `xin_post_process_service` / `xin_knowledge_fusion_service` /
//! `xin_conversation_review_service` / `xin_personality_evolution_service` /
//! `xin_proactive_service` / `xin_tool_service`），dispatcher 复用
//! `xin_orchestration_commands` 原函数（2b-2 裁定 15 同口径，实现层迁移登记阶段4）。
//!
//! 对话停止越权收口（裁定 3-A）：`xin_v3_dialogue_stop` 仅凭 `conversation_id`
//! 停生成，缺少主体校验 → dispatcher 层补会话归属校验（查询 `xin_conversations`
//! 的 `user_id` 匹配调用用户）。

pub const OWNED_TABLES: &[&str] = &[
    "xin_conversations",
    "xin_checkpoints",
    "xin_compaction_config",
    "xin_compaction_records",
];
