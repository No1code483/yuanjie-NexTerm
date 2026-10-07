//! recycle.list 插件适配层（L2 Feature；parent customs.recycle）。
//!
//! 回收站「一切皆插件」拆分：纯前端视图子插件，仅登记 manifest 供插件管理页展示与启停联动；
//! 无后端命令、无业务表、无基线迁移。视图 UI 位于前端 plugins/customs/recycle/features/list/。

mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, Plugin, PluginHandle};
use kernel::tauri_glue::KernelState;
use kernel_api::{KernelError, Manifest};

pub struct RecycleListPlugin;

#[async_trait::async_trait]
impl Plugin for RecycleListPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    async fn init(&self, _ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        Ok(PluginHandle::default())
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("recycle_list")
        .setup(|app, _api| {
            // 必须在 kernel 与父插件（customs.recycle）之后装配
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(RecycleListPlugin));
            drop(registry);

            Ok(())
        })
        .build()
}
