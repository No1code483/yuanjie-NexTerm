//! boards.terminal（L1）服务边界说明。
//!
//! 批次3a：L1 直接承载终端板块全部 29 条命令 + 6 张表（5 业务 + FTS5 虚表）。
//! 实现层（terminal_service / terminal_mux / ssh_service / terminal_repo / ssh_repo）
//! 仍留主应用，dispatcher 复用（2b-2 裁定 15 同口径，实现层迁移登记阶段4）。
//! `terminal.linux` 插槽为 3b 预留（L2 manifest 与 linux 命令面属 3b）。

pub const OWNED_TABLES: &[&str] = &[
    "terminal_sessions",
    "terminal_history",
    "terminal_history_fts",
    "terminal_tab_layout",
    "terminal_config",
    "ssh_profiles",
];
