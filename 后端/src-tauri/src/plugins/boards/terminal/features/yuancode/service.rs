//! terminal.yuancode 插件服务边界说明。
//!
//! 自有表 14 张：3c 2 张（yuan_code_snippets／yuan_code_workspaces）+ 5a 4 张
//! （editor_documents／editor_versions／editor_sessions／distill_dataset）+
//! 5e 8 张（api_keys／model_routing_rules／project_index_*），
//! 主应用迁移创建，本插件基线迁移只登记归属。实现层 `yuancode_service` / `git_service` /
//! `inline_service` / `LspManager` / `editor_service` / `file_edit_service` /
//! `tools_commands` / `distill_dataset_service` 留主应用，dispatcher 复用。

pub const OWNED_TABLES: &[&str] = &[
    "yuan_code_snippets",
    "yuan_code_workspaces",
    "editor_documents",
    "editor_versions",
    "editor_sessions",
    "distill_dataset",
    "api_keys",
    "model_routing_rules",
    "project_index_projects",
    "project_index_files",
    "project_index_symbols",
    "project_index_imports",
    "project_index_dependencies",
    "project_index_changes",
];
