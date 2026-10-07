//! knowledge.templates 插件清单（批C3 深度物理拆分；L2 Feature，parent boards.knowledge）。
//!
//! 插入模版：kb_templates 表归属自 L1 迁出（features/templates/migrations/0001_baseline.sql
//! 执行 UPDATE 归属迁移）+ 4 条模版命令自 L1 迁入（短码 kt，旧命令名不变）。
//! 可选子插件（手稿 20260926）：可独立停用/删除，停用后模版入口隐藏。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "knowledge.templates".into(),
        name: "插入模版".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.knowledge".into()),
        slot: Some("knowledge.templates".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // kb_templates 自 L1 迁入（表 DDL 仍由主应用迁移创建，本插件登记归属）。
            db: vec!["kb_templates".into()],
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 4 条模版命令（kb_get/create/update/delete_template，短码 kt）。
            ipc: vec!["kt_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "knowledge.templates".into(),
    })
}
