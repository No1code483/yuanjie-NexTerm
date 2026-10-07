//! home.focus 插件清单（批C4；L2 Feature，parent boards.home）。
//!
//! 专注模式：纯前端组件（FocusMode，番茄钟 + 白噪音 + localStorage 历史持久化），
//! 无自有命令、无业务表、无领域事件。UI 由 L1 Home 页整页视图渲染（focusMode
//! 状态切换，非路由），入口按钮按本插件启停状态门控。
//! 可选子插件（手稿 20260926 首页可选子插件）。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "home.focus".into(),
        name: "专注".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.home".into()),
        slot: Some("home.focus".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 无业务表（专注历史存 localStorage）。
            db: vec![],
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 无 IPC。
            ipc: vec![],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "home.focus".into(),
    })
}
