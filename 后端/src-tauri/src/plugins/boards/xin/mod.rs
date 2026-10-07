//! boards.xin 插件适配层（L1 Board，阶段3 批次4a-1）。
//!
//! L1 直接承载小欣基础面 28 条命令（裁定 1 零消费全量「留 + alias」）+ 6 张表归属登记；
//! `xin.wellness` / `xin.realtime` 两个插槽由本批 L2（features/wellness、features/realtime）
//! 挂载。启用/停用/挂载校验推迟到首个 kernel_dispatch → ensure_activated → activate_all
//! （与 boards.ai / boards.terminal 同一懒激活时序）。

pub mod features;

mod commands;
mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, MigrationFile, Plugin, PluginHandle};
use kernel::tauri_glue::{register_command, KernelState};
use kernel_api::{KernelError, Manifest};

pub struct XinPlugin;

#[async_trait::async_trait]
impl Plugin for XinPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    /// 4a-1：登记 6 张小欣基础域表归属（基线迁移，只登记不重放 DDL）。
    async fn init(&self, ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        ctx.register.migrations = Some((
            "boards.xin".to_string(),
            vec![MigrationFile {
                version: 1,
                sql: include_str!("migrations/0001_baseline.sql"),
            }],
        ));
        Ok(PluginHandle {
            migrations_ns: Some("boards.xin".to_string()),
            ..Default::default()
        })
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("xin")
        .setup(|app, _api| {
            // 必须在 kernel 之后装配（main.rs 顺序保证 KernelState 已 manage）
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            // registry 登记（setup 为同步闭包 → blocking_lock）
            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(XinPlugin));
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