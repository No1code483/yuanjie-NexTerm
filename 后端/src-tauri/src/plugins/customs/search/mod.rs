//! customs.search 插件适配层（批次6b）。
//!
//! search 插件不直接拥有 DB 表（复用 kb/editor/todos/journals 等跨模块查询），
//! 故无 migration 登记。15 条 search 命令 dispatcher。
//!
//! handler 闭包仅捕获 AppHandle，派发时运行时解析 AppState（契约 06 §8.1），
//! 业务实现复用 commands/search_commands 原函数。

mod commands;
mod manifest;
mod service;

pub mod features;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, Plugin, PluginHandle};
use kernel::tauri_glue::{register_command, KernelState};
use kernel_api::{KernelError, Manifest};

pub struct SearchPlugin;

#[async_trait::async_trait]
impl Plugin for SearchPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    async fn init(&self, ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        // search 不拥有 DB 表，无迁移登记。
        Ok(PluginHandle::default())
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("se")
        .setup(|app, _api| {
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(SearchPlugin));
            drop(registry);

            // AppState 在主应用 setup 中晚于插件 setup manage；这里只捕获 AppHandle，
            // 由 handler 在真正派发时解析 AppState，确保认证/pool/服务均为主应用实例。
            let handle = app.clone();
            for alias in commands::IPC_ALIASES {
                let handle = handle.clone();
                let legacy = alias.legacy_command;
                register_command(&state, alias.new_command, move |args| {
                    let handle = handle.clone();
                    async move { commands::dispatch_legacy(handle, legacy, args).await }
                });
            }
            Ok(())
        })
        .build()
}
