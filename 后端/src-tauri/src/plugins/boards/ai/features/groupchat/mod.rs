//! ai.groupchat 插件适配层（L2 Feature，阶段3 批次2b-1；parent boards.ai）。
//!
//! registry、manifest 登记 + 2 条 dispatcher 业务 handler（S2：dispatcher 状态注入）。
//! 无基线迁移（不持有业务表，与 L1 同口径）。handler 闭包仅捕获 AppHandle，
//! 派发时运行时解析主应用 AppState（契约 06 §8.1）。

mod commands;
mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, Plugin, PluginHandle};
use kernel::tauri_glue::{register_command, KernelState};
use kernel_api::{KernelError, Manifest};

pub struct AiGroupChatPlugin;

#[async_trait::async_trait]
impl Plugin for AiGroupChatPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    /// 无迁移、无订阅、无服务；命令 handler 由 init() 登记。
    async fn init(&self, _ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        Ok(PluginHandle::default())
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("ai_groupchat")
        .setup(|app, _api| {
            // 必须在 kernel 与 L1（boards.ai）之后装配（main.rs 顺序保证：
            // KernelState 已 manage，且 L1 已登记 → validate_mount 可解析 parent）
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(AiGroupChatPlugin));
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