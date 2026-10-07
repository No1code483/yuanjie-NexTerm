//! ai.chat 插件清单（L2 Feature，parent boards.ai）。
//!
//! 对话：纯前端视图（对话面板渲染会话消息流），数据来自 L1 页面已加载的
//! 会话/消息状态，无自有命令、无自有表。
//! 可选子插件：可独立停用/删除，停用后对话入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "ai.chat".into(),
        name: "对话".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.ai".into()),
        slot: Some("ai.chat".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（对话数据来自 L1 页面已加载状态，不新增查询）。
            db: vec![],
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 无自有 IPC。
            ipc: vec![],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "ai.chat".into(),
    })
}
