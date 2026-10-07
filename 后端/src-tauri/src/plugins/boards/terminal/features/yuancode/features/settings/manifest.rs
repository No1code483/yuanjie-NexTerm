//! terminal.yuancode.settings 插件清单（L3 Feature，parent terminal.yuancode）。
//!
//! 编辑器设置：纯前端视图（编辑器偏好/快捷键设置视图），无自有命令、无自有表。
//! 可选子插件：可独立停用/删除，停用后编辑器设置入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "terminal.yuancode.settings".into(),
        name: "编辑器设置".into(),
        level: PluginLevel::Feature,
        parent: Some("terminal.yuancode".into()),
        slot: Some("terminal.yuancode.settings".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（编辑器设置数据由 L2 terminal.yuancode 登记）。
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
        i18n_namespace: "terminal.yuancode.settings".into(),
    })
}
