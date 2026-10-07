use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel, SlotSpec};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "customs.recycle".into(),
        name: "回收站".into(),
        // 手稿 20260926：回收站为首页板块必备子插件（L1 Custom → L2 Feature 归位）
        level: PluginLevel::Feature,
        parent: Some("boards.home".into()),
        slot: Some("home.recycle".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 批次6a S1：recycle_bin 表归属（db_归属判定表 §4，migration 0025）。
            db: vec![
                "recycle_bin".into(),
            ],
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 7 条 recycle IPC 别名 + 0 条 extension/adapter。
            ipc: vec!["rc_*".into()],
            fs: vec![],
            net: vec![],
        },
        // 回收站「一切皆插件」：2 个纯前端视图 L2 子插件插槽
        slots: vec![
            SlotSpec {
                id: "recycle.list".into(),
                r#type: "panel".into(),
                description: "回收站列表".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "recycle.actions".into(),
                r#type: "panel".into(),
                description: "回收站操作".into(),
                capacity: 1,
                route_prefix: None,
            },
        ],
        i18n_namespace: "recycle".into(),
    })
}
