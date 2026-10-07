//! boards.ai 插件适配层（L1 Board，阶段3 批次2b-1 骨架 / 2b-2 会话面收编）。
//!
//! 2b-1：L1 骨架仅做 registry 登记 + 插槽声明（承载 ai.models / ai.sessions /
//! ai.agent / ai.groupchat 四个 L2 Feature）。
//! 2b-2（裁定 11/12）：L1 首次「既持 L2 插槽又持自有 IPC」——承载会话面
//! 18 条命令（会话 10 + 消息 3 + 参与者 1 + 模板 4）。
//! BUG-033 修订（2026-09-27）：4 张会话表归属已随 ai.sessions migrations/0002
//! 转移至 ai.sessions，L1 manifest 不再声明 db（否则二次冷激活重复登记冲突）。
//! 启用/停用/挂载校验推迟到首个 kernel_dispatch → ensure_activated → activate_all
//! （与 _hello / boards.home / boards.knowledge 同一懒激活时序）。

pub mod features;

mod commands;
mod manifest;
mod service;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, MigrationFile, Plugin, PluginHandle};
use kernel::tauri_glue::{register_command, KernelState};
use kernel_api::{KernelError, Manifest};

pub struct AiPlugin;

#[async_trait::async_trait]
impl Plugin for AiPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    /// 2b-2：v1 基线迁移保留（全新库先登记归属 boards.ai，供 ai.sessions v2 UPDATE
    /// 转移；已记账库不重放）；manifest 已不声明表 → 激活时无重复登记。
    async fn init(&self, ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        ctx.register.migrations = Some((
            "boards.ai".to_string(),
            vec![MigrationFile {
                version: 1,
                sql: include_str!("migrations/0001_baseline.sql"),
            }],
        ));
        Ok(PluginHandle {
            migrations_ns: Some("boards.ai".to_string()),
            ..Default::default()
        })
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("ai")
        .setup(|app, _api| {
            // 必须在 kernel 之后装配（main.rs 顺序保证 KernelState 已 manage）
            let state = app.state::<KernelState>();
            let _owned_tables = service::OWNED_TABLES;

            // registry 登记（setup 为同步闭包 → blocking_lock）
            let mut registry = state.registry.blocking_lock();
            registry.register(Box::new(AiPlugin));
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
