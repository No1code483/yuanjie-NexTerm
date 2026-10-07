//! customs.auth 插件适配层。
//!
//! registry、manifest、migration 登记 + 23 条 dispatcher 业务 handler（S2：
//! dispatcher 状态注入）。handler 闭包仅捕获 AppHandle，派发时运行时解析
//! 主应用 AppState（契约 06 §8.1），认证/pool/服务均来自真实状态。
//!
//! 2026-09-13（阶段3 批次1a-2a）：`crypto/`（用户密钥）自内核 `src/crypto/`、
//! `src/services/crypto_service.rs`、`src/commands/mek_rotation_commands.rs`、
//! `src/services/mek_rotation_scheduler.rs` 收编为本插件私有实现；
//! 内核仅保留门面与加密原语，接口契约见 `kernel_api::MekProvider`。

mod commands;
mod crypto;
pub mod features;
mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, MigrationFile, Plugin, PluginHandle};
use kernel::tauri_glue::{register_command, KernelState};
use kernel_api::{KernelError, Manifest, MekProvider};

pub use crypto::crypto_service::CryptoService;
pub use crypto::mek_rotation_commands;
pub use crypto::mek_rotation_scheduler::start_mek_rotation_scheduler;

pub struct AuthPlugin;

#[async_trait::async_trait]
impl Plugin for AuthPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    async fn init(&self, ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        ctx.register.migrations = Some((
            "customs.auth".to_string(),
            vec![
                MigrationFile {
                    version: 1,
                    sql: include_str!("migrations/0001_baseline.sql"),
                },
                // 批次1a-2a：mek_versions / mek_rotation_log 归属登记。
                MigrationFile {
                    version: 2,
                    sql: include_str!("migrations/0002_mek_baseline.sql"),
                },
            ],
        ));
        Ok(PluginHandle {
            migrations_ns: Some("customs.auth".to_string()),
            ..Default::default()
        })
    }
}

/// 加密基础设施实现工厂：装配期由 `AppState::new` 调用，注入内核门面。
///
/// 唯一实例：门面（`crate::crypto::mek_manager::MekManager`）持有它，
/// 会话密钥缓存与轮换结果因此始终落在同一实例上。
pub fn mek_provider() -> Box<dyn MekProvider> {
    Box::new(crypto::mek_manager::AuthMekProvider::new())
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("auth")
        .setup(|app, _api| {
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;
            let _cross_module_tables = service::CROSS_MODULE_KEY_PREFIXED_TABLES;

            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(AuthPlugin));
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
