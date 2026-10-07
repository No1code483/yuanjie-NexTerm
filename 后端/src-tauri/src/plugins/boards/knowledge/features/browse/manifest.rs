//! knowledge.browse 插件清单（批C3；L2 Feature，parent boards.knowledge）。
//!
//! 条目浏览：纯前端视图（条目列表/分类浏览），数据来自 L1 页面
//! 已加载的条目/分类/标签状态，无自有命令、无自有表。
//! 可选子插件（手稿 20260926）：可独立停用/删除，停用后条目浏览入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "knowledge.browse".into(),
        name: "条目浏览".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.knowledge".into()),
        slot: Some("knowledge.browse".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（条目浏览数据来自 L1 页面已加载状态，不新增查询）。
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
        i18n_namespace: "knowledge.browse".into(),
    })
}
