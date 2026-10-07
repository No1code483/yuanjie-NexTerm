//! knowledge.history 插件适配层（L2 Feature，批C3；parent boards.knowledge）。
//!
//! 可选子插件：仅登记 manifest 供插件管理页展示与启停联动；无后端命令、
//! 无业务表、无基线迁移。快照与反链视图（HistoryView）UI 由 L1 Knowledge 页承载。

mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, Plugin, PluginHandle};
use kernel::tauri_glue::KernelState;
use kernel_api::{KernelError, Manifest};

pub struct KnowledgeHistoryPlugin;

#[async_trait::async_trait]
impl Plugin for KnowledgeHistoryPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    async fn init(&self, _ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        Ok(PluginHandle::default())
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("knowledge_history")
        .setup(|app, _api| {
            // 必须在 kernel 与 L1（boards.knowledge）之后装配
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(KnowledgeHistoryPlugin));
            drop(registry);

            Ok(())
        })
        .build()
}
