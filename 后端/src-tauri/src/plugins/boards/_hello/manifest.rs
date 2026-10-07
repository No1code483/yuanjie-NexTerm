//! _hello 插件清单（B8；契约：06_Rust代码契约.md §八 + §十一修订④）
//!
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel, SlotSpec};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "_hello".into(),
        name: "Hello 插件化".into(),
        level: PluginLevel::Board,
        parent: None,
        slot: None,
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            db: vec!["hw_notes".into()],
            events: EventPerms {
                subscribe: vec!["_hello:*".into()],
                // 修订④：发布域必须等于插件 id（validate_manifest 强校验）
                publish: vec!["_hello:*".into()],
            },
            ipc: vec!["_hello_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![SlotSpec {
            id: "hello.panel".into(),
            r#type: "panel".into(),
            description: "演示插槽".into(),
            capacity: 2,
            route_prefix: None,
        }],
        i18n_namespace: "_hello".into(),
    })
}
