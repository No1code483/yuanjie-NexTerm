//! ai.agent 插件清单（阶段3 批次2b-1；L2 Feature，parent boards.ai）。
//!
//! 覆盖面：ai_agents 表 CRUD 4 条命令（`get_ai_agents` / `add_ai_agent` /
//! `update_ai_agent` / `delete_ai_agent`）。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "ai.agent".into(),
        name: "智能体集".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.ai".into()),
        slot: Some("ai.agent".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // ai_agents 为既有旧表（短码 `ag` ≠ 旧名前缀 `ai_` → name_prefixed=0），
            // 归属登记见 migrations/0001_baseline.sql。
            db: vec!["ai_agents".into()],
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
            ipc: vec!["ag_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "ai.agent".into(),
    })
}