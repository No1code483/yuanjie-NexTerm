//! boards.knowledge 插件适配层（L1 Board，阶段3 批次2a-1）。
//!
//! 按裁定 33-A：知识库为**单一 L1**（不设 L2 子插件），66 条命令按 2a-1 / 2a-2 / 2a-3
//! 三段迁移，本段落地 29 条核心数据面命令 + 10 张表的归属登记。
//! 启用/停用/挂载校验推迟到首个 kernel_dispatch → ensure_activated → activate_all
//! （与 _hello / boards.home 同一懒激活时序）。

mod commands;
mod manifest;
mod service;
// 批C3：4 个 L2 子插件（material/learning 必备；templates/graph 可选）
pub mod features;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, MigrationFile, Plugin, PluginHandle};
use kernel::tauri_glue::{register_command, KernelState};
use kernel_api::{KernelError, Manifest};

pub struct KnowledgePlugin;

#[async_trait::async_trait]
impl Plugin for KnowledgePlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    async fn init(&self, ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        ctx.register.migrations = Some((
            "boards.knowledge".to_string(),
            vec![MigrationFile {
                version: 1,
                sql: include_str!("migrations/0001_baseline.sql"),
            }],
        ));
        Ok(PluginHandle {
            migrations_ns: Some("boards.knowledge".to_string()),
            ..Default::default()
        })
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("knowledge")
        .setup(|app, _api| {
            // 必须在 kernel 之后装配（main.rs 顺序保证 KernelState 已 manage）
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(KnowledgePlugin));
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
