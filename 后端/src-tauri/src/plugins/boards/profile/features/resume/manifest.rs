//! profile.resume 插件清单（L2 Feature，parent boards.profile）。
//!
//! 简历：纯前端视图（简历信息展示/编辑），数据来自 L1 Profile 页已加载状态，
//! 无自有命令、无自有表。可选子插件：可独立停用/删除，停用后简历入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "profile.resume".into(),
        name: "简历".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.profile".into()),
        slot: Some("profile.resume".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（简历数据来自 L1 页面已加载状态，不新增查询）。
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
        i18n_namespace: "profile.resume".into(),
    })
}
