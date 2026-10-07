//! terminal.yuancode.skills 插件适配层（L3 Feature；parent terminal.yuancode）。
//!
//! 可选子插件：仅登记 manifest 供插件管理页展示与启停联动；无后端命令、
//! 无业务表、无基线迁移。技能与片段 UI 由 L2 terminal.yuancode 承载。

mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, Plugin, PluginHandle};
use kernel::tauri_glue::KernelState;
use kernel_api::{KernelError, Manifest};

pub struct YuanCodeSkillsPlugin;

#[async_trait::async_trait]
impl Plugin for YuanCodeSkillsPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    async fn init(&self, _ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        Ok(PluginHandle::default())
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("yuancode_skills")
        .setup(|app, _api| {
            // 必须在 kernel 与 L2（terminal.yuancode）之后装配
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(YuanCodeSkillsPlugin));
            drop(registry);

            Ok(())
        })
        .build()
}
