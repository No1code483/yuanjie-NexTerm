//! auth.recovery 插件清单（L2 Feature，parent customs.auth）。
//!
//! 认证「一切皆插件」拆分：纯前端视图，无自有命令/表。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "auth.recovery".into(),
        name: "找回密码".into(),
        level: PluginLevel::Feature,
        parent: Some("customs.auth".into()),
        slot: Some("auth.recovery".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            db: vec![],
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            ipc: vec![],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "auth.recovery".into(),
    })
}
