//! boards.knowledge 插件清单（阶段3 批次2a-1；L1 Board）。
//!
//! 批C3 起按手稿 20260926 拆出 4 个 L2 子插件（material/learning【必备】+
//! templates/graph【可选】）；kb_templates 表与 4 条模版命令随 templates L2 迁出
//! （归属迁移见 features/templates/migrations/0001_baseline.sql）。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel, SlotSpec};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "boards.knowledge".into(),
        name: "知识库".into(),
        level: PluginLevel::Board,
        parent: None,
        slot: None,
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 9 张 kb_* 表由本插件归属登记（migrations/0001_baseline.sql）；
            // kb_templates 已随批C3 迁出至 knowledge.templates。
            // ⚠️ kb_categories.id 被 game_kb_category_mapping.category_id 逻辑引用（无物理
            // FK，models/game.rs:888），迁移不得改变其值（档案 §6.6 ID 红线）。
            db: vec![
                "kb_attachment_cache".into(),
                "kb_categories".into(),
                "kb_entries".into(),
                "kb_entry_tags".into(),
                "kb_recent_access".into(),
                "kb_references".into(),
                "kb_snapshots".into(),
                "kb_tags".into(),
                "kb_tracked_paths".into(),
            ],
            // 2a-1 不发布/订阅领域事件；`knowledge:item.*` 契约按批次 1b-2c 先例另批接线。
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
            ipc: vec!["kb_*".into()],
            // 知识库含本地文件读取（kb_read_external_file / kb_scan_directory 等，属 2a-2），
            // 当前内核未对 fs 权限做实际校验，暂与 boards.home 同口径留空并登记。
            fs: vec![],
            net: vec![],
        },
        // 批C3：4 个 L2 子插件插槽（material/learning 必备；templates/graph 可选）
        slots: vec![
            SlotSpec {
                id: "knowledge.material".into(),
                r#type: "panel".into(),
                description: "资料库".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "knowledge.learning".into(),
                r#type: "panel".into(),
                description: "学习库".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "knowledge.templates".into(),
                r#type: "panel".into(),
                description: "插入模版".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "knowledge.graph".into(),
                r#type: "panel".into(),
                description: "关系图谱".into(),
                capacity: 1,
                route_prefix: None,
            },
            // 批C3：8 个纯前端子插件插槽（无命令无表）
            SlotSpec {
                id: "knowledge.browse".into(),
                r#type: "panel".into(),
                description: "条目浏览".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "knowledge.search".into(),
                r#type: "panel".into(),
                description: "搜索".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "knowledge.tags".into(),
                r#type: "panel".into(),
                description: "标签".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "knowledge.media".into(),
                r#type: "panel".into(),
                description: "媒体查看器".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "knowledge.editors".into(),
                r#type: "panel".into(),
                description: "编辑器".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "knowledge.import".into(),
                r#type: "panel".into(),
                description: "导入".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "knowledge.history".into(),
                r#type: "panel".into(),
                description: "快照与反链".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "knowledge.ai".into(),
                r#type: "panel".into(),
                description: "AI 辅助".into(),
                capacity: 1,
                route_prefix: None,
            },
        ],
        i18n_namespace: "knowledge".into(),
    })
}
