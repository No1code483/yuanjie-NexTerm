//! boards.home 插件适配层（L1 Board，阶段3 批次1b-1）。
//!
//! L1 骨架：只做 registry 登记 + 插槽声明（承载 home.todo / home.journal /
//! home.timer / home.news 四个 L2 Feature 插件；home.timer 属批次 1b-2a、
//! home.news 属批次 1b-2b-1），
//! 无自有 IPC 命令、无基线迁移（不持有业务表）。
//! 启用/停用/挂载校验推迟到首个 kernel_dispatch → ensure_activated → activate_all
//! （与 _hello / boards.profile 同一懒激活时序）。

pub mod features;

mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, Plugin, PluginHandle};
use kernel::tauri_glue::KernelState;
use kernel_api::{KernelError, Manifest};

pub struct HomePlugin;

#[async_trait::async_trait]
impl Plugin for HomePlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    /// L1 骨架无迁移、无订阅、无服务；L2 由各 Feature 插件自行登记。
    async fn init(&self, _ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        Ok(PluginHandle::default())
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("home")
        .setup(|app, _api| {
            // 必须在 kernel 之后装配（main.rs 顺序保证 KernelState 已 manage）
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            // registry 登记（setup 为同步闭包 → blocking_lock）；L1 无 IPC 命令可登记
            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(HomePlugin));
            drop(registry);
            Ok(())
        })
        .build()
}
