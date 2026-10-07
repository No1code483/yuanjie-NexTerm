//! knowledge.templates 插件适配层（L2 Feature，批C3 深度拆分；parent boards.knowledge）。
//!
//! registry、manifest 登记 + 归属迁移（kb_templates 自 L1 迁入）+ 4 条模版命令
//! dispatcher（实现层复用 `kb_commands` 原函数）。可选子插件：可独立停用/删除。

mod commands;
mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, MigrationFile, Plugin, PluginHandle};
use kernel::tauri_glue::{register_command, KernelState};
use kernel_api::{KernelError, Manifest};

pub struct KnowledgeTemplatesPlugin;

#[async_trait::async_trait]
impl Plugin for KnowledgeTemplatesPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    /// 批C3：kb_templates 归属迁移（boards.knowledge → knowledge.templates，含
    /// name_prefixed 1→0 改写），须先于 register_manifest_tables 应用。
    async fn init(&self, ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        ctx.register.migrations = Some((
            "knowledge.templates".to_string(),
            vec![MigrationFile {
                version: 1,
                sql: include_str!("migrations/0001_baseline.sql"),
            }],
        ));
        Ok(PluginHandle {
            migrations_ns: Some("knowledge.templates".to_string()),
            ..Default::default()
        })
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("knowledge_templates")
        .setup(|app, _api| {
            // 必须在 kernel 与 L1（boards.knowledge）之后装配（main.rs 顺序保证：
            // L1 已登记 → L1 基线的 kb_templates 归属行已存在，归属 UPDATE 才有目标）
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(KnowledgeTemplatesPlugin));
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
