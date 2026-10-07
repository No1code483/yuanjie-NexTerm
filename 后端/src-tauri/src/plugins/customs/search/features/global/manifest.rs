//! search.global 插件清单（L2 Feature，parent customs.search）。
//!
//! 全站搜索：纯前端视图（跨模块统一搜索入口），数据来自 L1 页面已加载状态，
//! 无自有命令、无自有表。可选子插件：可独立停用/删除，停用后全站搜索入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "search.global".into(),
        name: "全站搜索".into(),
        level: PluginLevel::Feature,
        parent: Some("customs.search".into()),
        slot: Some("search.global".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（全站搜索数据来自 L1 页面已加载状态，不新增查询）。
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
        i18n_namespace: "search.global".into(),
    })
}
