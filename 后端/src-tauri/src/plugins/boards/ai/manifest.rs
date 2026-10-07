//! boards.ai 插件清单（阶段3 批次2b-1；L1 Board，AI 会话板块）。
//!
//! L1 持有导航主干 + 4 个插槽承载 ai.models / ai.sessions / ai.agent / ai.groupchat
//! 四个 L2 Feature 插件（2b-1 落地模型与 Agent 管理面；会话列表归 batchC1；
//! 群聊业务面属 2b-2）。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel, SlotSpec};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "boards.ai".into(),
        name: "AI 会话".into(),
        level: PluginLevel::Board,
        parent: None,
        slot: None,
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 裁定 12（2b-2）已随 BUG-033 修订（2026-09-27）：4 张会话表归属经
            // ai.sessions migrations/0002_transfer_ownership.sql 转移至 ai.sessions，
            // L1 不再声明（重复登记会在二次冷激活时与 ai.sessions 归属冲突 → Error）。
            // L1 仍保留会话面 18 条 IPC 命令（实现层复用，裁定 15）。
            db: vec![],
            // 本批不发布/订阅领域事件；3 条前端事件通道（ai-stream / ai-orchestrator /
            // ai-model-health-changed）为 Tauri emit，不走内核事件总线（S5 零改动）。
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 2b-2：L1 承载会话面 18 条 IPC；ACL/handler 键由内核按短码 `ai` 构造。
            ipc: vec!["ai_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![
            SlotSpec {
                id: "ai.models".into(),
                r#type: "panel".into(),
                description: "模型管理".into(),
                capacity: 1,
                route_prefix: None,
            },
            // batchC1：会话列表（AI会话必备子插件）
            SlotSpec {
                id: "ai.sessions".into(),
                r#type: "panel".into(),
                description: "会话列表".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "ai.agent".into(),
                r#type: "panel".into(),
                description: "智能体集".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "ai.groupchat".into(),
                r#type: "panel".into(),
                description: "辩论群聊".into(),
                capacity: 1,
                route_prefix: None,
            },
            // 本轮：AI 会话纯前端 L2 子插件插槽
            SlotSpec {
                id: "ai.chat".into(),
                r#type: "panel".into(),
                description: "对话".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "ai.multimodel".into(),
                r#type: "panel".into(),
                description: "多模型对比".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "ai.orchestration".into(),
                r#type: "panel".into(),
                description: "群聊编排".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "ai.prompts".into(),
                r#type: "panel".into(),
                description: "提示词模板".into(),
                capacity: 1,
                route_prefix: None,
            },
        ],
        i18n_namespace: "ai".into(),
    })
}