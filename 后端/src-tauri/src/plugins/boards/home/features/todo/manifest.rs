//! home.todo 插件清单（阶段3 批次1b-1；L2 Feature，parent boards.home）。
//!
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "home.todo".into(),
        name: "待办".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.home".into()),
        slot: Some("home.todo".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // todos 为既有旧表，归属登记见 migrations/0001_baseline.sql。
            db: vec!["todos".into()],
            // 批次1b-2c（裁定 24-A）：领域事件域 = 插件 id，事件名形如 `home.todo:todo.created`。
            events: EventPerms {
                subscribe: vec![],
                publish: vec!["home.todo:*".into()],
            },
            // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
            ipc: vec!["td_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "home.todo".into(),
    })
}
