//! terminal.yuancode.agent 插件清单（L3 Feature，parent terminal.yuancode）。
//!
//! Agent 与目标：纯前端视图（Agent 任务/目标管理视图），无自有命令、无自有表。
//! 可选子插件：可独立停用/删除，停用后 Agent 与目标入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "terminal.yuancode.agent".into(),
        name: "Agent 与目标".into(),
        level: PluginLevel::Feature,
        parent: Some("terminal.yuancode".into()),
        slot: Some("terminal.yuancode.agent".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（Agent 与目标数据由 L2 terminal.yuancode 登记）。
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
        i18n_namespace: "terminal.yuancode.agent".into(),
    })
}
