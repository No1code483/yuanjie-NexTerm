//! game.mapping 插件清单（L2 Feature，parent boards.game）。
//!
//! 知识映射：纯前端视图（知识库条目与游戏知识域映射展示），数据来自 L1 页面
//! 已加载状态，无自有命令、无自有表。
//! 可选子插件：可独立停用/删除，停用后映射入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "game.mapping".into(),
        name: "知识映射".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.game".into()),
        slot: Some("game.mapping".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（映射数据来自 L1 页面已加载状态，不新增查询）。
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
        i18n_namespace: "game.mapping".into(),
    })
}
