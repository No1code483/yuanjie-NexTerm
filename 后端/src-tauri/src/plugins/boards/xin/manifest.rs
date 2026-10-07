//! boards.xin 插件清单（阶段3 批次4a-1；L1 Board，小欣板块基础面）。
//!
//! 覆盖面：`xin_basic_commands` 全量 28 条（人格 / 记忆 / 情绪 / TTS/STT / 多模态 /
//! 每日简报；零消费 17 条按裁定 1「留 + alias」）+ 6 张表。声明 2 个插槽
//! （`xin.wellness` / `xin.realtime`，均由 4a-1 同批挂载 L2）。
//!
//! **表名前缀口径（裁定 2）**：内核按「短码 + `_`」机械推导（`registry.rs:94`），
//! L1 短码 `xn` → 前缀 `xn_`，而 6 张表均为 `xin_` 前缀 → **全部 name_prefixed=0**，
//! 由 `migrations/0001_baseline.sql` 登记归属（只登记不重放 DDL）。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel, SlotSpec};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "boards.xin".into(),
        name: "小欣".into(),
        level: PluginLevel::Board,
        parent: None,
        slot: None,
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 6 张自有表（`xin_config` / `xin_memories` / `xin_summaries` / `xin_moods` /
            // `xin_persona_memories` / `xin_persona_switch_log`）；短码 `xn` 与 `xin_` 前缀
            // 不命中 → name_prefixed=0（裁定 2）。
            db: vec![
                "xin_config".into(),
                "xin_memories".into(),
                "xin_summaries".into(),
                "xin_moods".into(),
                "xin_persona_memories".into(),
                "xin_persona_switch_log".into(),
            ],
            // 本批不发布/订阅内核领域事件（S5 按实测登记）。
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 28 条 IPC；ACL/handler 键由内核按短码 `xn` 构造（<短码>_<命令名>）。
            ipc: vec!["xn_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![
            SlotSpec {
                id: "xin.wellness".into(),
                r#type: "panel".into(),
                description: "小欣健康助手".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "xin.realtime".into(),
                r#type: "panel".into(),
                description: "小欣实时语音对话".into(),
                capacity: 1,
                route_prefix: None,
            },
            // 4a-2：编排面插槽（由 xin.orchestration L2 挂载）
            SlotSpec {
                id: "xin.orchestration".into(),
                r#type: "panel".into(),
                description: "小欣编排面".into(),
                capacity: 1,
                route_prefix: None,
            },
            // 小欣「一切皆插件」：12 个纯前端面板子插件插槽（无命令无表）
            SlotSpec {
                id: "xin.chat".into(),
                r#type: "panel".into(),
                description: "对话".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "xin.memory".into(),
                r#type: "panel".into(),
                description: "记忆".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "xin.mood".into(),
                r#type: "panel".into(),
                description: "心情".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "xin.briefing".into(),
                r#type: "panel".into(),
                description: "简报".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "xin.compaction".into(),
                r#type: "panel".into(),
                description: "上下文压缩".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "xin.dream".into(),
                r#type: "panel".into(),
                description: "梦境".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "xin.checkpoint".into(),
                r#type: "panel".into(),
                description: "检查点".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "xin.search".into(),
                r#type: "panel".into(),
                description: "对话搜索".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "xin.review".into(),
                r#type: "panel".into(),
                description: "复盘".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "xin.skill".into(),
                r#type: "panel".into(),
                description: "技能".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "xin.tool".into(),
                r#type: "panel".into(),
                description: "工具".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "xin.evolution".into(),
                r#type: "panel".into(),
                description: "人格进化".into(),
                capacity: 1,
                route_prefix: None,
            },
        ],
        i18n_namespace: "xin".into(),
    })
}