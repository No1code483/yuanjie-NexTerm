//! terminal.console 插件适配层（L2 Feature；parent boards.terminal）。
//!
//! registry、manifest 登记；无后端命令（纯前端视图）、无业务表、无基线迁移。
//! 拆分目的：终端命令行在插件管理页可独立启停（手稿 20260926「一切皆插件」）。

mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, Plugin, PluginHandle};
use kernel::tauri_glue::KernelState;
use kernel_api::{KernelError, Manifest};

pub struct TerminalConsolePlugin;

#[async_trait::async_trait]
impl Plugin for TerminalConsolePlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    /// 无迁移、无订阅、无命令；仅登记 manifest 供启停管理。
    async fn init(&self, _ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        Ok(PluginHandle::default())
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("terminal_console")
        .setup(|app, _api| {
            // 必须在 kernel 与 L1（boards.terminal）之后装配（main.rs 顺序保证：
            // KernelState 已 manage，且 L1 已登记 → validate_mount 可解析 parent）
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(TerminalConsolePlugin));
            drop(registry);

            Ok(())
        })
        .build()
}
