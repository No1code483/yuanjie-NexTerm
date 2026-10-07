//! knowledge.material 插件清单（批C3；L2 Feature，parent boards.knowledge）。
//!
//! 资料库视图（library='material'）：必备子插件（手稿 20260926），不可单独停用，
//! 仅随父插件 boards.knowledge 同步停用。无自有命令、无自有表（kb_* 命令与表
//! 归属 L1，库切换仅为 kb_categories.library 字段值维度）。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "knowledge.material".into(),
        name: "资料库".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.knowledge".into()),
        slot: Some("knowledge.material".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无自有表（资料/学习两库共用 L1 的 kb_categories/kb_entries 等表）。
            db: vec![],
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 无自有 IPC（kb_* 全集归属 L1）。
            ipc: vec![],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "knowledge.material".into(),
    })
}
