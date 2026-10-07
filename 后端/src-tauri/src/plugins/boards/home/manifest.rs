//! boards.home 插件清单（阶段3 批次1b-1；L1 Board 首个「L1+L2 嵌套」样板）。
//!
//! L1 挂载内核导航主干，并以 6 个插槽承载子插件：home.profile / home.recycle
//! （手稿 20260926 归位的首页必备子插件）+ home.todo / home.journal /
//! home.timer / home.news 四个 L2 Feature 插件（本批先落地前两个）。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel, SlotSpec};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "boards.home".into(),
        name: "首页".into(),
        level: PluginLevel::Board,
        parent: None,
        slot: None,
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // L1 骨架不直接持有业务表：todos / journals 归属由对应 L2 自行登记。
            db: vec![],
            // 本批不发布/订阅领域事件（home 域事件契约属批次 1b-2c）。
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            ipc: vec![],
            fs: vec![],
            net: vec![],
        },
        slots: vec![
            SlotSpec {
                id: "home.profile".into(),
                r#type: "panel".into(),
                description: "个人中心（必备）".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "home.recycle".into(),
                r#type: "panel".into(),
                description: "回收站（必备）".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "home.todo".into(),
                r#type: "panel".into(),
                description: "待办面板".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "home.journal".into(),
                r#type: "panel".into(),
                description: "日志面板".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "home.news".into(),
                r#type: "panel".into(),
                description: "新闻面板".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "home.timer".into(),
                r#type: "panel".into(),
                description: "计时器面板".into(),
                capacity: 1,
                route_prefix: None,
            },
            // 批C4：专注插槽（home.focus L2，可选子插件，纯前端）
            SlotSpec {
                id: "home.focus".into(),
                r#type: "panel".into(),
                description: "专注".into(),
                capacity: 1,
                route_prefix: None,
            },
        ],
        i18n_namespace: "home".into(),
    })
}
