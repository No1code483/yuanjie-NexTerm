//! recycle.actions 插件清单（L2 Feature，parent customs.recycle）。
//! 回收站「一切皆插件」拆分：纯前端视图，无自有命令/表。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "recycle.actions".into(),
        name: "回收站操作".into(),
        level: PluginLevel::Feature,
        parent: Some("customs.recycle".into()),
        slot: Some("recycle.actions".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（纯前端视图）。
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
        i18n_namespace: "recycle.actions".into(),
    })
}
