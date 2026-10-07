//! ai.sessions 插件适配层（L2 Feature，batchC1；parent boards.ai）。
//!
//! registry、manifest、migration 登记 + 10 条 dispatcher 业务 handler（S2：
//! dispatcher 状态注入）。handler 闭包仅捕获 AppHandle，派发时运行时解析
//! 主应用 AppState（契约 06 §8.1），认证/pool/服务均来自真实状态。
//!
//! batchC1：会话列表功能从 L1 boards.ai 收编到 L2 ai.sessions。
//! 业务实现复用 chat_commands（裁定 11），返回值序列化与旧 IPC 路径一致。

mod commands;
mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, MigrationFile, Plugin, PluginHandle};
use kernel::tauri_glue::{register_command, KernelState};
use kernel_api::{KernelError, Manifest};

pub struct AiSessionsPlugin;

#[async_trait::async_trait]
impl Plugin for AiSessionsPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    /// batchC1：L2 登记 4 张会话表归属（基线迁移，只登记不重放 DDL）。
    /// BUG-033 修复（v2）：真机库中 4 表已由 L1 基线登记（owner=boards.ai），
    /// v1 的 INSERT OR IGNORE 静默跳过 → 归属校验失败；v1 已记账不可重放，
    /// 归属改写走 v2 UPDATE（0002_transfer_ownership.sql）。
    async fn init(&self, ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        ctx.register.migrations = Some((
            "ai.sessions".to_string(),
            vec![
                MigrationFile {
                    version: 1,
                    sql: include_str!("migrations/0001_baseline.sql"),
                },
                MigrationFile {
                    version: 2,
                    sql: include_str!("migrations/0002_transfer_ownership.sql"),
                },
            ],
        ));
        Ok(PluginHandle {
            migrations_ns: Some("ai.sessions".to_string()),
            ..Default::default()
        })
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("ai_sessions")
        .setup(|app, _api| {
            // 必须在 kernel 与 L1（boards.ai）之后装配（main.rs 顺序保证：
            // KernelState 已 manage，且 L1 已登记 → validate_mount 可解析 parent）
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            // registry 登记
            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(AiSessionsPlugin));
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
