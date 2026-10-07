//! ai.multimodel 插件清单（L2 Feature，parent boards.ai）。
//!
//! 多模型对比：纯前端视图（并排渲染多个模型的响应对比），数据来自 L1 页面
//! 已加载的会话/消息状态，无自有命令、无自有表。
//! 可选子插件：可独立停用/删除，停用后多模型对比入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "ai.multimodel".into(),
        name: "多模型对比".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.ai".into()),
        slot: Some("ai.multimodel".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（对比数据来自 L1 页面已加载状态，不新增查询）。
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
        i18n_namespace: "ai.multimodel".into(),
    })
}
