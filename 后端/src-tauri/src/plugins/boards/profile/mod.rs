//! boards.profile 插件适配层。
//!
//! registry、manifest、migration 登记 + 22 条 dispatcher 业务 handler（S2：
//! dispatcher 状态注入）。handler 闭包仅捕获 AppHandle，派发时运行时解析
//! 主应用 AppState（契约 06 §8.1），认证/pool/服务均来自真实状态。
//!
//! 2026-09-13（阶段3 批次1a-1）：4 条账号口令命令（profile_change_password /
//! profile_change_username / profile_update_profile / create_temp_account）
//! 按《插件化重构期规则》§三.5「跨插件命令归属唯一」收归 customs.auth，
//! 本插件 alias 由 26 条降为 22 条。

mod commands;
pub mod features;
mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, MigrationFile, Plugin, PluginHandle};
use kernel::tauri_glue::{register_command, KernelState};
use kernel_api::{KernelError, Manifest};

pub struct ProfilePlugin;

#[async_trait::async_trait]
impl Plugin for ProfilePlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    async fn init(&self, ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        ctx.register.migrations = Some((
            "boards.profile".to_string(),
            vec![MigrationFile {
                version: 1,
                sql: include_str!("migrations/0001_baseline.sql"),
            }],
        ));
        Ok(PluginHandle {
            migrations_ns: Some("boards.profile".to_string()),
            ..Default::default()
        })
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("profile")
        .setup(|app, _api| {
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;
            let _export_read_only_tables = service::EXPORT_READ_ONLY_TABLES;

            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(ProfilePlugin));
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
