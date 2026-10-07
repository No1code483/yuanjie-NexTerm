//! terminal.yuancode.git 插件清单（L3 Feature，parent terminal.yuancode）。
//!
//! 版本控制：纯前端视图（Git 面板/历史/差异视图），无自有命令、无自有表。
//! 可选子插件：可独立停用/删除，停用后版本控制入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "terminal.yuancode.git".into(),
        name: "版本控制".into(),
        level: PluginLevel::Feature,
        parent: Some("terminal.yuancode".into()),
        slot: Some("terminal.yuancode.git".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（Git 数据由 L2 terminal.yuancode 登记）。
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
        i18n_namespace: "terminal.yuancode.git".into(),
    })
}
