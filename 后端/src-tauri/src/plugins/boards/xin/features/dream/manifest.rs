//! xin.dream 插件清单（L2 Feature，parent boards.xin）。
//!
//! 小欣「一切皆插件」拆分：纯前端面板，无自有命令/表。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "xin.dream".into(),
        name: "梦境".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.xin".into()),
        slot: Some("xin.dream".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（纯前端面板）。
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
        i18n_namespace: "xin.dream".into(),
    })
}
