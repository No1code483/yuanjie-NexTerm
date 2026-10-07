//! intelligence.dashboard 插件清单（L2 Feature，parent customs.intelligence）。
//!
//! 仪表盘：纯前端视图（智能层概览面板），数据来自 L1 页面已加载状态，
//! 无自有命令、无自有表。可选子插件：可独立停用/删除，停用后仪表盘入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "intelligence.dashboard".into(),
        name: "仪表盘".into(),
        level: PluginLevel::Feature,
        parent: Some("customs.intelligence".into()),
        slot: Some("intelligence.dashboard".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（仪表盘数据来自 L1 页面已加载状态，不新增查询）。
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
        i18n_namespace: "intelligence.dashboard".into(),
    })
}
