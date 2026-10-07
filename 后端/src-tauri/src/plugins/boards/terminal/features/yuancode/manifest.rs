//! terminal.yuancode 插件清单（阶段3 批次3c + 5e；L2 Feature，parent boards.terminal）。
//!
//! 覆盖面：Yuan Code 编辑器核心 54 条（yuancode_commands 27 + yuan_inline_commands 3 +
//! git_commands 13 + lsp_commands 5 + browser_commands 6——文件树/编辑器/AI 补全/
//! 代码片段/工作区/Git 面板/LSP/内嵌浏览器）。Agent / MCP / 目标 / 沙箱等其余 yuan_*
//! 命令域归 3d/3e，不在本插件。
//! 数据表：yuan_code_snippets（v48）/ yuan_code_workspaces（v49），短码 `yc`，
//! 表名不以 `yc_` 开头 → name_prefixed=0（既有表不改名，同 terminal_* 先例）。
//! 5e 新增 8 张表（api_keys / model_routing_rules / project_index_*）归属登记于本插件
//! （主应用迁移创建，本插件只登记归属）。
//! 无领域事件（emit 均为 Tauri 事件 ai-stream / yuan-code-*-done，非内核 Domain 事件）。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel, SlotSpec};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "terminal.yuancode".into(),
        name: "Yuan Code 编辑器核心".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.terminal".into()),
        slot: Some("terminal.yuancode".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 14 张表（3c 2 张 + 5a 4 张 + 5e 8 张）归属登记于本插件（主应用迁移创建，本插件只登记归属）。
            db: vec![
                "yuan_code_snippets".into(),
                "yuan_code_workspaces".into(),
                "editor_documents".into(),
                "editor_versions".into(),
                "editor_sessions".into(),
                "distill_dataset".into(),
                "api_keys".into(),
                "model_routing_rules".into(),
                "project_index_projects".into(),
                "project_index_files".into(),
                "project_index_symbols".into(),
                "project_index_imports".into(),
                "project_index_dependencies".into(),
                "project_index_changes".into(),
            ],
            // 不发布/订阅内核领域事件（emit 均为 Tauri 事件，S1 实测）。
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 302 条 IPC（3c 54 + 5a 56 + 5b 46 + 5c/5d/5e 146）；ACL/handler 键由内核按短码 `yc` 构造。
            ipc: vec!["yc_*".into()],
            // 文件树/读写/搜索/替换/Git/LSP/设置均为工作区文件系统访问（fs 面在段级
            // S1 实测为进程级系统访问，不走 manifest fs 白名单；登记说明）。
            fs: vec![],
            net: vec![],
        },
        // L3 三级嵌套插槽（6 个纯前端子插件，均挂载于 terminal.yuancode 之下）
        slots: vec![
            SlotSpec {
                id: "terminal.yuancode.editor".into(),
                r#type: "panel".into(),
                description: "编辑器核心".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "terminal.yuancode.agent".into(),
                r#type: "panel".into(),
                description: "Agent 与目标".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "terminal.yuancode.git".into(),
                r#type: "panel".into(),
                description: "版本控制".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "terminal.yuancode.skills".into(),
                r#type: "panel".into(),
                description: "技能与片段".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "terminal.yuancode.sandbox".into(),
                r#type: "panel".into(),
                description: "沙箱运行".into(),
                capacity: 1,
                route_prefix: None,
            },
            SlotSpec {
                id: "terminal.yuancode.settings".into(),
                r#type: "panel".into(),
                description: "编辑器设置".into(),
                capacity: 1,
                route_prefix: None,
            },
        ],
        i18n_namespace: "terminal.yuancode".into(),
    })
}
