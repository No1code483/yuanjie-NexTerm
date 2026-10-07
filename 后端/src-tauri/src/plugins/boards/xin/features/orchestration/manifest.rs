//! xin.orchestration 插件清单（阶段3 批次4a-2；L2 Feature，parent boards.xin）。
//!
//! 覆盖面：`xin_orchestration_commands` 全量 87 条编排命令（context 7 / dialogue 7 /
//! tools 5 / fusion 5 / post 6 / dream 8 / commit 9 / eval 4 / evolution 7 / proactive 6 /
//! checkpoint 5 / review 4 / compaction 6），零消费约 50 条按裁定 1「留 + alias」。
//! 编排面涉及 4 张表（`xin_conversations` / `xin_checkpoints` / `xin_compaction_config` /
//! `xin_compaction_records`，均 `name_prefixed=0`）；`xin_memories` 跨段写
//! (`xin_dialogue_service.rs:751`) 归属登记见 L1。
//!
//! **表名前缀口径（裁定 2）**：L2 短码 `xo` → `xo_`，四张编排表均为 `xin_` 前缀 → 全部
//! `name_prefixed=0`。基线迁移只登记归属（只登记不重放 DDL）。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "xin.orchestration".into(),
        name: "小欣编排".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.xin".into()),
        slot: Some("xin.orchestration".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 4 张编排面表（仅归属登记，已由主应用历史迁移创建）；
            // 短码 `xo` 与 `xin_` 前缀不命中 → name_prefixed=0（裁定 2）。
            db: vec![
                "xin_conversations".into(),
                "xin_checkpoints".into(),
                "xin_compaction_config".into(),
                "xin_compaction_records".into(),
            ],
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 87 条 IPC；ACL/handler 键由内核按短码 `xo` 构造（<短码>_<命令名>）。
            ipc: vec!["xo_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "xin.orchestration".into(),
    })
}
