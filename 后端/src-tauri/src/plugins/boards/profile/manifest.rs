use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel, SlotSpec};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "boards.profile".into(),
        name: "个人中心".into(),
        // 手稿 20260926：个人中心为首页板块必备子插件（L1 Board → L2 Feature 归位）
        level: PluginLevel::Feature,
        parent: Some("boards.home".into()),
        slot: Some("home.profile".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 自有表可读写；跨模块导出只读范围在 service.rs 与插件档案中单独声明，
            // 不将其他插件表加入 db 写权限。
            db: vec!["user_profiles".into(), "resumes".into(), "quotes".into()],
            events: EventPerms {
                subscribe: vec![],
                publish: vec!["boards.profile:*".into()],
            },
            ipc: vec!["pf_*".into()],
            fs: vec![],
            net: vec![],
        },
        // 个人中心 4 个 L2 子插件插槽（account 必备；resume/quote/settings 可选）
        slots: vec![
            SlotSpec {
                id: "profile.account".into(),
                r#type: "panel".into(),
                description: "账号".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "profile.resume".into(),
                r#type: "panel".into(),
                description: "简历".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "profile.quote".into(),
                r#type: "panel".into(),
                description: "语录".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "profile.settings".into(),
                r#type: "panel".into(),
                description: "设置".into(),
                capacity: 1,
                route_prefix: None,
            },
        ],
        i18n_namespace: "profile".into(),
    })
}
