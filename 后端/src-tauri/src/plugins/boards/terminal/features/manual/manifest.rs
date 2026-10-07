//! terminal.manual 插件清单（批C2 命令手册拆分；L2 Feature，parent boards.terminal）。
//!
//! 覆盖面：命令手册 5 条路由（/terminal/manual 及 terminal/yuancode/linux/shortcuts 子页）。
//! 纯前端静态页面（无后端命令、无业务表、无领域事件）——后端半体仅登记 manifest 归属，
//! 使插件管理页可独立启停该子插件。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "terminal.manual".into(),
        name: "命令手册".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.terminal".into()),
        slot: Some("terminal.manual".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无业务表（命令手册纯静态文档，无落库）。
            db: vec![],
            // 无领域事件（纯前端页面，无 emit 点）。
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 无 IPC（纯前端静态页面，不调任何后端命令）。
            ipc: vec![],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "terminal.manual".into(),
    })
}
