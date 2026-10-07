//! customs.systemtools 插件适配层（批次4c）。
//!
//! registry/manifest/migration 登记 + 10 条 system 命令 dispatcher。
//! extension_*（2）/ adapter_*（3）裁定 T2/T3 零消费判删 —— 命令函数已删除，
//! 不在 IPC_ALIASES 中。
//!
//! handler 闭包仅捕获 AppHandle，派发时运行时解析 AppState（契约 06 §8.1），
//! 业务实现复用 commands/system_commands 原函数。

mod commands;
mod manifest;
mod service;


use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, MigrationFile, Plugin, PluginHandle};
use kernel::tauri_glue::{register_command, KernelState};
use kernel_api::{KernelError, Manifest};

pub struct SystemtoolsPlugin;

#[async_trait::async_trait]
impl Plugin for SystemtoolsPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    async fn init(&self, ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        ctx.register.migrations = Some((
            "customs.systemtools".to_string(),
            vec![MigrationFile {
                version: 1,
                sql: include_str!("migrations/0001_baseline.sql"),
            }],
        ));
        Ok(PluginHandle {
            migrations_ns: Some("customs.systemtools".to_string()),
            ..Default::default()
        })
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("st")
        .setup(|app, _api| {
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;
            let _cross_module_tables = service::CROSS_MODULE_KEY_PREFIXED_TABLES;

            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(SystemtoolsPlugin));
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
