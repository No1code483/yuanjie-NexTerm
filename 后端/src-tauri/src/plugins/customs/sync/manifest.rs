use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel, SlotSpec};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "customs.sync".into(),
        name: "同步".into(),
        level: PluginLevel::Custom,
        parent: None,
        slot: None,
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 批次6b S1：sync_queue / sync_devices 表归属（db_归属判定表 §4）。
            db: vec![
                "sync_queue".into(),
                "sync_devices".into(),
            ],
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 21 条 sync IPC 别名 + 0 条 extension/adapter。
            ipc: vec!["sy_*".into()],
            fs: vec![],
            net: vec![],
        },
        // 2 个 L2 子插件插槽（纯前端子插件，无命令无表）
        slots: vec![
            SlotSpec {
                id: "sync.devices".into(),
                r#type: "panel".into(),
                description: "设备管理".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "sync.conflicts".into(),
                r#type: "panel".into(),
                description: "冲突解决".into(),
                capacity: 1,
                route_prefix: None,
            },
        ],
        i18n_namespace: "sync".into(),
    })
}
