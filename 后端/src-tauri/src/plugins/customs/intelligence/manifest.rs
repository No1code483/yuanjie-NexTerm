use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel, SlotSpec};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "customs.intelligence".into(),
        name: "底层智能".into(),
        level: PluginLevel::Custom,
        parent: None,
        slot: None,
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 批次6c S1：intelligence 自有表归属。
            // activity_logs / suggestions / behavior_patterns / intelligence_settings
            // 由 migrations v61-v64 创建，此处补登记 ownership。
            db: vec![
                "activity_logs".into(),
                "suggestions".into(),
                "behavior_patterns".into(),
                "intelligence_settings".into(),
            ],
            events: EventPerms {
                subscribe: vec![
                    "home:*".into(),
                    "kb:*".into(),
                    "ai:*".into(),
                    "game:*".into(),
                ],
                // BUG-034 修复：publish 域必须等于插件 id（security.rs::validate_manifest
                // 域校验，home.todo:* / boards.profile:* 同口径）；intelligence 域声明
                // 无实际发布点（S5 零领域事件），域对齐为 customs.intelligence 零风险。
                publish: vec![
                    "customs.intelligence:*".into(),
                ],
            },
            // 74 条 intelligence IPC 别名。
            ipc: vec!["sp_*".into()],
            fs: vec![],
            net: vec![],
        },
        // 5 个 L2 子插件插槽（纯前端子插件，无命令无表）
        slots: vec![
            SlotSpec {
                id: "intelligence.dashboard".into(),
                r#type: "panel".into(),
                description: "仪表盘".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "intelligence.suggestions".into(),
                r#type: "panel".into(),
                description: "智能建议".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "intelligence.behavior".into(),
                r#type: "panel".into(),
                description: "行为分析".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "intelligence.activity".into(),
                r#type: "panel".into(),
                description: "活动日志".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "intelligence.settings".into(),
                r#type: "panel".into(),
                description: "设置".into(),
                capacity: 1,
                route_prefix: None,
            },
        ],
        i18n_namespace: "intelligence".into(),
    })
}
