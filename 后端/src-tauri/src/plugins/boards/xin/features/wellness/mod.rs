//! xin.wellness 插件适配层（L2 Feature，阶段3 批次4a-1；parent boards.xin）。
//!
//! registry、manifest 登记 + 17 条 dispatcher 业务 handler（实现层复用
//! `xin_wellness_commands` 原函数，含裁定 3 按 `user_id` 分片收口）。
//! 2 张预留表基线迁移（只登记不重放 DDL）；handler 闭包仅捕获 AppHandle，
//! 派发时运行时解析主应用 AppState（契约 06 §8.1）。

mod commands;
mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, MigrationFile, Plugin, PluginHandle};
use kernel::tauri_glue::{register_command, KernelState};
use kernel_api::{KernelError, Manifest};

pub struct XinWellnessPlugin;

#[async_trait::async_trait]
impl Plugin for XinWellnessPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    /// 4a-1：登记 2 张预留表归属（基线迁移，只登记不重放 DDL）。
    async fn init(&self, ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        ctx.register.migrations = Some((
            "xin.wellness".to_string(),
            vec![MigrationFile {
                version: 1,
                sql: include_str!("migrations/0001_baseline.sql"),
            }],
        ));
        Ok(PluginHandle {
            migrations_ns: Some("xin.wellness".to_string()),
            ..Default::default()
        })
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("xin_wellness")
        .setup(|app, _api| {
            // 必须在 kernel 与 L1（boards.xin）之后装配（main.rs 顺序保证：
            // KernelState 已 manage，且 L1 已登记 → validate_mount 可解析 parent）
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(XinWellnessPlugin));
            drop(registry);

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