use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel, SlotSpec};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "customs.search".into(),
        name: "搜索".into(),
        // 手稿 20260926：搜索为八大板块之一（L1 Custom → Board 归位）
        level: PluginLevel::Board,
        parent: None,
        slot: None,
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 批次6b S1：search 不直接拥有表（复用 kb_entries / editor_documents / messages / todos / journals），
            // 仅声明 cross_module_key_prefixed_tables 为只读访问。
            db: vec![],
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 15 条 search IPC 别名 + 0 条 extension/adapter。
            ipc: vec!["se_*".into()],
            fs: vec![],
            net: vec![],
        },
        // 3 个 L2 子插件插槽（纯前端子插件，无命令无表）
        slots: vec![
            SlotSpec {
                id: "search.browser".into(),
                r#type: "panel".into(),
                description: "浏览器搜索".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "search.global".into(),
                r#type: "panel".into(),
                description: "全站搜索".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "search.bookmarks".into(),
                r#type: "panel".into(),
                description: "收藏站点".into(),
                capacity: 1,
                route_prefix: None,
            },
        ],
        i18n_namespace: "search".into(),
    })
}
