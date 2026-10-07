//! ai.prompts 插件清单（L2 Feature，parent boards.ai）。
//!
//! 提示词模板：纯前端视图（管理/选用对话提示词模板），模板数据来自 L1 页面
//! 已加载状态，无自有命令、无自有表。
//! 可选子插件：可独立停用/删除，停用后提示词模板入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "ai.prompts".into(),
        name: "提示词模板".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.ai".into()),
        slot: Some("ai.prompts".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（模板数据来自 L1 页面已加载状态，不新增查询）。
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
        i18n_namespace: "ai.prompts".into(),
    })
}
