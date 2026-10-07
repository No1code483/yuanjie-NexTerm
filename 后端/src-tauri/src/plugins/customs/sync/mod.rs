//! customs.sync 插件适配层（批次6b）。
//!
//! registry/manifest/migration 登记 + 21 条 sync 命令 dispatcher。
//!
//! handler 闭包仅捕获 AppHandle，派发时运行时解析 AppState（契约 06 §8.1），
//! 业务实现复用 commands/sync_commands 原函数。

mod commands;
pub mod features;
mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, MigrationFile, Plugin, PluginHandle};
use kernel::tauri_glue::{register_command, KernelState};
use kernel_api::{KernelError, Manifest};

pub struct SyncPlugin;

#[async_trait::async_trait]
impl Plugin for SyncPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    async fn init(&self, ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        ctx.register.migrations = Some((
            "customs.sync".to_string(),
            vec![MigrationFile {
                version: 1,
                sql: include_str!("migrations/0001_baseline.sql"),
            }],
        ));
        Ok(PluginHandle {
            migrations_ns: Some("customs.sync".to_string()),
            ..Default::default()
        })
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("sy")
        .setup(|app, _api| {
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(SyncPlugin));
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
