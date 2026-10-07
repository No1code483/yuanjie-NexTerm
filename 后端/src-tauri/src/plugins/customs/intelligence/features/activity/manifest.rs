//! intelligence.activity 插件清单（L2 Feature，parent customs.intelligence）。
//!
//! 活动日志：纯前端视图（活动记录列表），数据来自 L1 页面已加载状态，
//! 无自有命令、无自有表。可选子插件：可独立停用/删除，停用后活动日志入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "intelligence.activity".into(),
        name: "活动日志".into(),
        level: PluginLevel::Feature,
        parent: Some("customs.intelligence".into()),
        slot: Some("intelligence.activity".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（活动数据来自 L1 页面已加载状态，不新增查询）。
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
        i18n_namespace: "intelligence.activity".into(),
    })
}
