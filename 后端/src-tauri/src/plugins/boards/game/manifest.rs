//! boards.game 插件清单（阶段3 批次4b；L1 Board，游戏 3D 重构）。
//!
//! 覆盖面：game 35 + game_story 4 + game_intelligence 2 + game_opponent 1 + game_nl 1
//! + game_behavior 1 = **44 条** IPC 命令 + 17 张 `game_*` 表归属登记。
//! L2 插槽 `game.3d` / `game.2dpreview` 由后续批次挂载。
//!
//! 表名前缀口径（裁定 2）：短码 `gm` + `_` = `gm_`，17 张表均以 `game_` 前缀
//! → **全部 name_prefixed=0**（由 migrations/0001_baseline.sql 登记归属，只登记不重放 DDL）。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel, SlotSpec};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "boards.game".into(),
        name: "游戏".into(),
        level: PluginLevel::Board,
        parent: None,
        slot: None,
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 17 张自有表（游戏 3D 重构 - 世界/建筑/知识/突破/历史/积分/剧情/NPC/关系/传言/技能）
            // 短码 `gm` 与 `game_` 前缀不命中 → name_prefixed=0。
            db: vec![
                "game_worlds".into(),
                "game_knowledge_domains".into(),
                "game_buildings".into(),
                "game_knowledge_progress".into(),
                "game_breakthrough_records".into(),
                "game_build_history".into(),
                "game_kb_category_mapping".into(),
                "game_points_log".into(),
                "game_daily_limit_counter".into(),
                "game_points_source_config".into(),
                "game_npcs".into(),
                "game_npc_conversations".into(),
                "game_stories".into(),
                "game_story_nodes".into(),
                "game_npc_memories".into(),
                "game_npc_relationships".into(),
                "game_npc_rumors".into(),
                "game_player_skill".into(),
            ],
            // 本批不发布/订阅内核领域事件（S5 按实测登记）。
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 44 条 IPC；ACL/handler 键由内核按短码 `gm` 构造（<短码>_<命令名>）。
            ipc: vec!["gm_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![
            // 4b 后续批次挂载：game.3d / game.2dpreview
            SlotSpec {
                id: "game.3d".into(),
                r#type: "panel".into(),
                description: "游戏 3D 场景".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "game.2dpreview".into(),
                r#type: "panel".into(),
                description: "游戏 2D 预览".into(),
                capacity: 1,
                route_prefix: None,
            },
            // 3 个纯前端 L2 子插件插槽（preview/play3d/mapping）
            SlotSpec {
                id: "game.preview".into(),
                r#type: "panel".into(),
                description: "游戏预览".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "game.play3d".into(),
                r#type: "panel".into(),
                description: "3D 游戏".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "game.mapping".into(),
                r#type: "panel".into(),
                description: "知识映射".into(),
                capacity: 1,
                route_prefix: None,
            },
        ],
        i18n_namespace: "game".into(),
    })
}
