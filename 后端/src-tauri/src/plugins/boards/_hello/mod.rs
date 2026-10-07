//! _hello 板块插件（阶段1 B8：插件化最小闭环 hello-world）
//!
//! 契约：06_Rust代码契约.md §八 模板；时序按 B7 落地决策（懒激活）：
//! Tauri setup 仅做 registry 登记 + IPC 命令登记，
//! 启用/停用/迁移推迟到首个 kernel_dispatch → ensure_activated → activate_all。

mod commands;
mod manifest;

use tauri::{Manager, Runtime};

use kernel::plugin::{KernelCtx, MigrationFile, Plugin, PluginHandle};
use kernel::tauri_glue::{register_command, KernelState};
use kernel_api::{KernelError, Manifest};

pub struct HelloPlugin;

#[async_trait::async_trait]
impl Plugin for HelloPlugin {
    fn manifest(&self) -> &'static Manifest {
        manifest::manifest()
    }

    /// 阶段1 最小实现：仅登记基线迁移（订阅/服务为空，由内核按 handle 逆向回滚）
    async fn init(&self, ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
        ctx.register.migrations = Some((
            "_hello".to_string(),
            vec![MigrationFile {
                version: 1,
                sql: include_str!("migrations/0001_baseline.sql"),
            }],
        ));
        Ok(PluginHandle {
            migrations_ns: Some("_hello".to_string()),
            ..Default::default()
        })
    }
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("_hello") // 插件短名 = manifest.id
        .setup(|app, _api| {
            // 必须在 kernel 之后装配（main.rs 顺序保证 KernelState 已 manage）
            let state = app.state::<KernelState>();

            // ① registry 登记（setup 为同步闭包 → blocking_lock；register 无返回值）
            let mut reg = state.registry.blocking_lock();
            reg.register(Box::new(HelloPlugin));
            drop(reg);

            // ② IPC 命令登记：契约逻辑名 → 白名单键 _hello_hello_ping
            //    （check_ipc + 停用即拒由 kernel_dispatch 入口统一兜底，闭包保持极简）
            register_command(&state, "_hello:plugin:hello_ping", |args| async move {
                commands::hello_ping(args).await
            });
            let handle = app.clone();
            register_command(&state, "_hello:plugin:hello_publish", move |args| {
                let handle = handle.clone();
                async move { commands::hello_publish(handle, args).await }
            });
            Ok(())
        })
        .build()
}
