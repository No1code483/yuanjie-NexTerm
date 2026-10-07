//! customs.intelligence 插件服务边界说明。
//!
//! 自有表：activity_logs, suggestions, behavior_patterns, intelligence_settings
//! （由 migrations v61-v64 创建，此处仅补登记 ownership）。

pub const OWNED_TABLES: &[&str] = &[
    "activity_logs",
    "suggestions",
    "behavior_patterns",
    "intelligence_settings",
];
