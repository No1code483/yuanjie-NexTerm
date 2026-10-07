//! _hello IPC 命令（契约：06_Rust代码契约.md §八；阶段1 B8 计划）
//!
//! check_ipc 白名单与停用即拒已由 kernel_dispatch 入口统一执行（B7 落地决策），
//! 命令体保持纯净。

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

/// 演示命令：返回 "pong:_hello"
pub async fn hello_ping(_args: Json) -> Result<Json, String> {
    Ok(Json::String("pong:_hello".to_string()))
}

/// 发布领域事件，验证 Rust 总线、持久化和前端桥接全链路。
pub async fn hello_publish<R: Runtime>(handle: AppHandle<R>, _args: Json) -> Result<Json, String> {
    let kernel = handle.state::<kernel::tauri_glue::KernelState>();
    let origin = "_hello".to_string();
    let name = "_hello:ping";
    kernel
        .security
        .check_publish(
            super::manifest::manifest(),
            name,
            kernel_api::EventScope::Domain,
        )
        .map_err(|error| error.to_string())?;
    kernel
        .bus
        .publish(
            &origin,
            kernel_api::Event {
                name: name.to_string(),
                origin: origin.clone(),
                domain: origin.clone(),
                scope: kernel_api::EventScope::Domain,
                payload: serde_json::json!({ "message": "ping from _hello" }),
                at: chrono::Utc::now().timestamp_millis(),
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({ "event": name }))
}
