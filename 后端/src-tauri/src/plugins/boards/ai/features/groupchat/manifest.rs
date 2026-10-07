//! ai.groupchat 插件清单（阶段3 批次2b-1；L2 Feature，parent boards.ai）。
//!
//! 覆盖面：群聊编排状态查询 + 强制结束 2 条命令（`ai_get_orchestration_status` /
//! `ai_end_group_chat`）——本批仅**骨架占位**，业务实现复用 `ai_commands` 原函数；
//! 其消费的会话/消息面（chat_service）与 4 张会话表归属属批次 2b-2。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "ai.groupchat".into(),
        name: "辩论群聊".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.ai".into()),
        slot: Some("ai.groupchat".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 本 L2 不持有业务表：编排状态经 `chat_service::get_messages` 读取
            // （messages 表归属属批次 2b-2），强制结束经 `chat_service::stop_generation`。
            db: vec![],
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
            ipc: vec!["gc_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "ai.groupchat".into(),
    })
}