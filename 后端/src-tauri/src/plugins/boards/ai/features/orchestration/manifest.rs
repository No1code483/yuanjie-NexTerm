//! ai.orchestration 插件清单（L2 Feature，parent boards.ai）。
//!
//! 群聊编排：纯前端视图（编排多个 AI 参与者的发言顺序/角色），数据来自 L1
//! 页面已加载的参与者/会话状态，无自有命令、无自有表。
//! 可选子插件：可独立停用/删除，停用后群聊编排入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "ai.orchestration".into(),
        name: "群聊编排".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.ai".into()),
        slot: Some("ai.orchestration".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（编排数据来自 L1 页面已加载状态，不新增查询）。
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
        i18n_namespace: "ai.orchestration".into(),
    })
}
