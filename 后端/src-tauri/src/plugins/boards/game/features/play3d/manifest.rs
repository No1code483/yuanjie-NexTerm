//! game.play3d 插件清单（L2 Feature，parent boards.game）。
//!
//! 3D 游戏：纯前端视图（3D 场景渲染），数据来自 L1 页面已加载状态，
//! 无自有命令、无自有表。
//! 可选子插件：可独立停用/删除，停用后 3D 入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "game.play3d".into(),
        name: "3D 游戏".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.game".into()),
        slot: Some("game.play3d".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（3D 场景数据来自 L1 页面已加载状态，不新增查询）。
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
        i18n_namespace: "game.play3d".into(),
    })
}
