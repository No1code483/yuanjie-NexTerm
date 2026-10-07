//! terminal.linux 插件服务边界说明。
//!
//! 无自有表（linux/docker 命令为系统查询 / 内核源码目录访问 / docker CLI，不落库）、
//! 无基线迁移。实现层 `linux_service`（1,270 行）留主应用，dispatcher 复用
//!（2b-2 裁定 15 同口径，实现层迁移登记阶段4）。

pub const OWNED_TABLES: &[&str] = &[];
