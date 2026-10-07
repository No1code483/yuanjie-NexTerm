//! home.timer 插件清单（阶段3 批次1b-2a；L2 Feature，parent boards.home）。
//!
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "home.timer".into(),
        name: "计时".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.home".into()),
        slot: Some("home.timer".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // timers 为既有旧表，归属登记见 migrations/0001_baseline.sql。
            db: vec!["timers".into()],
            // 本批不发布/订阅领域事件（home 域事件契约属批次 1b-2c）。
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
            ipc: vec!["ti_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "home.timer".into(),
    })
}
