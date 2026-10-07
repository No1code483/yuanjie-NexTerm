//! knowledge.material 插件适配层（L2 Feature，批C3；parent boards.knowledge）。
//!
//! 必备子插件（手稿 20260926）：仅登记 manifest 供插件管理页展示与父链联动，
//! 无后端命令、无业务表、无基线迁移。资料库视图（library='material'）UI 由
//! L1 Knowledge 页与 Layout 侧边栏承载，本插件提供启停归属。

mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, Plugin, PluginHandle};
use kernel::tauri_glue::KernelState;
use kernel_api::{KernelError, Manifest};

pub struct KnowledgeMaterialPlugin;

#[async_trait::async_trait]
impl Plugin for KnowledgeMaterialPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    async fn init(&self, _ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        Ok(PluginHandle::default())
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("knowledge_material")
        .setup(|app, _api| {
            // 必须在 kernel 与 L1（boards.knowledge）之后装配
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(KnowledgeMaterialPlugin));
            drop(registry);

            Ok(())
        })
        .build()
}
