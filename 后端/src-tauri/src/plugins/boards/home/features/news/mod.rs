//! home.news 插件适配层（L2 Feature，阶段3 批次1b-2b-1；parent boards.home）。
//!
//! registry、manifest、migration 登记 + 14 条 dispatcher 业务 handler（S2：
//! dispatcher 状态注入）。handler 闭包仅捕获 AppHandle，派发时运行时解析
//! 主应用 AppState（契约 06 §8.1），认证/pool/服务均来自真实状态。

mod commands;
mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, MigrationFile, Plugin, PluginHandle};
use kernel::tauri_glue::{register_command, KernelState};
use kernel_api::{KernelError, Manifest};

pub struct NewsPlugin;

#[async_trait::async_trait]
impl Plugin for NewsPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    async fn init(&self, ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        ctx.register.migrations = Some((
            "home.news".to_string(),
            vec![
                MigrationFile {
                    version: 1,
                    sql: include_str!("migrations/0001_baseline.sql"),
                },
                MigrationFile {
                    version: 2,
                    sql: include_str!("migrations/0002_hardening_user_id.sql"),
                },
            ],
        ));
        Ok(PluginHandle {
            migrations_ns: Some("home.news".to_string()),
            ..Default::default()
        })
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("home_news")
        .setup(|app, _api| {
            // 必须在 kernel 与 L1（boards.home）之后装配（main.rs 顺序保证：
            // KernelState 已 manage，且 L1 已登记 → validate_mount 可解析 parent）
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(NewsPlugin));
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
