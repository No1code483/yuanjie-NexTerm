//! customs.sync 插件服务边界说明。
//!
//! 自有表：sync_queue、sync_devices（批次6b S1 登记）。

pub const OWNED_TABLES: &[&str] = &["sync_queue", "sync_devices"];
