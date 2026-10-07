//! knowledge.graph 插件清单（批C3；L2 Feature，parent boards.knowledge）。
//!
//! 关系图谱：纯前端视图（GraphView 渲染条目/分类关联图），数据来自 L1 页面
//! 已加载的条目/分类/标签状态，无自有命令、无自有表。
//! 可选子插件（手稿 20260926）：可独立停用/删除，停用后图谱入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "knowledge.graph".into(),
        name: "关系图谱".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.knowledge".into()),
        slot: Some("knowledge.graph".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（图谱数据来自 L1 页面已加载状态，不新增查询）。
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
        i18n_namespace: "knowledge.graph".into(),
    })
}
