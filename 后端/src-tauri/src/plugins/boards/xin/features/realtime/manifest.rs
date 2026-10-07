//! xin.realtime 插件清单（阶段3 批次4a-1；L2 Feature，parent boards.xin）。
//!
//! 覆盖面：`xin_realtime_commands` 全量 4 条（实时语音对话状态机：start / stop /
//! push_chunk / get_state）。全部已消费（`hooks/useRealtimeSession.ts`）。
//! 无业务表（`XinRealtimeService` 为内存态全局单例，不落库）、无基线迁移。
//! **登记（裁定 3）**：4 条命令的全局单例会话无主体分片（多用户互斥），架构级改造
//! （单会话 → 多会话管理）登记阶段4，本批仅登记不实施。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "xin.realtime".into(),
        name: "小欣实时语音".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.xin".into()),
        slot: Some("xin.realtime".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 不持有业务表（内存态会话，不落库）。
            db: vec![],
            // 不发布/订阅内核领域事件（事件经 Tauri Channel 流式推送，不走内核事件总线）。
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 4 条 IPC；ACL/handler 键由内核按短码 `xr` 构造（<短码>_<命令名>）。
            ipc: vec!["xr_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "xin.realtime".into(),
    })
}