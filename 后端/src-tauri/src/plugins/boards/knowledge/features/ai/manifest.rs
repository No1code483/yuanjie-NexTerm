//! knowledge.ai 插件清单（批C3；L2 Feature，parent boards.knowledge）。
//!
//! AI 辅助：纯前端视图（条目摘要/问答等 AI 辅助入口），数据来自 L1 页面
//! 已加载的条目状态，无自有命令、无自有表。
//! 可选子插件（手稿 20260926）：可独立停用/删除，停用后 AI 辅助入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "knowledge.ai".into(),
        name: "AI 辅助".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.knowledge".into()),
        slot: Some("knowledge.ai".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（AI 辅助数据来自 L1 页面已加载状态，不新增查询）。
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
        i18n_namespace: "knowledge.ai".into(),
    })
}
