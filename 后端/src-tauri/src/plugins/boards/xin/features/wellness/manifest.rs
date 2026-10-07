//! xin.wellness 插件清单（阶段3 批次4a-1；L2 Feature，parent boards.xin）。
//!
//! 覆盖面：`xin_wellness_commands` 全量 17 条（情绪历史 / 记忆整合 / 会话桥 /
//! 提醒 / 习惯 / 番茄钟 / 活动摘要 / 人格演化）。**前端零消费**（后端子系统完整、
//! UI 未接线 → 裁定 1「留 + alias」并登记功能缺口）。
//! 命令实现为**内存单例**（`MOOD_TRACKER` 等，不落库）；2 张 `xin_reminders` /
//! `xin_habits` 表为「零写入预留」（`xiaoxin_repo` 5 函数无调用方）→ 一并登记归属。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "xin.wellness".into(),
        name: "小欣健康助手".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.xin".into()),
        slot: Some("xin.wellness".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 2 张预留表（零写入）；L2 短码 `xw` 与 `xin_` 前缀不命中 → name_prefixed=0。
            db: vec!["xin_reminders".into(), "xin_habits".into()],
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 17 条 IPC；ACL/handler 键由内核按短码 `xw` 构造（<短码>_<命令名>）。
            ipc: vec!["xw_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "xin.wellness".into(),
    })
}