//! boards.terminal 插件清单（阶段3 批次3a；L1 Board，终端板块）。
//!
//! 覆盖面：terminal_commands 全量 29 条（PTY 会话 / 历史 / 布局 / WSL / Mux / SSH / 配置主题，
//! 裁定 T7 零消费 19 条判「留 + alias」）+ 6 张表（5 业务 + 1 FTS5 虚表 terminal_history_fts）。
//! 插槽预留 `terminal.linux`（3b 挂载 L2，1b-1 教训：L2 须独立短码 lx）。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel, SlotSpec};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "boards.terminal".into(),
        name: "终端".into(),
        level: PluginLevel::Board,
        parent: None,
        slot: None,
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 6 张自有表：terminal_sessions / terminal_history / terminal_tab_layout /
            // terminal_config / ssh_profiles + FTS5 虚表 terminal_history_fts（v40，
            // 随 terminal_history 归属本插件；短码 tm，terminal_ 前缀不命中 → name_prefixed=0）。
            db: vec![
                "terminal_sessions".into(),
                "terminal_history".into(),
                "terminal_history_fts".into(),
                "terminal_tab_layout".into(),
                "terminal_config".into(),
                "ssh_profiles".into(),
            ],
            // 本批不发布/订阅内核领域事件；`terminal-output`（PTY 输出）与 `mux:notification`
            // 为 Tauri emit，不走内核事件总线（S5 零改动登记）。
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 29 条 IPC；ACL/handler 键由内核按短码 `tm` 构造（<短码>_<命令名>）。
            ipc: vec!["tm_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![
            SlotSpec {
                id: "terminal.linux".into(),
                r#type: "panel".into(),
                description: "Linux 子系统".into(),
                capacity: 1,
                route_prefix: None,
            },
            // 批次3c：yuancode 插槽（C2 裁定本次新增，3c 起挂载 terminal.yuancode L2）
            SlotSpec {
                id: "terminal.yuancode".into(),
                r#type: "panel".into(),
                description: "Yuan Code 编程".into(),
                capacity: 1,
                route_prefix: None,
            },
            // 批C2：manual 插槽（命令手册 L2，5 路由纯前端）
            SlotSpec {
                id: "terminal.manual".into(),
                r#type: "ui-route".into(),
                description: "命令手册".into(),
                capacity: 1,
                route_prefix: None,
            },
            // 批C4：console 插槽（终端命令行 L2，纯前端）
            SlotSpec {
                id: "terminal.console".into(),
                r#type: "panel".into(),
                description: "终端命令行".into(),
                capacity: 1,
                route_prefix: None,
            },
            // 批C4：mux 插槽（标签页与分屏 L2，纯前端）
            SlotSpec {
                id: "terminal.mux".into(),
                r#type: "panel".into(),
                description: "标签页与分屏".into(),
                capacity: 1,
                route_prefix: None,
            },
            // 批C4：tools 插槽（命令辅助工具 L2，纯前端）
            SlotSpec {
                id: "terminal.tools".into(),
                r#type: "panel".into(),
                description: "命令辅助工具".into(),
                capacity: 1,
                route_prefix: None,
            },
        ],
        i18n_namespace: "terminal".into(),
    })
}
