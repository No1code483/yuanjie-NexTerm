//! 内核插件 ACL 构建脚本（Tauri 2 插件权限机制）
//!
//! 为 kernel_dispatch 生成 allow-/deny- 权限（kebab-case：kernel-dispatch），
//! 聚合入口为 permissions/default.toml → 前端经 capabilities 授权 "kernel:default"。
//! 依据：06_Rust代码契约 §十一 2026-09-07 B8 前置修订（传输名纠偏 + ACL 三件套）。

const COMMANDS: &[&str] = &["kernel_dispatch"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
