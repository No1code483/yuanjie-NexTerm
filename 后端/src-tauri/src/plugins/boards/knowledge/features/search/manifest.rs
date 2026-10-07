//! knowledge.search 插件清单（批C3；L2 Feature，parent boards.knowledge）。
//!
//! 搜索：纯前端视图（条目/全文搜索界面），数据来自 L1 页面
//! 已加载的条目/分类/标签状态，无自有命令、无自有表。
//! 可选子插件（手稿 20260926）：可独立停用/删除，停用后搜索入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "knowledge.search".into(),
        name: "搜索".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.knowledge".into()),
        slot: Some("knowledge.search".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（搜索数据来自 L1 页面已加载状态，不新增查询）。
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
        i18n_namespace: "knowledge.search".into(),
    })
}
