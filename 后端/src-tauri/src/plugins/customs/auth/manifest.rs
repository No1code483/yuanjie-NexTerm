use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel, SlotSpec};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "customs.auth".into(),
        name: "认证体系".into(),
        level: PluginLevel::Custom,
        parent: None,
        slot: None,
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 批次1a-1：users / permissions / auth_sessions；
            // 批次1a-2a crypto 收编后追加 mek_versions / mek_rotation_log（由 0002 基线迁移登记）。
            // system_config 的 2fa_* 键位为既有 2FA 实现的跨模块受限写，见 service.rs。
            db: vec![
                "users".into(),
                "permissions".into(),
                "auth_sessions".into(),
                "mek_versions".into(),
                "mek_rotation_log".into(),
            ],
            // 本批不发布/订阅领域事件（04_事件总线 §六 未列 auth 域事件）。
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            ipc: vec!["au_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![
            SlotSpec {
                id: "auth.login".into(),
                r#type: "panel".into(),
                description: "登录".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "auth.register".into(),
                r#type: "panel".into(),
                description: "注册".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "auth.recovery".into(),
                r#type: "panel".into(),
                description: "找回密码".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "auth.temp".into(),
                r#type: "panel".into(),
                description: "临时账号".into(),
                capacity: 1,
                route_prefix: None,
            },
        ],
        i18n_namespace: "auth".into(),
    })
}
