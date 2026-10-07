// plugins/_legacy — D5 整树快迁收编（2026-09-25）：旧 commands/ + services/ 目录
// 整体迁入插件命名空间，由各插件 dispatch_legacy 代理复用。
// 按插件逐个自包含拆分属后续长期迭代（见 阶段4 DoD D5 行）。
pub mod commands;
pub mod services;
