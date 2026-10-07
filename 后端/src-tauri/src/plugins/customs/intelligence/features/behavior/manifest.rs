//! intelligence.behavior 插件清单（L2 Feature，parent customs.intelligence）。
//!
//! 行为分析：纯前端视图（行为模式可视化），数据来自 L1 页面已加载状态，
//! 无自有命令、无自有表。可选子插件：可独立停用/删除，停用后行为分析入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "intelligence.behavior".into(),
        name: "行为分析".into(),
        level: PluginLevel::Feature,
        parent: Some("customs.intelligence".into()),
        slot: Some("intelligence.behavior".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（行为数据来自 L1 页面已加载状态，不新增查询）。
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
        i18n_namespace: "intelligence.behavior".into(),
    })
}
